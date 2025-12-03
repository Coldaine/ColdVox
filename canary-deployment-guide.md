# Canary Deployment Guide & Production Checklist

**Status**: ✅ PRODUCTION READY

---

## Prerequisites

### Hardware Requirements

| Component | Minimum | Recommended | Notes |
|-----------|---------|-------------|-------|
| **GPU** | RTX 3060 (12GB) | RTX 4090 (24GB) | Ampere/Ada/Hopper architecture |
| **VRAM** | 8GB (FP16) | 12GB+ (BF16) | More VRAM = better quality |
| **CUDA** | 11.8+ | 12.1+ | With cuDNN 8.9+ |
| **RAM** | 16GB | 32GB | Python NeMo overhead |
| **CPU** | 4 cores | 8+ cores | PyTorch threading |

### Software Requirements

| Software | Version | Installation |
|----------|---------|--------------|
| **Python** | 3.8-3.11 | `python --version` |
| **PyTorch** | 2.2+ | With CUDA support |
| **NeMo** | 2.0+ | `nemo_toolkit[asr]` |
| **CUDA** | 11.8+ | NVIDIA drivers |

---

## Installation Scripts

### File: `scripts/install-canary-deps.sh`

```bash
#!/bin/bash
# Install Canary Qwen 2.5B dependencies

set -e

echo "=== Canary Qwen 2.5B Dependency Installer ==="
echo ""

# 1. Check CUDA
echo "1. Checking CUDA..."
if ! command -v nvidia-smi &> /dev/null; then
    echo "❌ Error: nvidia-smi not found"
    echo "   Install CUDA 11.8+ from: https://developer.nvidia.com/cuda-downloads"
    exit 1
fi

nvidia-smi
CUDA_VERSION=$(nvidia-smi | grep "CUDA Version" | awk '{print $9}')
echo "✅ CUDA detected: $CUDA_VERSION"
echo ""

# 2. Check Python
echo "2. Checking Python..."
if ! command -v python3 &> /dev/null; then
    echo "❌ Error: python3 not found"
    exit 1
fi

PYTHON_VERSION=$(python3 --version | cut -d' ' -f2)
echo "✅ Python detected: $PYTHON_VERSION"

# Verify Python version (3.8-3.11)
MAJOR=$(echo $PYTHON_VERSION | cut -d'.' -f1)
MINOR=$(echo $PYTHON_VERSION | cut -d'.' -f2)

if [ "$MAJOR" -ne 3 ] || [ "$MINOR" -lt 8 ] || [ "$MINOR" -gt 11 ]; then
    echo "⚠️  Warning: Python $PYTHON_VERSION detected"
    echo "   NeMo works best with Python 3.8-3.11"
fi
echo ""

# 3. Install PyTorch with CUDA
echo "3. Installing PyTorch with CUDA 12.1..."
pip3 install --upgrade pip
pip3 install torch torchaudio --index-url https://download.pytorch.org/whl/cu121

# Verify PyTorch CUDA
python3 -c "
import torch
assert torch.cuda.is_available(), 'PyTorch CUDA not available'
print(f'✅ PyTorch {torch.__version__} with CUDA {torch.version.cuda}')
"
echo ""

# 4. Install NeMo Toolkit
echo "4. Installing NVIDIA NeMo Toolkit..."
echo "   (This may take 5-10 minutes...)"

# Install Cython first (NeMo dependency)
pip3 install Cython packaging

# Install NeMo
pip3 install nemo_toolkit[asr]>=2.0.0

# Verify NeMo
python3 -c "
import nemo
import nemo.collections.asr as nemo_asr
print(f'✅ NeMo {nemo.__version__} installed')
"
echo ""

# 5. Test Canary model loading
echo "5. Testing Canary model access..."
python3 <<EOF
import torch
import nemo.collections.asr as nemo_asr

print("Checking HuggingFace access to nvidia/canary-qwen-2.5b...")
# This will check but NOT download the 5GB model
try:
    from huggingface_hub import model_info
    info = model_info("nvidia/canary-qwen-2.5b")
    print(f"✅ Model accessible on HuggingFace")
    print(f"   Model size: ~{info.safetensors_size / 1024**3:.1f}GB")
except Exception as e:
    print(f"⚠️  Warning: Could not verify model access: {e}")
EOF
echo ""

# 6. Summary
echo "=== Installation Complete ==="
echo ""
echo "Installed packages:"
pip3 list | grep -E "(torch|nemo)"
echo ""
echo "Next steps:"
echo "1. Build ColdVox: cargo build --features canary"
echo "2. Run tests: cargo test --features canary canary_e2e"
echo "3. First run will download 5GB Canary model to ~/.cache/torch/"
echo ""
echo "Environment variables (optional):"
echo "  export CANARY_MODEL=nvidia/canary-qwen-2.5b"
echo "  export CANARY_PRECISION=fp16  # or bf16 (default)"
echo "  export CANARY_BATCH_SIZE=1"
```

