# ColdVox Runner Agent

This directory contains documentation, scripts, and prompts for managing and debugging the self-hosted GitHub Actions runner used for ColdVox CI/CD.

## Contents

- **[RunnerAgent.md](RunnerAgent.md)** - Complete architecture and operational guide
- **[prompts/](prompts/)** - LLM prompts for debugging and optimization
- **[scripts/](scripts/)** - Helper scripts for runner management (symlinked from repo root)

## Quick Start

### Daily Workflow
```bash
# 1. Update toolchain
rustup update stable

# 2. Run health check
bash scripts/runner_health_check.sh

# 3. Simulate CI locally
cd /home/coldaine/actions-runner/_work/ColdVox/ColdVox
bash scripts/ci/setup-vosk-cache.sh
cargo check --workspace --features vosk
```

### Debugging CI Failures
```bash
# View runner logs
journalctl -u actions.runner.Coldaine-ColdVox.laptop-extra.service --since "1 hour ago"

# Check environment
cd /home/coldaine/actions-runner/_work/ColdVox/ColdVox
env | grep -E "(RUST|CARGO|VOSK|LD_LIBRARY)"

# Re-run failing command
cargo build --workspace --features vosk
```

## Key Principles

1. **Test before you push** - Simulate CI locally first
2. **Direct access wins** - Leverage local hardware for faster debugging
3. **Minimal tooling** - Bash, systemd, cargo only
4. **LLM-assisted** - Use CLI tools like `gemini` for complex diagnostics

## Related Documentation

- [Architecture Details](RunnerAgent.md) - Full system design
- [Debug Agent Prompt](prompts/debug_agent_prompt.md) - LLM assistant configuration
- [CI Workflows](../../.github/workflows/) - GitHub Actions definitions
