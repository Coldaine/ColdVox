#!/usr/bin/env bash
set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

if [[ ! -d .venv ]]; then
  echo "Creating Python 3.12 virtual environment in .venv..."
  python3.12 -m venv .venv
fi

source .venv/bin/activate

# Ensure required Python deps for whisper
python -m pip install --quiet --upgrade pip
python -m pip install --quiet faster-whisper

# Exec any passed command within venv
if [[ $# -gt 0 ]]; then
  exec "$@"
else
  echo "Venv is ready and activated."
fi
