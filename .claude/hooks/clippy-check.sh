#!/bin/bash
cd /home/coldaine/Projects/ColdVox/crates/app
OUTPUT=$(cargo clippy --quiet 2>&1)

if [ -n "$OUTPUT" ]; then
    # Escape the output for JSON
    ESCAPED_OUTPUT=$(echo "$OUTPUT" | jq -Rs .)
    echo "{\"type\": \"context\", \"context\": \"Clippy found issues:\\n${ESCAPED_OUTPUT:1:-1}\"}"
else
    echo '{"type": "continue"}'
fi