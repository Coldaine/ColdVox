# Makefile

# Fast tests for pre-commit (< 3 seconds)
.PHONY: test-fast
test-fast:
	@echo "Running fast injection tests..."
	@pytest tests/injection/fast/ -m "not hardware" --timeout=0.5 -q

# Hardware tests (non-blocking)
.PHONY: test-hardware
test-hardware:
	@echo "Starting hardware tests (non-blocking)..."
	@python -m tests.hardware.runner --non-blocking

# Complete E2E with real audio (blocking, for release)
.PHONY: test-e2e-audio
test-e2e-audio:
	@echo "Running complete WAV→injection tests..."
	@pytest tests/e2e/ -m "e2e and audio" --timeout=30

# Pre-release gate
.PHONY: test-release
test-release: test-fast
	@echo "Running release tests..."
	@pytest tests/e2e/ -m "release_gate" --timeout=60
	@python -m tests.hardware.runner --blocking --required-only

# Install git hooks
.PHONY: install-hooks
install-hooks:
	@cp hooks/pre-commit .git/hooks/
	@chmod +x .git/hooks/pre-commit
	@echo "Pre-commit hook installed"

# Developer setup
.PHONY: dev-setup
dev-setup: install-hooks
	@pip install -e ".[dev]"
	@echo "Development environment ready"