---
doc_type: reference
subsystem: stt
status: active
freshness: current
preservation: preserve
summary: Docker container options for STT inference on RTX 5090, benchmarks, setup, and operational notes
signals: ['stt', 'docker', 'parakeet', 'whisper', 'gpu', 'rtx-5090']
created: 2026-03-25
last_verified: 2026-03-25
---

# STT Docker Container Reference

Evaluated Docker containers for speech-to-text inference on an RTX 5090 (32 GB VRAM, Blackwell SM120, CUDA 12.8, Windows 11 + WSL2).

## Tested Containers (March 2026)

### 1. fedirz/faster-whisper-server (Recommended — Best balance)

- **Image**: `fedirz/faster-whisper-server:latest-cuda`
- **Size**: 7.17 GB
- **Port**: 8000
- **API**: OpenAI-compatible `/v1/audio/transcriptions`
- **GPU**: Yes (CUDA, auto-downloads models on first request)
- **Auth**: None
- **Tested on RTX 5090**: Yes — GPU acceleration confirmed, 8.6 GB VRAM with small model

```bash
docker pull fedirz/faster-whisper-server:latest-cuda

docker run -d --name faster-whisper \
  --gpus all \
  -p 8100:8000 \
  -v faster-whisper-models:/root/.cache/huggingface \
  fedirz/faster-whisper-server:latest-cuda
```

**Test results (RTX 5090, Whisper small, ColdVox WAVs):**

| File | Duration | Transcription | Latency |
|---|---|---|---|
| test_1.wav | 4s | "On August 27th, 1837, she writes." | 0.585s |
| test_2.wav | ~5s | "Far from it, Sire. Your Majesty haven't given no directions about it." | 0.731s |
| test_3.wav | ~3s | "He is the one with the worst record." | 0.392s |
| test_5.wav | ~4s | "Your play must be not merely a good play, but a successful one." | 0.489s |

First request downloads the model (~56s for small). Subsequent requests are fast.
Supports model swapping: `Systran/faster-whisper-tiny`, `small`, `medium`, `large-v3`.

