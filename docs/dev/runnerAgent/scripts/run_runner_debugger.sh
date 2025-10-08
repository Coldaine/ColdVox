#!/usr/bin/env bash
set -euo pipefail

# Runner debugger script
# Usage:
#   ./run_runner_debugger.sh [RUNNER_PATH] [COMMAND]
# Defaults:
#   RUNNER_PATH=/home/coldaine/actions-runner/_work/ColdVox/ColdVox
#   COMMAND='cargo build --workspace --features vosk'

RUNNER_PATH=${1:-/home/coldaine/actions-runner/_work/ColdVox/ColdVox}
CMD=${2:-"cargo build --workspace --features vosk"}

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
PROMPTS_DIR="$SCRIPT_DIR/../prompts"
DEBUG_PROMPT="$PROMPTS_DIR/debug_agent_prompt.md"
SYS_PROMPT="$PROMPTS_DIR/system_update_prompt.md"

OUT_BASE="$SCRIPT_DIR/../debug_runs"
TIMESTAMP=$(date -u +%Y%m%dT%H%M%SZ)
OUT_DIR="$OUT_BASE/$TIMESTAMP"
mkdir -p "$OUT_DIR"

# Allow the user to override the LLM command. Default to `gemini` if available.
LLM_CMD=${LLM_CMD:-}
if [ -z "$LLM_CMD" ]; then
  if command -v gemini >/dev/null 2>&1; then
    LLM_CMD="gemini"
  else
    echo "No LLM CLI found. Set LLM_CMD to a CLI command (e.g. 'gemini' or 'openai chat') and ensure it's in PATH." >&2
    exit 1
  fi
fi

# Timeout for LLM calls (seconds)
LLM_TIMEOUT=${LLM_TIMEOUT:-300}


echo "Using runner workspace: $RUNNER_PATH"
echo "Reproduction command: $CMD"
echo "Outputs will be saved to: $OUT_DIR"

# Collect recent runner service logs
echo "Collecting runner journal logs..."
journalctl -u actions.runner.Coldaine-ColdVox.laptop-extra.service --since "1 hour ago" > "$OUT_DIR/journal.log" || true

# Ensure the runner workspace exists
if [ ! -d "$RUNNER_PATH" ]; then
  echo "Runner workspace path does not exist: $RUNNER_PATH" >&2
  echo "If you're running remotely, SSH into the runner or adjust the path." >&2
  exit 2
fi

cd "$RUNNER_PATH"

# Run the reproduction command and capture output
echo "Running reproduction command in $RUNNER_PATH ..."
bash -lc "$CMD" 2>&1 | tee "$OUT_DIR/build.log" || true