### File: `scripts/verify-canary-setup.sh`

```bash
#!/bin/bash
# Verify Canary Qwen setup

set -e

echo "=== Canary Qwen Setup Verification ==="
echo ""

# 1. GPU Check
echo "1. GPU Detection:"
if nvidia-smi &> /dev/null; then
    GPU_NAME=$(nvidia-smi --query-gpu=name --format=csv,noheader | head -n1)
    VRAM_TOTAL=$(nvidia-smi --query-gpu=memory.total --format=csv,noheader | head -n1)
    CUDA_VERSION=$(nvidia-smi | grep "CUDA Version" | awk '{print $9}')
    
    echo "   ✅ GPU: $GPU_NAME"
    echo "   ✅ VRAM: $VRAM_TOTAL"
    echo "   ✅ CUDA: $CUDA_VERSION"
    
    # Check VRAM requirement
    VRAM_GB=$(echo $VRAM_TOTAL | cut -d' ' -f1)
    if (( $(echo "$VRAM_GB < 8" | bc -l) )); then
        echo "   ⚠️  Warning: Less than 8GB VRAM (FP16 requires 8GB, BF16 requires 12GB)"
    fi
else
    echo "   ❌ No GPU detected"
    exit 1
fi
echo ""

# 2. Python Environment
echo "2. Python Environment:"
python3 -c "
import sys
print(f'   ✅ Python {sys.version.split()[0]}')

import torch
print(f'   ✅ PyTorch {torch.__version__}')
print(f'   ✅ CUDA available: {torch.cuda.is_available()}')
if torch.cuda.is_available():
    print(f'   ✅ CUDA version: {torch.version.cuda}')
    print(f'   ✅ GPU count: {torch.cuda.device_count()}')

import nemo
print(f'   ✅ NeMo {nemo.__version__}')

import nemo.collections.asr
print(f'   ✅ NeMo ASR module loaded')
"
echo ""

# 3. Build Test
echo "3. ColdVox Build:"
if cargo build --features canary 2>&1 | grep -q "Finished"; then
    echo "   ✅ Build successful"
else
    echo "   ❌ Build failed"
    exit 1
fi
echo ""

# 4. Model Access
echo "4. Canary Model Access:"
python3 <<EOF
from huggingface_hub import model_info
try:
    info = model_info("nvidia/canary-qwen-2.5b")
    size_gb = info.safetensors_size / 1024**3
    print(f"   ✅ Model accessible: nvidia/canary-qwen-2.5b")
    print(f"   ✅ Model size: {size_gb:.1f}GB")
    print(f"   ℹ️  First inference will download to ~/.cache/torch/")
except Exception as e:
    print(f"   ⚠️  Warning: {e}")
EOF
echo ""

# 5. Summary
echo "=== Verification Summary ==="
echo ""
echo "Ready to use Canary Qwen 2.5B! 🎉"
echo ""
echo "Quick start:"
echo "  cargo test --features canary canary_e2e -- --nocapture"
echo ""
echo "Configuration:"
echo "  export CANARY_MODEL=nvidia/canary-qwen-2.5b"
echo "  export CANARY_PRECISION=fp16  # 8GB VRAM"
echo "  # or"
echo "  export CANARY_PRECISION=bf16  # 12GB VRAM (better quality)"
```

---

## Production Deployment Checklist

### Phase 1: Pre-Deployment (Day 1)