**Source**: [hub.docker.com/r/fedirz/faster-whisper-server](https://hub.docker.com/r/fedirz/faster-whisper-server)

---

### 2. ghcr.io/speaches-ai/speaches (Most flexible — STT + TTS)

- **Image**: `ghcr.io/speaches-ai/speaches:latest-cuda`
- **Size**: 8.58 GB
- **Port**: 8000
- **API**: OpenAI-compatible `/v1/audio/transcriptions` + `/v1/audio/translations` + TTS
- **GPU**: Yes (CUDA 12.6.3 base)
- **Auth**: None
- **Tested on RTX 5090**: Container runs, models must be explicitly installed via `POST /v1/models`

```bash
docker pull ghcr.io/speaches-ai/speaches:latest-cuda

docker run -d --name speaches \
  --gpus all \
  -p 8000:8000 \
  -v speaches-models:/home/ubuntu/.cache/huggingface/hub \
  ghcr.io/speaches-ai/speaches:latest-cuda

# Install a model (required before first transcription)
curl -X POST http://localhost:8000/v1/models \
  -H "Content-Type: application/json" \
  -d '{"model": "Systran/faster-whisper-small"}'
```

**Notes**: Unlike faster-whisper-server, models must be explicitly downloaded before use.
Also provides TTS endpoints — useful if you need speech synthesis too.

**Source**: [speaches.ai](https://speaches.ai/installation/) | [github.com/speaches-ai/speaches](https://github.com/speaches-ai/speaches)

---

### 3. ghcr.io/achetronic/parakeet (CPU-only — Zero VRAM)

- **Image**: `ghcr.io/achetronic/parakeet:latest`
- **Size**: 1.31 GB
- **Port**: 5092
- **API**: OpenAI-compatible `/v1/audio/transcriptions`
- **GPU**: No (CPU only, ONNX Runtime)
- **Auth**: None
- **Tested on RTX 5090**: N/A (CPU only)

```bash
docker pull ghcr.io/achetronic/parakeet:latest

docker run -d --name parakeet-cpu \
  -p 5092:5092 \
  ghcr.io/achetronic/parakeet:latest
```

**Test results (CPU, Parakeet-TDT 0.6B v3 INT8):**

| File | Transcription | Latency |
|---|---|---|
| test_1.wav | "On august twenty seventh, eighteen thirty seven, she writes." | 1.885s |

**Use case**: Always-on background STT that doesn't compete for GPU VRAM with other models (llama-swap, ComfyUI, etc.).

**Source**: [github.com/achetronic/parakeet](https://github.com/achetronic/parakeet)

---

### 4. NVIDIA NIM Parakeet (Best quality — Native RTX 5090 support)

Requires free NGC API key. Uses Triton inference server internally (not ONNX Runtime), so Blackwell SM120 is natively supported. RTX 50xx explicitly listed in support matrix.

- **Images**: `nvcr.io/nim/nvidia/<container-id>:latest`
- **Port**: 9000 (HTTP) + 50051 (gRPC)
- **API**: OpenAI-style `/v1/audio/transcriptions` + gRPC streaming
- **GPU**: Yes (native Blackwell/SM120)
- **Auth**: NGC API key (free at [org.ngc.nvidia.com/setup/api-keys](https://org.ngc.nvidia.com/setup/api-keys))

#### Available NIM containers

| Container ID | Model | Languages | Streaming | Notes |
|---|---|---|---|---|
| `parakeet-0-6b-ctc-en-us` | CTC 0.6B | English | No | Has `rtx-latest` tag |
| `parakeet-1-1b-ctc-en-us` | CTC 1.1B | English | No | Best EN accuracy |
| `parakeet-0.6b-tdt` | TDT 0.6B | English | No | #1 Open ASR Leaderboard |
| `parakeet-1-1b-rnnt-multilingual` | RNNT 1.1B | 25+ langs | Yes | Streaming multilingual |
| `whisper-large-v3` | Whisper v3 | 99 langs | No | Offline only |
| `nemotron-asr-streaming` | Nemotron 0.6B | English | Yes | Lowest latency (<24ms) |

```bash
# One-time setup
export NGC_API_KEY=<your-key>
echo "$NGC_API_KEY" | docker login nvcr.io --username '$oauthtoken' --password-stdin

# RTX-optimized Parakeet (recommended starter)
docker run -it --rm \
  --runtime=nvidia --gpus '"device=0"' --shm-size=8GB \
  -e NGC_API_KEY \
  -e NIM_HTTP_API_PORT=9000 \
  -p 9000:9000 -p 50051:50051 \
  -v nim-cache:/opt/nim/.cache \
  nvcr.io/nim/nvidia/parakeet-0-6b-ctc-en-us:rtx-latest

# Best accuracy
docker run -it --rm \
  --runtime=nvidia --gpus '"device=0"' --shm-size=8GB \
  -e NGC_API_KEY \
  -e NIM_HTTP_API_PORT=9000 \
  -e NIM_TAGS_SELECTOR="name=parakeet-1-1b-ctc-en-us,mode=all" \
  --ulimit nofile=2048:2048 \
  -p 9000:9000 -p 50051:50051 \
  -v nim-cache:/opt/nim/.cache \
  nvcr.io/nim/nvidia/parakeet-1-1b-ctc-en-us:latest

# Real-time streaming (lowest latency)
docker run -it --rm \
  --runtime=nvidia --gpus '"device=0"' --shm-size=8GB \
  -e NGC_API_KEY \
  -e NIM_HTTP_API_PORT=9000 \
  -e NIM_TAGS_SELECTOR=mode=str \
  -p 9000:9000 -p 50051:50051 \
  -v nim-cache:/opt/nim/.cache \
  nvcr.io/nim/nvidia/nemotron-asr-streaming:latest
```

**First startup downloads models from NGC (~10+ GB, up to 30 minutes). Use `-v nim-cache:/opt/nim/.cache` to persist across restarts.**

**Source**: [docs.nvidia.com/nim/speech/latest](https://docs.nvidia.com/nim/speech/latest/reference/support-matrix/asr.html)

---

### 5. marcpope/parakeet-api (Community GPU Parakeet — Fastest tested)

- **Image**: `marcpope/parakeet-api:latest`
- **Size**: 13.7 GB
- **Port**: 8000
- **API**: `/audio/transcriptions` (note: no `/v1/` prefix), Swagger at `/docs`
- **GPU**: Yes (CUDA, NeMo + PyTorch, Parakeet-TDT 0.6B v2 baked in)
- **Auth**: None
- **Tested on RTX 5090**: Yes — GPU confirmed, ~4 GB VRAM

```bash
docker pull marcpope/parakeet-api:latest

docker run -d --name parakeet-gpu \
  -p 8200:8000 \
  --gpus all \
  marcpope/parakeet-api:latest
```

**Test results (RTX 5090, Parakeet-TDT 0.6B v2, GPU):**

| File | Duration | Transcription | Latency |
|---|---|---|---|
| test_1.wav | 4s | "On august twenty seventh, eighteen thirty seven, she writes Yeah." | 8.4s (cold) / **0.1s** |
| test_2.wav | ~5s | "Far from it, sire your majesty having given no directions about it..." | **0.130s** |
| test_3.wav | ~3s | "He is the one with the worst record." | **0.086s** |
| 10s recording | 10s | "Testing, testing, testing, record. Everybody loves the something..." | **0.126s** |

**Fastest container tested** — 86ms for a 3s clip after warm-up. 4-5x faster than faster-whisper-server. First request is slow (~8s, model warm-up).

**Caveat**: API endpoint is `/audio/transcriptions` not `/v1/audio/transcriptions`. Not directly OpenAI SDK-compatible without adjusting the base URL path.

**Source**: [hub.docker.com/r/marcpope/parakeet-api](https://hub.docker.com/r/marcpope/parakeet-api)

---

### 6. mekopa/whisperx-blackwell (Blackwell-native Whisper + Diarization)

- **Image**: `mekopa/whisperx-blackwell:latest`
- **Port**: 8003
- **API**: REST `/transcribe` (not OpenAI-compatible)
- **GPU**: Yes (CUDA 13.0, Blackwell purpose-built)
- **Auth**: HuggingFace token (for pyannote diarization models)
- **Features**: WhisperX large-v3 + word alignment + speaker diarization

```bash
docker pull mekopa/whisperx-blackwell:latest

docker run -d --name whisperx \
  --gpus all --ipc=host \
  -p 8003:8003 \
  -v /path/to/audio:/data \
  -e HF_TOKEN="your_token" \
  mekopa/whisperx-blackwell:latest
```

**Caveat**: Targets datacenter Blackwell (SM121). RTX 5090 is SM120 — untested but may work via Hopper compatibility spoof.

**Source**: [github.com/Mekopa/whisperx-blackwell](https://github.com/Mekopa/whisperx-blackwell)

---

## Head-to-Head: RTX 5090 Test Results (March 2026)

All tested with ColdVox WAV files. VRAM measured with all containers running simultaneously.

| Container | Model | test_1 (4s) | test_2 (5s) | test_3 (3s) | 10s recording | VRAM |
|---|---|---|---|---|---|---|
| **marcpope/parakeet-api** | Parakeet-TDT 0.6B v2 | 8.4s cold / **0.1s** | **0.130s** | **0.086s** | **0.126s** | ~4 GB |
| **faster-whisper-server** | Whisper small | 56s cold / 0.585s | 0.731s | 0.392s | — | ~2 GB |
| **Parakeet CPU (achetronic)** | Parakeet-TDT 0.6B v3 INT8 | 1.885s | — | — | — | 0 GB |
| **Native parakeet-stt** | Parakeet-TDT 0.6B v3 INT8 | ~1s | ~1s | ~0.5s | ~1.5s | 0 GB |

**Total VRAM with 3 GPU containers running**: 14.6 GB / 32 GB

**Winner**: marcpope/parakeet-api — 86ms for a 3s clip is 4-5x faster than faster-whisper. Uses Parakeet-TDT 0.6B v2 which is also #3 on the Open ASR Leaderboard (better accuracy than Whisper).

---

## Benchmarks (Open ASR Leaderboard, March 2026)

### Current Leaderboard (English track)

| Rank | Model | Avg WER | Speed (RTFx) | Params | Released | Available via |
|---|---|---|---|---|---|---|
| 1 | **IBM Granite 4.0 1B Speech** | **5.52%** | 280x | 1B | Mar 2026 | HuggingFace (Apache 2.0) |
| 2 | NVIDIA Canary-Qwen-2.5B | 5.63% | 418x | 2.5B | Jun 2025 | NIM |
| 3 | IBM Granite-Speech-3.3-8B | 5.85% | — | ~9B | 2025 | — |
| 4 | **Parakeet-TDT-0.6B v2** | **6.05%** | **3,386x** | 0.6B | 2024 | NIM, marcpope |
| 5 | Parakeet-TDT-0.6B v3 | 6.34% | ~3,333x | 0.6B | Aug 2025 | NIM, achetronic |
| 6 | Nemotron Streaming 0.6B | 6.93% | real-time | 0.6B | 2025 | NIM |
| 7 | Parakeet-CTC-1.1B | 7.40% | 2,728x | 1.1B | 2025 | NIM |
| 8 | Whisper Large v3 | 7.44% | 145x | 1.55B | Nov 2023 | NIM, faster-whisper |
| — | Microsoft VibeVoice-ASR | 7.77% | 51x | 9B | Jan 2026 | HuggingFace |

RTFx = real-time factor (hours of audio processed per real hour). Higher = faster.

### LibriSpeech WER (gold standard clean/noisy benchmark)

| Model | test-clean | test-other |
|---|---|---|
| Parakeet-TDT-1.1B | 1.39% | 2.62% |
| IBM Granite 4.0 1B | 1.42% | 2.85% |
| Parakeet-RNNT-1.1B | 1.46% | 2.47% |
| Parakeet-TDT-0.6B v2 | 1.69% | 3.19% |
| Parakeet-TDT-0.6B v3 | 1.93% | 3.59% |
| Whisper Large v3 | 2.70% | 5.20% |

### Key takeaway

Parakeet-TDT 0.6B v2 is **23x faster** than Whisper Large v3, uses **5x less VRAM**, and has **better accuracy** (6.05% vs 7.44% avg WER). IBM Granite 4.0 1B just took #1 overall (March 2026) but at 280x RTFx is 12x slower than Parakeet.

**Sources**: [HuggingFace Open ASR Leaderboard](https://huggingface.co/spaces/hf-audio/open_asr_leaderboard), [IBM Research](https://research.ibm.com/blog/granite-speech-recognition-hugging-face-chart)

---

## 2026 ASR Landscape (January–March 2026)

The field has been more active than the leaderboard suggests. Major releases:

### New models worth evaluating

| Model | Org | Params | Languages | Key capability | Released |
|---|---|---|---|---|---|
| **IBM Granite 4.0 1B Speech** | IBM | 1B | 6 | #1 accuracy at 1B params, Apache 2.0 | Mar 2026 |
| **Qwen3-ASR-1.7B** | Alibaba | 1.7B | 52 | Runs on vLLM, language ID, timestamps | Jan 2026 |
| **Voxtral Transcribe 2** | Mistral | 4B | 13 | Real-time (200ms), open-weights, Apache 2.0 | Feb 2026 |
| **VibeVoice-ASR** | Microsoft | 9B | EN | 60-min single-pass, joint diarization | Jan 2026 |
| **Moonshine Voice/v2** | Useful Sensors | 245M | EN | 6.65% WER at 245M params, CPU-only, streaming | Feb 2026 |
| **Meta Omnilingual** | Meta | 7B | 1,600+ | 500 never-before-transcribed languages | Late 2025 |

### Particularly relevant for this project

- **Qwen3-ASR-1.7B** — Same Qwen family as our LLM stack. Runs on vLLM (already configured). 52 languages. Could serve alongside Qwen3.5-35B via llama-swap or vLLM.
- **Moonshine Voice v2** — 245M params, 6.65% WER (beats Whisper at 1.55B). Runs on CPU with streaming. Perfect for always-on dictation without GPU.
- **IBM Granite 4.0 1B** — Current #1 at only 1B params, Apache 2.0. HuggingFace model available.

### Architecture trends

1. **Conformer + LLM decoder** is the winning recipe (Canary-Qwen, Granite, VibeVoice)
2. **Scale efficiency** is the 2026 theme — Granite 4.0 achieving #1 with 1B params
3. **No Whisper v4** — OpenAI shifted to closed API models (gpt-4o-transcribe)
4. **Streaming via sliding-window attention** gaining traction (Moonshine v2, Voxtral)

**Sources**: [Qwen3-ASR](https://github.com/QwenLM/Qwen3-ASR), [IBM Granite](https://research.ibm.com/blog/granite-speech-recognition-hugging-face-chart), [Voxtral](https://mistral.ai/news/voxtral-transcribe-2), [VibeVoice](https://github.com/microsoft/VibeVoice), [Moonshine](https://github.com/moonshine-ai/moonshine), [Meta Omnilingual](https://ai.meta.com/blog/omnilingual-asr-advancing-automatic-speech-recognition/)

---

## Operational Considerations

### Cold start time

| Container | First-ever start | Subsequent starts |
|---|---|---|
| Parakeet CPU (achetronic) | ~2 min (model baked in) | ~5s |
| faster-whisper-server | ~1 min + model download | ~10-15s |
| Speaches | ~1 min + explicit model install | ~10-15s |
| NIM Parakeet | **~30 min** (NGC model download) | ~30s (with cache volume) |
| marcpope/parakeet-api | ~5 min (model baked in) | ~15-20s |

### VRAM coexistence with llama-swap

Qwen3.5-35B at 65k context uses 27.6 GB. Total GPU: 32 GB. Budget: ~4.4 GB for STT.

| Container | VRAM used | Coexists with Qwen3.5-35B? |
|---|---|---|
| Parakeet CPU | 0 GB | Always |
| faster-whisper (tiny) | ~1 GB | Yes |
| faster-whisper (small) | ~2 GB | Yes (tight) |
| faster-whisper (large-v3) | ~6-10 GB | No — must stop llama-swap |
| NIM Parakeet 0.6B | ~2-4 GB | Yes (tight) |
| NIM Parakeet 1.1B | ~4 GB | Marginal |

### Streaming support

| Container | Streaming | Protocol | Latency |
|---|---|---|---|
| NIM Parakeet RNNT 1.1B | Yes | gRPC (Riva) | ~100ms |
| NIM Nemotron Streaming | Yes | gRPC (Riva) | **<24ms** |
| All others | No | HTTP batch | Request-dependent |

For real-time dictation/voice input, only the NIM streaming containers support live audio → partial transcript streaming. All community containers are batch-only (send file → get text).

### API compatibility

All containers except whisperx-blackwell use the OpenAI `/v1/audio/transcriptions` endpoint format:

```bash
curl -X POST http://localhost:<port>/v1/audio/transcriptions \
  -F "file=@audio.wav" \
  -F "model=<model-name>" \
  -F "response_format=json"
```

Response: `{"text": "transcribed text here"}`

This means any OpenAI SDK client works with zero code changes — just change `base_url`.

```python
from openai import OpenAI

client = OpenAI(base_url="http://localhost:8100/v1", api_key="not-needed")
result = client.audio.transcriptions.create(
    model="Systran/faster-whisper-small",
    file=open("audio.wav", "rb"),
)
print(result.text)
```

---

## Native (non-Docker) Option

A native Python install of Parakeet-TDT 0.6B v3 via `onnx-asr` also works:

- **Location**: `D:\LocalLargeLanguageModels\parakeet-stt\`
- **Venv**: `parakeet-stt\venv\`
- **Port**: 5092
- **Status**: Works on CPU. GPU (ONNX Runtime CUDA EP) fails due to cuDNN/Blackwell incompatibility.
- **Startup**: `cd parakeet-stt && venv\Scripts\python.exe app.py`
- **API**: OpenAI-compatible `/v1/audio/transcriptions`

This is a fallback if Docker is unavailable.

---

## Recommendation Matrix

| Use case | Best option | Why |
|---|---|---|
| Quick transcription, VRAM available | `fedirz/faster-whisper-server` (small) | 0.5s latency, GPU, OpenAI API, no auth |
| Coexist with llama-swap | `ghcr.io/achetronic/parakeet` (CPU) | Zero VRAM, 1.9s latency, good enough |
| Best accuracy, English | NIM `parakeet-1-1b-ctc-en-us` | 1.39% WER, native RTX 5090 |
| Real-time streaming | NIM `nemotron-asr-streaming` | <24ms latency, gRPC |
| Multilingual | NIM `parakeet-1-1b-rnnt-multilingual` | 25+ languages, streaming |
| 99-language coverage | NIM `whisper-large-v3` or faster-whisper (large-v3) | Whisper's breadth |
| STT + TTS combo | `ghcr.io/speaches-ai/speaches` | Both in one container |
| Speaker diarization | `mekopa/whisperx-blackwell` | WhisperX + pyannote |

---

## Port Allocation

| Port | Service | Container |
|---|---|---|
| 5092 | Parakeet CPU / native | achetronic/parakeet or native app.py |
| 8000 | Speaches | speaches-ai/speaches |
| 8100 | faster-whisper-server | fedirz/faster-whisper-server |
| 8003 | whisperx-blackwell | mekopa/whisperx-blackwell |
| 9000 | NIM Parakeet | nvcr.io/nim/nvidia/* |
| 50051 | NIM gRPC streaming | nvcr.io/nim/nvidia/* |
