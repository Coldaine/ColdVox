# Send runner logs to LLM for analysis
journalctl -u actions.runner.Coldaine-ColdVox.laptop-extra.service --since "1 hour ago" | \
  gemini "I see this error in my runner logs: [paste error]. Diagnose and provide fix commands."

# Analyze CI failure
gh run view 18344561673 --log-failed | \
  gemini "My CI failed with these logs. What's wrong and how do I fix it?"

# Get build optimization suggestions
cargo build --workspace --features vosk --timings 2>&1 | \
  gemini "Here's my build timing. What's slow and how can I optimize it?"