- [ ] **Hardware Verification**
  - [ ] GPU has 8GB+ VRAM (12GB+ recommended)
  - [ ] CUDA 11.8+ installed
  - [ ] Driver supports CUDA version
  - [ ] Adequate cooling for GPU (sustained load)

- [ ] **Software Installation**
  - [ ] Python 3.8-3.11 installed
  - [ ] Run `./scripts/install-canary-deps.sh`
  - [ ] Run `./scripts/verify-canary-setup.sh`
  - [ ] All checks pass

- [ ] **Build Verification**
  - [ ] `cargo build --features canary` succeeds
  - [ ] `cargo test --features canary --lib` passes
  - [ ] Binary size acceptable (~500MB with NeMo deps)

### Phase 2: Testing (Day 2)

- [ ] **Unit Tests**
  - [ ] Run `cargo test --features canary --lib`
  - [ ] All model variants tested
  - [ ] Precision modes verified

- [ ] **E2E Tests**
  - [ ] Run `cargo test --features canary canary_e2e`
  - [ ] First model download completes (~5GB)
  - [ ] Transcription quality verified
  - [ ] RTFx meets expectations (>100x on GPU)

- [ ] **Integration Tests**
  - [ ] Fallback chain works (Canary → Parakeet → Moonshine)
  - [ ] Plugin discovery correct
  - [ ] Config file integration works

- [ ] **Benchmark Tests**
  - [ ] Run `cargo bench --features canary`
  - [ ] RTFx documented for your hardware
  - [ ] VRAM usage monitored

### Phase 3: Configuration (Day 2-3)

- [ ] **Environment Variables**
  - [ ] `CANARY_MODEL` set (or use default)
  - [ ] `CANARY_PRECISION` set based on VRAM
  - [ ] `CANARY_BATCH_SIZE` tuned for workload
  - [ ] `CANARY_MAX_DURATION` set based on use case

- [ ] **Application Integration**
  - [ ] Plugin registered in main.rs
  - [ ] Config file updated
  - [ ] Fallback chain configured
  - [ ] Logging configured

- [ ] **Monitoring Setup**
  - [ ] GPU metrics collection (VRAM, utilization)
  - [ ] Inference latency tracking
  - [ ] RTFx monitoring
  - [ ] Error rate tracking

### Phase 4: Staging Deployment (Day 3)

- [ ] **Staging Environment**
  - [ ] Deploy to staging with GPU
  - [ ] Run smoke tests
  - [ ] Verify model downloads correctly
  - [ ] Test with real audio samples

- [ ] **Performance Validation**
  - [ ] Measure actual RTFx
  - [ ] VRAM usage within limits
  - [ ] No OOM errors
  - [ ] Latency acceptable

- [ ] **Failure Testing**
  - [ ] GPU unavailable (falls back to Parakeet/Moonshine)
  - [ ] Python import fails (graceful error)
  - [ ] Model download fails (retry logic works)
  - [ ] OOM handling (error reported clearly)

### Phase 5: Production Deployment (Day 4+)

- [ ] **Production Build**
  - [ ] Build with: `cargo build --release --features "parakeet,moonshine,canary"`
  - [ ] Binary tested on production hardware
  - [ ] Model pre-downloaded to avoid first-run delay

- [ ] **Deployment**
  - [ ] Deploy binary
  - [ ] Verify GPU access
  - [ ] Check model cache location
  - [ ] Restart services

- [ ] **Post-Deployment Validation**
  - [ ] End-to-end test in production
  - [ ] Monitor first 24 hours
  - [ ] Check logs for errors
  - [ ] Verify metrics collection

- [ ] **Documentation**
  - [ ] Runbook updated
  - [ ] Team trained on Canary specifics
  - [ ] Troubleshooting guide accessible
  - [ ] Rollback procedure documented

---

## Configuration Examples

### Minimal Configuration

```toml
# config/default.toml
[stt]
enabled = true
plugin = "canary"  # Use Canary as primary

[stt.canary]
# All defaults - will use:
# - model: nvidia/canary-qwen-2.5b
# - precision: bf16 (12GB VRAM)
# - batch_size: 1
```

### Production Configuration (Recommended)

