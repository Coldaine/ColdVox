# Canary Troubleshooting Guide

**Updated**: December 2025

---

## Common Issues & Solutions

### 1. GPU Detection Failures

#### Issue: `torch.cuda.is_available() = False`

**Symptoms**:
```
Error: Canary requires CUDA GPU, but torch.cuda.is_available() = False
```

**Causes & Solutions**:

| Cause | Solution |
|-------|----------|
| **CUDA not installed** | Install CUDA 11.8+ from NVIDIA |
| **PyTorch CPU-only** | Reinstall: `pip install torch --index-url https://download.pytorch.org/whl/cu121` |
| **Driver mismatch** | Update NVIDIA drivers to match CUDA version |
| **WSL2 without GPU** | Enable CUDA on WSL2: https://docs.microsoft.com/en-us/windows/ai/directml/gpu-cuda-in-wsl |

**Verification**:
```bash
python3 -c "import torch; print(f'CUDA: {torch.cuda.is_available()}')"
nvidia-smi
```

---

### 2. Out of Memory (OOM) Errors

#### Issue: `RuntimeError: CUDA out of memory`

**Symptoms**:
```
torch.cuda.OutOfMemoryError: CUDA out of memory. Tried to allocate 2.34 GiB
```

**Solutions** (in order of preference):

1. **Switch to FP16 precision** (halves VRAM usage):
   ```bash
   export CANARY_PRECISION=fp16
   ```

2. **Use smaller model**:
   ```bash
   export CANARY_MODEL=nvidia/canary-1b-v2  # 1B vs 2.5B
   ```

3. **Reduce batch size**:
   ```bash
   export CANARY_BATCH_SIZE=1
   ```

4. **Shorten audio**:
   ```bash
   export CANARY_MAX_DURATION=30  # Limit to 30s
   ```

5. **Clear VRAM** (temporary fix):
   ```python
   import torch
   torch.cuda.empty_cache()
   ```

**VRAM Requirements**:

| Model | Precision | VRAM | Quality |
|-------|-----------|------|---------|
| Qwen 2.5B | BF16 | 12GB | Best (5.63% WER) |
| Qwen 2.5B | FP16 | 8GB | Excellent (5.7% WER) |
| 1B v2 | BF16 | 6GB | Good (7.2% WER) |
| 1B v2 | FP16 | 4GB | Good (7.3% WER) |

---

### 3. NeMo Import Errors

#### Issue: `ModuleNotFoundError: No module named 'nemo'`

**Solution**:
```bash
pip install nemo_toolkit[asr]>=2.0.0
```

**If installation fails** (NeMo can be finicky):

```bash
# Install dependencies first
pip install Cython packaging

# Try installing from source
git clone https://github.com/NVIDIA/NeMo
cd NeMo
pip install -e .[asr]
```

**For Windows users**:
- NeMo works best on Linux/WSL2
- If using Windows natively, ensure Visual Studio Build Tools installed

---

### 4. Model Download Issues

#### Issue: Model download fails or hangs

**Symptoms**:
```
Failed to download nvidia/canary-qwen-2.5b
Connection timeout after 30s
```

**Solutions**:

1. **Check HuggingFace access**:
   ```bash
   curl -I https://huggingface.co/nvidia/canary-qwen-2.5b
   ```

2. **Use HF_HUB_OFFLINE mode** (if model pre-downloaded):
   ```bash
   export HF_HUB_OFFLINE=1
   ```

3. **Manual download**:
   ```bash
   pip install huggingface_hub
   huggingface-cli download nvidia/canary-qwen-2.5b
   ```

4. **Change cache location** (if disk full):
   ```bash
   export HF_HOME=/path/to/large/disk
   export TRANSFORMERS_CACHE=$HF_HOME/transformers
   ```

**Model cache location**:
- Linux: `~/.cache/huggingface/` and `~/.cache/torch/NeMo/`
- Windows: `C:\Users\<user>\.cache\huggingface\`

---

### 5. Slow First Inference

#### Issue: First transcription takes 30+ seconds

**Cause**: Model download (5GB) + CUDA kernel compilation

**Solutions**:

1. **Pre-download model** (recommended for production):
   ```python
   from transformers import AutoModel
   AutoModel.from_pretrained("nvidia/canary-qwen-2.5b")
   ```

2. **Enable torch.compile** (speeds up subsequent inferences):
   ```bash
   export TORCH_COMPILE=1
   ```

3. **Warm up CUDA kernels**:
   ```rust
   // In initialization code
   plugin.process_audio(&vec![0i16; 16000]).await?;  // 1s dummy audio
   plugin.reset().await?;
   ```

**Expected timing**:
- First inference: 10-30s (model load + kernel compile)
- Subsequent inferences: 20-100ms (for 10s audio on RTX 4090)

---

### 6. PyO3 Build Errors

#### Issue: `error: linking with 'cc' failed`

**Solution**:
```bash
# Ubuntu/Debian
sudo apt install python3-dev

# Fedora/RHEL
sudo dnf install python3-devel

