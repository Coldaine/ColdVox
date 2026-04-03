---
doc_type: status
subsystem: general
status: active
---

# Current Product Direction & Reality

- **Target OS:** Windows 11 priority.
- **Python Environment:** Exclusively managed by `uv`. Do NOT use `mise` or raw `pip` for Python packages. Ensure `.python-version` is respected.
- **STT Backend:**
  - **Moonshine:** The current working backend, but considered a fragile dependency due to PyO3.
  - **Parakeet:** The designated successor for a pure-Rust/Windows-native STT pipeline (CUDA/DirectML). It *does* compile; focus on runtime validation.
  - **Vaporware:** The `whisper`, `coqui`, `leopard`, and `silero-stt` feature flags are dead stubs. Do not attempt to use them.