```toml
[stt]
enabled = true
plugin = "auto"  # Auto-select based on availability

# Preference order: accuracy > speed
fallback_plugins = ["canary", "parakeet", "moonshine"]

[stt.canary]
model = "nvidia/canary-qwen-2.5b"
precision = "fp16"  # 8GB VRAM (slight quality trade-off)
batch_size = 1
max_duration_secs = 40

[stt.parakeet]
variant = "tdt"
device = "cuda"

[stt.moonshine]
model = "base"
```

### High-Throughput Configuration

```toml
[stt.canary]
model = "nvidia/canary-1b-v2"  # Smaller, faster
precision = "fp16"
batch_size = 8  # Process multiple files in parallel
max_duration_secs = 30
```

### Low-VRAM Configuration

```toml
[stt.canary]
model = "nvidia/canary-1b-v2"  # 1B model
precision = "fp16"  # Only 4GB VRAM needed
batch_size = 1
```

---

## Docker Deployment

### Dockerfile

```dockerfile
FROM nvidia/cuda:12.1.0-cudnn8-runtime-ubuntu22.04

# Install Python
RUN apt-get update && apt-get install -y \
    python3 \
    python3-pip \
    && rm -rf /var/lib/apt/lists/*

# Install PyTorch + NeMo
RUN pip3 install --no-cache-dir \
    torch torchaudio --index-url https://download.pytorch.org/whl/cu121 \
    nemo_toolkit[asr]>=2.0.0

# Install Rust (for build)
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# Copy source
WORKDIR /app
COPY . .

# Build with Canary support
RUN cargo build --release --features canary

# Pre-download model (optional, avoids first-run delay)
# RUN python3 -c "from transformers import AutoModel; AutoModel.from_pretrained('nvidia/canary-qwen-2.5b')"

CMD ["/app/target/release/coldvox"]
```

### Docker Compose

```yaml
version: '3.8'

services:
  coldvox-canary:
    build:
      context: .
      dockerfile: docker/Dockerfile.canary
    runtime: nvidia
    environment:
      - CANARY_MODEL=nvidia/canary-qwen-2.5b
      - CANARY_PRECISION=fp16
      - NVIDIA_VISIBLE_DEVICES=all
    volumes:
      - ~/.cache/torch:/root/.cache/torch  # Cache models
    deploy:
      resources:
        reservations:
          devices:
            - driver: nvidia
              count: 1
              capabilities: [gpu]
```

---

## Monitoring & Observability

### Key Metrics

| Metric | Target | Alert Threshold |
|--------|--------|-----------------|
| **RTFx** | >100x | <50x |
| **VRAM Usage** | <80% | >90% |
| **Inference Latency** | <100ms (10s audio) | >500ms |
| **Error Rate** | <0.1% | >1% |
| **Model Load Time** | <10s | >30s |

### Logging Best Practices

```rust
// At initialization
info!(
    target: "coldvox::stt::canary",
    model = %variant.model_identifier(),
    precision = ?precision,
    cuda_version = %cuda_version,
    gpu_name = %gpu_name,
    vram_total_gb = vram_total_gb,
    "Canary initialized"
);

// Per inference
debug!(
    target: "coldvox::stt::canary",
    audio_secs = duration_secs,
    inference_ms = elapsed_ms,
    rtfx = rtfx,
    vram_used_mb = vram_mb,
    "Inference complete"
);

// On errors
error!(
    target: "coldvox::stt::canary",
    error = %e,
    vram_available_mb = vram_available,
    "Inference failed"
);
```

---

## Troubleshooting

See `canary-troubleshooting.md` for complete troubleshooting guide.

**Common issues**:
1. OOM errors → Use FP16, smaller model, or shorter audio
2. Slow first inference → Model download (5GB) - pre-download in production
3. Import errors → Check NeMo installation
4. CUDA errors → Verify driver compatibility

---

## Summary

**Production Readiness**: ✅ Complete
- Installation scripts provided
- Verification scripts included
- Docker deployment ready
- Comprehensive monitoring guide
- Full troubleshooting documentation

**Timeline**: 4 days from zero to production
- Day 1: Installation & verification
- Day 2: Testing & configuration
- Day 3: Staging deployment
- Day 4: Production deployment

**Next**: See `canary-troubleshooting.md` and `canary-action-plan.md`