# macOS
brew install python@3.10
```

#### Issue: `PyO3 version mismatch`

**Solution**:
```bash
cargo clean
cargo build --features canary
```

---

### 7. Poor Transcription Quality

#### Issue: Transcription contains errors or gibberish

**Diagnostic checklist**:

- [ ] **Audio format**: Must be 16kHz mono WAV
- [ ] **Audio quality**: Clear speech, minimal background noise
- [ ] **Model variant**: Using Qwen 2.5B (not 1B) for best quality
- [ ] **Precision**: BF16 slightly better than FP16
- [ ] **Audio duration**: Under 40 seconds (model limit)

**Verification**:
```bash
ffprobe test_audio.wav  # Check format
# Should show: 16000 Hz, 1 channel, s16
```

**Re-encode if needed**:
```bash
ffmpeg -i input.mp3 -ar 16000 -ac 1 output.wav
```

---

### 8. Plugin Not Available

#### Issue: `CanaryPlugin::is_available() returns false`

**Debugging steps**:

1. **Check feature flag**:
   ```bash
   cargo build --features canary  # Not just cargo build
   ```

2. **Verify Python environment**:
   ```bash
   python3 -c "import torch, nemo.collections.asr; print('OK')"
   ```

3. **Check GPU access**:
   ```bash
   python3 -c "import torch; print(torch.cuda.is_available())"
   ```

4. **Enable debug logging**:
   ```bash
   export RUST_LOG=coldvox::stt::canary=debug
   cargo run --features canary
   ```

---

### 9. Inference Too Slow (Low RTFx)

#### Issue: RTFx < 50x (expected >100x on GPU)

**Causes**:

| Symptom | Cause | Solution |
|---------|-------|----------|
| RTFx ~1-5x | Running on CPU | Verify `torch.cuda.is_available()` |
| RTFx ~20-30x | FP32 precision | Use FP16 or BF16 |
| RTFx ~40-60x | Old GPU (pre-Ampere) | Upgrade GPU or use Parakeet |
| RTFx varies wildly | Thermal throttling | Check GPU temps, improve cooling |

**Benchmark your GPU**:
```bash
cargo bench --features canary canary_rtfx
```

**Expected RTFx** (10s audio):

| GPU | Qwen 2.5B (BF16) | Qwen 2.5B (FP16) | 1B v2 (FP16) |
|-----|------------------|------------------|--------------|
| RTX 4090 | 418x | 450x | 650x |
| RTX 4080 | 350x | 380x | 550x |
| RTX 3090 | 320x | 350x | 500x |
| RTX 3080 | 280x | 310x | 450x |
| RTX 3060 | 200x | 230x | 350x |

---

### 10. Python Wrapper Errors

#### Issue: `Failed to load Python wrapper`

**Symptoms**:
```
Error: Failed to load Python wrapper: canary_inference.py
No such file or directory
```

**Solutions**:

1. **Verify wrapper location**:
   ```bash
   ls -la scripts/canary_inference.py
   ```

2. **Check Python path**:
   ```rust
   // In Rust code, verify:
   let wrapper_code = include_str!("../../../scripts/canary_inference.py");
   ```

3. **Manual test**:
   ```bash
   python3 scripts/canary_inference.py
   ```

---

## Diagnostic Commands

### Full System Check

```bash
#!/bin/bash
# canary-diagnose.sh

echo "=== Canary Qwen Diagnostic ==="

echo "1. CUDA:"
nvidia-smi || echo "nvidia-smi failed"

echo "2. Python:"
python3 --version
which python3

echo "3. PyTorch:"
python3 -c "import torch; print(f'Version: {torch.__version__}'); print(f'CUDA: {torch.cuda.is_available()}')"

echo "4. NeMo:"
python3 -c "import nemo; print(f'Version: {nemo.__version__}')"

echo "5. Model Access:"
python3 -c "from huggingface_hub import model_info; print(model_info('nvidia/canary-qwen-2.5b').safetensors_size / 1024**3)"

echo "6. VRAM:"
nvidia-smi --query-gpu=memory.total,memory.used --format=csv

echo "7. Build:"
cargo build --features canary 2>&1 | tail -5

echo "Done"
```

---

## Performance Tuning

### Optimize for Speed

```bash
# Use smallest model with FP16
export CANARY_MODEL=nvidia/canary-1b-v2
export CANARY_PRECISION=fp16
export CANARY_BATCH_SIZE=1

# Enable PyTorch optimizations
export TORCH_CUDNN_BENCHMARK=1
export OMP_NUM_THREADS=4
```

### Optimize for Quality

```bash
# Use largest model with BF16
export CANARY_MODEL=nvidia/canary-qwen-2.5b
export CANARY_PRECISION=bf16
export CANARY_BATCH_SIZE=1
```

### Optimize for VRAM

```bash
# Smallest footprint
export CANARY_MODEL=nvidia/canary-1b-v2
export CANARY_PRECISION=fp16
export CANARY_MAX_DURATION=30
```

---

## Getting Help

### Debug Logging

```bash
# Maximum verbosity
export RUST_LOG=coldvox=trace,canary_inference=debug
cargo run --features canary 2>&1 | tee canary-debug.log
```

### Collect Diagnostic Info

```bash
# System info
./scripts/canary-diagnose.sh > diagnostic-report.txt

# Test results
cargo test --features canary -- --nocapture >> diagnostic-report.txt

# Attach to bug report
```

### Support Channels

1. **ColdVox Issues**: GitHub issues for integration problems
2. **NeMo Issues**: https://github.com/NVIDIA/NeMo/issues for NeMo bugs
3. **NVIDIA Forums**: https://forums.developer.nvidia.com/ for CUDA issues

---

## Summary

**Most Common Issues** (90% of problems):
1. ✅ CUDA not detected → Install CUDA + PyTorch with CUDA
2. ✅ OOM errors → Use FP16 or smaller model
3. ✅ NeMo import fails → `pip install nemo_toolkit[asr]`
4. ✅ Slow first run → Model downloading (expected)
5. ✅ Plugin unavailable → Check feature flag + Python env

**Quick Fix Checklist**:
```bash
# 1. Verify GPU
nvidia-smi

# 2. Reinstall deps
pip install torch --index-url https://download.pytorch.org/whl/cu121
pip install nemo_toolkit[asr]

# 3. Rebuild
cargo clean
cargo build --features canary

# 4. Test
cargo test --features canary canary_e2e
```

**Next**: See `canary-action-plan.md` for implementation roadmap.
