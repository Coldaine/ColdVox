# Windows DLL Requirements

When running ColdVox on Windows or in a Windows CI environment, certain native DLLs and redistributables are required for the application to function correctly, particularly for Python/PyO3 and hardware integrations.

## Required Runtime DLLs

### Visual C++ Redistributable
*   **`VCRUNTIME140.dll`**: Required by Python and many native C/C++ libraries. Install via the Microsoft Visual C++ 2015-2022 Redistributable (e.g., `choco install vcredist140`).
*   **`MSVCP140.dll`**: Also part of the Visual C++ Redistributable.

### PyO3 and Python Dependencies
*   **`python3.dll` / `python3X.dll`**: The core Python runtime DLLs matching the specific Python version (e.g., 3.12) used to build the PyO3 bindings (`coldvox-stt`). These must be in the `PATH` or the executable's directory.

### Machine Learning and CUDA (ONNX / Torch)
*   **ONNX Runtime DLLs**: If using the Parakeet ONNX backend.
*   **CUDA Toolkit DLLs**: If utilizing GPU acceleration (e.g., `cublas64_*.dll`, `cudart64_*.dll`, `cudnn64_*.dll`).

### Troubleshooting
If the application fails to start with error code `0xc0000135` or similar, it usually indicates a missing DLL. Use tools like **Dependencies** (a modern Dependency Walker) to inspect the compiled executable and identify which DLLs are missing from the system path.