# iterative debugging loop (max 2 iterations)
for ITER in 1 2; do
  echo "\n=== Debug iteration $ITER ==="

  COMBINED="$OUT_DIR/combined_iter_${ITER}.txt"
  {
    echo "--- SYSTEM PROMPT (debug_agent_prompt.md) ---\n"
    sed 's/\r$//' "$DEBUG_PROMPT"
    echo "\n--- RUNNER JOURNAL (last 1h) ---\n"
    sed 's/\r$//' "$OUT_DIR/journal.log" || true
    echo "\n--- LAST BUILD OUTPUT ---\n"
    sed 's/\r$//' "$OUT_DIR/build.log" || true
  } > "$COMBINED"

  RESPONSE="$OUT_DIR/response_iter_${ITER}.txt"

  # Prefer piping the combined file into gemini; this works with most CLI builds
  echo "Sending combined logs and prompt to LLM ($LLM_CMD) with timeout ${LLM_TIMEOUT}s..."
  # Run the configured LLM command headless with a timeout. Capture stderr to a file for debugging.
  # The command string should read from stdin and write to stdout.
  # Use sh -c to allow complex commands in LLM_CMD (e.g. 'gemini --model=gpt-4o-mini').
  GEMINI_ERR="$OUT_DIR/gemini_error_iter_${ITER}.log"
  set +e
  timeout "$LLM_TIMEOUT" sh -c "cat '$COMBINED' | $LLM_CMD" > "$RESPONSE" 2> "$GEMINI_ERR"
  LLM_EXIT=$?
  set -e
  if [ $LLM_EXIT -eq 124 ]; then
    echo "LLM command timed out after ${LLM_TIMEOUT}s. See $GEMINI_ERR for stderr." >&2
  elif [ $LLM_EXIT -ne 0 ]; then
    echo "LLM command exited with code $LLM_EXIT. See $GEMINI_ERR for stderr." >&2
  fi
  if [ -s "$GEMINI_ERR" ]; then
    echo "LLM stderr captured to: $GEMINI_ERR"
  fi

  echo "Gemini response saved to: $RESPONSE"

  # Extract suggested commands from fenced code blocks (```bash ... ```)
  CMD_OUT="$OUT_DIR/suggested_commands_iter_${ITER}.sh"
  awk '/```bash/{flag=1;next}/```/{flag=0}flag{print}' "$RESPONSE" > "$CMD_OUT" || true

  if [ -s "$CMD_OUT" ]; then
    chmod +x "$CMD_OUT"
    echo "Suggested commands extracted to: $CMD_OUT"
    echo "Review the commands and run them manually if appropriate."
  else
    echo "No explicit \`bash\` code blocks found in LLM response. Check $RESPONSE for suggestions." 
  fi

  # If it's the first iteration and the response suggests a system update, also run the system update prompt
  # (We just save the system update prompt + response for the user to review.)
  SYS_COMBINED="$OUT_DIR/system_update_combined_iter_${ITER}.txt"
  {
    echo "--- SYSTEM UPDATE PROMPT (system_update_prompt.md) ---\n"
    sed 's/\r$//' "$SYS_PROMPT"
    echo "\n--- CURRENT ENV ---\n"
    env | grep -E "(RUST|CARGO|VOSK|LD_LIBRARY|PATH)"
  } > "$SYS_COMBINED"
  timeout "$LLM_TIMEOUT" sh -c "cat '$SYS_COMBINED' | $LLM_CMD" > "$OUT_DIR/system_update_response_iter_${ITER}.txt" 2>> "$GEMINI_ERR" || true

  # If not last iteration, ask user whether to continue after printing summary
  if [ $ITER -lt 2 ]; then
    echo "Iteration $ITER complete. Review:"
    echo " - Gemini response: $RESPONSE"
    if [ -s "$CMD_OUT" ]; then
      echo " - Suggested commands file: $CMD_OUT"
    fi
    echo " - System update response: $OUT_DIR/system_update_response_iter_${ITER}.txt"
    echo "You may run suggested commands manually now. To continue to next iteration, press ENTER; to stop, type 'q' and press ENTER."
    read -r USER_CHOICE
    if [ "$USER_CHOICE" = "q" ]; then
      echo "User requested stop. Exiting iterative debug loop."
      break
    fi
    # Optionally, re-run the reproduction command to get fresh build output
    echo "Re-running reproduction command to gather fresh logs..."
    bash -lc "$CMD" 2>&1 | tee "$OUT_DIR/build_iter_${ITER}_postfix.log" || true
    # Append new build output to journal of this iteration
    cat "$OUT_DIR/build_iter_${ITER}_postfix.log" >> "$OUT_DIR/build.log"
  fi
done

# Create a notification file summarizing the debug run
NOTIFY_FILE="$SCRIPT_DIR/../debug_runs/notification_${TIMESTAMP}.md"
cat > "$NOTIFY_FILE" <<EOF
# Runner debug run - $TIMESTAMP

Runner workspace: $RUNNER_PATH
Reproduction command: $CMD
Output directory: $OUT_DIR

Files produced:
 - journal.log
 - build.log
 - response_iter_1.txt
 - suggested_commands_iter_1.sh (if present)
 - system_update_response_iter_1.txt
 - response_iter_2.txt (if produced)
 - suggested_commands_iter_2.sh (if present)

Review the Gemini responses and suggested command files in the output directory and apply fixes as needed.

EOF

echo "Debug run complete. Notification file created: $NOTIFY_FILE"
echo "If you want this script to open an issue or push a git commit with the notification, run the following manually:"
echo "  cp $NOTIFY_FILE /path/to/repo && git add <file> && git commit -m 'ci: runner debug report' && git push"

exit 0
