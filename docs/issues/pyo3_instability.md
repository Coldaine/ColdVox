---
doc_type: troubleshooting
subsystem: stt
version: 1.0.0
status: draft
owners: Documentation Working Group
last_reviewed: 2025-12-12
---

# Issue: PyO3 0.27 Instability on Python 3.13 (Moonshine Backend)

**Status**: DRAFT (Local)
**Created**: 2025-12-10
**Priority**: High (Blocks stable build on modern Linux distros)

## Problem
PyO3 0.27 introduces breaking changes and strict requirements for Python 3.13 compatibility, specifically regarding free-threaded builds (GIL removal). This impacts the `moonshine` STT plugin in ColdVox.

## Symptoms
- Build errors on systems with Python 3.13 default (e.g., Arch, Fedora Rawhide).
- Potential runtime panics if `#[pyclass]` structs do not implement `Sync`.
- API deprecations/renames (`Python::with_gil` replaced by `Python::attach`).

## Findings from Research
1. **Free-Threading (3.13t)**: Python 3.13 supports experimental free-threading. PyO3 0.27 requires `Sync` implementation for all `#[pyclass]` types to support this.
2. **API Change**: `Python::with_gil` is replaced by `Python::attach` in PyO3 0.27 for improved compatibility with free-threaded Python.
3. **Build Tooling**: Attempting to build against Python 3.13 with older versions (or mismatched feature flags) fails.
4. **Current Config**: `coldvox-stt` uses `pyo3 = "0.27"`.

## Impact on ColdVox
`moonshine.rs` uses `Python::with_gil` extensively. If the system Python is 3.13, the build may produce unstable binaries or fail link checks because our `MoonshinePlugin` struct holds `Py<PyAny>` fields that might need `Sync` guards in the new model.

## Recommendation
1. **Short Term**: Pin Python to 3.12 for stability via `.python-version` or `pyenv`.
2. **Code Change**: Audit all `Py<T>` usage in `moonshine.rs` for `Sync` compliance.
3. **Configuration**: Consider enabling `abi3-py313` feature in `Cargo.toml` or setting `PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1`.
