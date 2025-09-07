#!/bin/bash
set -euo pipefail

# Check for cargo-depgraph and install if not found
if ! command -v cargo-depgraph >/dev/null 2>&1; then
  echo "INFO: cargo-depgraph not found, installing..."
  cargo install cargo-depgraph
fi

# Check for graphviz.
# We don't install this automatically as it's a system-level dependency
# and would require sudo, which is not safe or appropriate for a pre-commit hook.
if ! command -v dot >/dev/null 2>&1; then
  echo "ERROR: dot (from graphviz) not found." >&2
  echo "Please install graphviz using your system's package manager (e.g., sudo apt-get install graphviz)." >&2
  exit 1
fi

# Ensure the output directory exists
output_dir="docs/dependency-graphs"
mkdir -p "$output_dir"

echo "Generating workspace dependency graph..."
cargo depgraph --workspace-only > "$output_dir/workspace-deps.dot"
dot -Tpng "$output_dir/workspace-deps.dot" -o "$output_dir/workspace-deps.png"
dot -Tsvg "$output_dir/workspace-deps.dot" -o "$output_dir/workspace-deps.svg"

echo "Generating full dependency graph..."
cargo depgraph --all-deps > "$output_dir/full-deps.dot"
dot -Tsvg "$output_dir/full-deps.dot" -o "$output_dir/full-deps.svg"

echo "Dependency graphs generated successfully."

# Stage the updated files
git add "$output_dir"

echo "Updated dependency graphs staged for commit."
