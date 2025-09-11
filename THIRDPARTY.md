# Third-Party Components

This file documents bundled or required third‑party components and their licenses/provenance.

## Vosk Speech Recognition Model (Small En-US 0.15)
- Path: `models/vosk-model-small-en-us-0.15/`
- Source: https://alphacephei.com/vosk/models
- Purpose: Offline speech-to-text acoustic + language model for the optional Vosk backend.
- License: Apache License 2.0 (see upstream project). The model directory includes acoustic graph, ivector extractor, and configuration files.
- Integrity: A small checksum file (`SHA256SUMS`) is provided for key model artifacts.

If you update or replace the model:
1. Ensure the license still permits redistribution.
2. Update `SHA256SUMS` with `sha256sum <files> > models/vosk-model-small-en-us-0.15/SHA256SUMS`.
3. Note changes here (date + reason).

| Date | Change | Notes |
|------|--------|-------|
| 2025-09-09 | Initial commit of model + checksums | Added integrity tracking |

## Crates.io Dependencies
All Rust library dependencies are declared per-crate in their respective `Cargo.toml` and are dual-licensed under MIT or Apache-2.0 unless otherwise noted.

## Non-Bundled Optional Tools
- `ydotool`, `kdotool`: Required at runtime for certain injection backends; governed by their own system package licenses.
- `libvosk.so`: Installed at CI / dev setup time from the Vosk distribution (Apache 2.0). Not committed to the repository.

## Contact
For questions about third‑party attribution: open an issue with label `licensing`.
