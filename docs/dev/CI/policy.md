# CI Policy

## Runner Strategy

- **GitHub-hosted runners**: Handle fast general CI work.
- **Self-hosted Fedora/Nobara runner**: Handles hardware-dependent tests.

## Do Not Use

- Xvfb on self-hosted runner
- `apt-get` on Fedora runner
- `DISPLAY=:99` in self-hosted jobs
