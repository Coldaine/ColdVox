#!/bin/bash
# Qt 6 Detection Script
# Extracted from .github/workflows/ci.yml for maintainability

set -euo pipefail

echo "=== Qt 6 Detection Script ==="

echo "Runner OS: $(lsb_release -a 2>/dev/null || echo 'lsb_release not found, and is not required')"
echo "Initial PATH: $PATH"

qt6_found=false
qmake_path=""
detection_method=""

# 1. Check for QT6_PATH environment variable override
if [[ -n "${QT6_PATH:-}" ]] && [[ -x "${QT6_PATH}/bin/qmake" ]]; then
    qmake_path="${QT6_PATH}/bin/qmake"
    detection_method="QT6_PATH override ($qmake_path)"
fi

# 2. Find qmake executable (qmake6, qmake-qt6, qmake) in PATH
if [[ -z "$qmake_path" ]]; then
    for qmake_candidate in qmake6 qmake-qt6 qmake; do
        if command -v "$qmake_candidate" >/dev/null 2>&1; then
            qmake_path=$(command -v "$qmake_candidate")
            detection_method="qmake in PATH ($qmake_path)"
            break
        fi
    done
fi

# 3. If qmake was found, verify it's Qt 6
if [[ -n "$qmake_path" ]]; then
    echo "Found qmake executable at: $qmake_path"
    qt_version=$($qmake_path -query QT_VERSION)
    echo "qmake -query QT_VERSION returned: $qt_version"
    if [[ "$qt_version" == 6.* ]]; then
        qt6_found=true
        echo "✅ Qt 6 detected via qmake version check."
    else
        echo "⚠️ Found qmake, but it's not for Qt 6 (version $qt_version). Resetting search."
        qmake_path=""
        detection_method=""
    fi
fi

# 4. Fallback to pkg-config
if [[ "$qt6_found" == "false" ]] && command -v pkg-config >/dev/null 2>&1; then
    echo "Attempting detection via pkg-config..."
    if pkg-config --exists Qt6Core; then
        qt6_found=true
        detection_method="pkg-config"
        echo "✅ Qt 6 detected via pkg-config."
    else
        echo "pkg-config did not find Qt6Core."
    fi
fi

# 5. Fallback to CMake
if [[ "$qt6_found" == "false" ]] && command -v cmake >/dev/null 2>&1; then
    echo "Attempting detection via CMake..."
    # Create a minimal CMakeLists.txt to find Qt6
    cat <<EOF > CMakeLists.txt
cmake_minimum_required(VERSION 3.16)
project(Qt6Check)
find_package(Qt6 QUIET REQUIRED)
EOF
    if cmake -B build -S . >/dev/null 2>&1; then
        qt6_found=true
        detection_method="CMake find_package"
        echo "✅ Qt 6 detected via CMake."
    else
        echo "CMake find_package(Qt6) failed."
    fi
    rm -rf CMakeLists.txt build
fi

echo "============================="
echo "Final detection result: qt6=$qt6_found"
if [[ "$qt6_found" == "true" ]]; then
    echo "Qt 6 detected on runner using method: $detection_method."
    exit 0
else
    echo "Qt 6 not detected; will skip qt-ui build and explicitly pass."
    exit 1
fi