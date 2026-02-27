#!/bin/bash
set -e

# Download external test audio datasets for audio quality monitoring tests
# Storage strategy: Hybrid - small samples in repo, large datasets downloaded on-demand

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
TEST_AUDIO_DIR="$PROJECT_ROOT/test_audio"

echo "=========================================="
echo "ColdVox Audio Quality Test Data Downloader"
echo "=========================================="
echo ""
echo "This script downloads real audio datasets for validating:"
echo "  - Off-axis speech detection (Pyramic anechoic recordings)"
echo "  - Baseline good audio (LibriSpeech professional recordings)"
echo "  - Too quiet audio (DAPS consumer device recordings - optional)"
echo ""
echo "Total download size: ~1.5GB (minimal) or ~5.5GB (with DAPS)"
echo ""

# Create directories
mkdir -p "$TEST_AUDIO_DIR"/{baseline,off_axis,quiet}

# Parse command line arguments
DOWNLOAD_DAPS=false
while [[ $# -gt 0 ]]; do
    case $1 in
        --with-daps)
            DOWNLOAD_DAPS=true
            shift
            ;;
        --help)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --with-daps    Also download DAPS dataset (~4GB) for quiet audio testing"
            echo "  --help         Show this help message"
            echo ""
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            echo "Run with --help for usage information"
            exit 1
            ;;
    esac
done

# Check for required tools
for tool in wget tar unzip ffmpeg; do
    if ! command -v $tool &> /dev/null; then
        echo "ERROR: Required tool '$tool' is not installed"
        exit 1
    fi
done

echo "Step 1/3: Downloading LibriSpeech test-clean (346MB)"
echo "------------------------------------------------------"
echo "Source: https://www.openslr.org/12"
echo "Purpose: Baseline professional recordings (should produce zero warnings)"
echo ""

if [ ! -f "$TEST_AUDIO_DIR/baseline/test-clean.tar.gz" ]; then
    wget -P "$TEST_AUDIO_DIR/baseline" \
        https://www.openslr.org/resources/12/test-clean.tar.gz
    echo "✓ Downloaded LibriSpeech test-clean"
else
    echo "✓ LibriSpeech test-clean already downloaded"
fi

if [ ! -d "$TEST_AUDIO_DIR/baseline/LibriSpeech" ]; then
    tar -xzf "$TEST_AUDIO_DIR/baseline/test-clean.tar.gz" \
        -C "$TEST_AUDIO_DIR/baseline"
    echo "✓ Extracted LibriSpeech test-clean"
else
    echo "✓ LibriSpeech test-clean already extracted"
fi

echo ""
echo "Step 2/3: Downloading Pyramic anechoic dataset (~1-2GB)"
echo "--------------------------------------------------------"
echo "Source: https://zenodo.org/records/1209563"
echo "Purpose: Off-axis speech detection (recordings at 0°, 90°, 180°)"
echo ""
echo "NOTE: This is a manual download due to Zenodo's authentication."
echo ""
echo "Please visit: https://zenodo.org/records/1209563"
echo "Download: 'pyramic_impulse_responses_anechoic.tar.gz' or 'pyramic_signals_anechoic.tar.gz'"
echo "Save to: $TEST_AUDIO_DIR/off_axis/"
echo ""
echo "Once downloaded, extract with:"
echo "  tar -xzf $TEST_AUDIO_DIR/off_axis/pyramic_*.tar.gz -C $TEST_AUDIO_DIR/off_axis/"
echo ""
read -p "Press Enter after downloading Pyramic dataset (or Ctrl+C to skip)..."

if [ -f "$TEST_AUDIO_DIR/off_axis/pyramic_"*.tar.gz ]; then
    tar -xzf "$TEST_AUDIO_DIR/off_axis/pyramic_"*.tar.gz \
        -C "$TEST_AUDIO_DIR/off_axis/"
    echo "✓ Extracted Pyramic dataset"
else
    echo "⚠ Pyramic dataset not found - tests requiring off-axis audio will be skipped"
fi

echo ""
if [ "$DOWNLOAD_DAPS" = true ]; then
    echo "Step 3/3: Downloading DAPS dataset (~4GB)"
    echo "-----------------------------------------"
    echo "Source: https://zenodo.org/records/4660670"
    echo "Purpose: Too quiet audio (consumer device recordings)"
    echo ""

    if [ ! -f "$TEST_AUDIO_DIR/quiet/daps.zip" ]; then
        wget -P "$TEST_AUDIO_DIR/quiet" \
            https://zenodo.org/records/4660670/files/daps.zip
        echo "✓ Downloaded DAPS dataset"
    else
        echo "✓ DAPS dataset already downloaded"
    fi

    if [ ! -d "$TEST_AUDIO_DIR/quiet/daps" ]; then
        unzip -q "$TEST_AUDIO_DIR/quiet/daps.zip" -d "$TEST_AUDIO_DIR/quiet"
        echo "✓ Extracted DAPS dataset"
    else
        echo "✓ DAPS dataset already extracted"
    fi
else
    echo "Step 3/3: Skipping DAPS dataset (use --with-daps to include)"
    echo "------------------------------------------------------------"
    echo "DAPS provides consumer device recordings for quiet audio testing."
    echo "It's optional - synthetic and committed samples may be sufficient."
fi

echo ""
echo "Step 4/4: Converting audio to 16kHz mono"
echo "-----------------------------------------"
echo "Converting any non-16kHz audio to 16kHz mono WAV..."
echo ""

# Convert LibriSpeech FLAC to WAV if needed
find "$TEST_AUDIO_DIR/baseline" -name "*.flac" -type f | head -5 | while read file; do
    wav_file="${file%.flac}_16k.wav"
    if [ ! -f "$wav_file" ]; then
        ffmpeg -i "$file" -ar 16000 -ac 1 -sample_fmt s16 "$wav_file" -y -loglevel error
        echo "  ✓ Converted $(basename "$file")"
    fi
done

echo ""
echo "=========================================="
echo "✓ Download complete!"
echo "=========================================="
echo ""
echo "Test audio location: $TEST_AUDIO_DIR"
echo ""
echo "Available datasets:"
echo "  - LibriSpeech test-clean: $TEST_AUDIO_DIR/baseline/LibriSpeech/"
if [ -d "$TEST_AUDIO_DIR/off_axis/pyramic" ]; then
    echo "  - Pyramic anechoic:       $TEST_AUDIO_DIR/off_axis/"
fi
if [ "$DOWNLOAD_DAPS" = true ]; then
    echo "  - DAPS consumer devices:  $TEST_AUDIO_DIR/quiet/daps/"
fi
echo ""
echo "Run integration tests with:"
echo "  cd crates/coldvox-audio-quality"
echo "  cargo test --test integration_test"
echo ""
