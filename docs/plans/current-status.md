---
doc_type: status
subsystem: general
status: active
---

# Current Product Status

## Target OS

Windows 11 priority.

## Python Environment

Exclusively managed by `uv`. Do NOT use `mise` or raw `pip` for Python packages. Ensure `.python-version` is respected.

## STT Backend Status

| Backend | Status | Notes |
|---------|--------|-------|
| Moonshine | Working | Current working backend, but fragile due to PyO3 dependency |
| Parakeet | Planned | Designated successor for pure-Rust/Windows-native STT pipeline (CUDA/DirectML). Compiles; needs runtime validation. |

## Vaporware (Dead Stubs)

The following feature flags are non-functional and should not be used:

- `whisper` - Removed
- `coqui` - Removed
- `leopard` - Removed
- `silero-stt` - Removed
