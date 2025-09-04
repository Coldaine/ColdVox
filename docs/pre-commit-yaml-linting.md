# Pre-commit Hook Configuration for YAML and GitHub Workflows

## Overview

This document describes the pre-commit hook system configured for ColdVox to automatically format and validate YAML files, with special attention to GitHub Actions workflows.

## Architecture

The system uses a layered approach:
1. **Prettier** - First pass: Auto-formats YAML files for consistent style
2. **Yamllint** - Second pass: Validates YAML syntax and structure
3. **Actionlint** - Third pass: Validates GitHub Actions workflow syntax
4. **Additional hooks** - General code quality checks

## Components

### 1. Pre-commit Framework

**Installation**: `pip install pre-commit`

The pre-commit framework manages Git hooks and runs configured tools before each commit. It's installed in the Python virtual environment and creates a git hook at `.git/hooks/pre-commit`.

### 2. Prettier (Auto-formatter)

**Purpose**: Automatically fixes formatting issues in YAML files

**Configuration** (`.pre-commit-config.yaml`):
```yaml
- repo: https://github.com/pre-commit/mirrors-prettier
  rev: v3.1.0
  hooks:
    - id: prettier
      types: [yaml]
      args: [--write]
```

**Auto-fixes**:
- Indentation inconsistencies
- Trailing spaces
- Missing newlines at end of files
- Quote style consistency
- Key ordering and spacing

### 3. Yamllint (Validator)

**Purpose**: Validates YAML files after formatting

**Configuration** (`.yamllint.yaml`):
```yaml
extends: default

rules:
  # Lenient line length for GitHub Actions
  line-length:
    max: 150
    level: warning

  # Support GitHub Actions 'on:' syntax
  truthy:
    allowed-values: ['true', 'false', 'on']
    check-keys: false

  # Disabled rules (handled by Prettier)
  indentation: disable
  trailing-spaces: disable
  new-line-at-end-of-file: disable
  empty-lines: disable

  # Flexible formatting
  brackets:
    min-spaces-inside: 0
    max-spaces-inside: -1
  quoted-strings:
    quote-type: any
    required: false
```

**Key decisions**:
- Indentation validation disabled (Prettier handles this)
- Line length increased to 150 chars (GitHub Actions can have long lines)
- Supports GitHub Actions specific syntax (`on:` keyword)
- Trailing space/newline checks disabled (Prettier handles these)

### 4. Actionlint (GitHub Actions Validator)

**Purpose**: Validates GitHub Actions workflow syntax and semantics

**Configuration** (`.pre-commit-config.yaml`):
```yaml
- repo: https://github.com/rhysd/actionlint
  rev: v1.7.4
  hooks:
    - id: actionlint
```

**Validates**:
- Workflow syntax correctness
- Job dependencies
- Action versions
- Expression syntax (`${{ }}`)
- Required/optional inputs
- Output references
- Secret usage

### 5. Additional Hooks

From `pre-commit-hooks`:
- **check-yaml**: Validates YAML can be parsed (with `--unsafe` for GitHub Actions tags)
- **end-of-file-fixer**: Ensures files end with newline
- **trailing-whitespace**: Removes trailing whitespace
- **check-merge-conflict**: Detects merge conflict markers
- **mixed-line-ending**: Enforces LF line endings

## Usage

### Initial Setup

```bash
# Install pre-commit
pip install pre-commit

# Install git hooks
pre-commit install

# Run on all files (initial cleanup)
pre-commit run --all-files
```

### Daily Usage

The hooks run automatically on `git commit`. Two scenarios:

**1. Auto-fixable issues** (formatting):
```bash
$ git commit -m "Update workflows"
prettier.................................................................Failed
- hook id: prettier
- files were modified by this hook
```
Files are auto-fixed. Review changes and commit again.

**2. Manual fix required** (validation errors):
```bash
$ git commit -m "Update workflows"
yamllint.................................................................Failed
- hook id: yamllint
- exit code: 1

.github/workflows/ci.yml
  10:5  error  wrong value for 'on'  (truthy)
```
Fix the error manually and commit again.

### Bypassing Hooks (Emergency Only)

```bash
git commit --no-verify -m "Emergency fix"
```

**Warning**: Only use when absolutely necessary. Always run `pre-commit run --all-files` afterward.

### Manual Validation

```bash
# Check specific files
pre-commit run --files .github/workflows/ci.yml

# Check all files
pre-commit run --all-files

# Run specific hook
pre-commit run yamllint --all-files
```

## File Structure

```
ColdVox/
├── .pre-commit-config.yaml   # Pre-commit hooks configuration
├── .yamllint.yaml            # Yamllint rules configuration
├── .git/
│   └── hooks/
│       └── pre-commit        # Generated hook script
└── .github/
    └── workflows/            # GitHub Actions workflows (validated)
```

## What Gets Auto-Fixed

### By Prettier
- ✅ Indentation (converts to 2 spaces)
- ✅ Trailing spaces removal
- ✅ End-of-file newlines
- ✅ Consistent quote usage
- ✅ Array/list formatting
- ✅ Key-value spacing

### By Other Hooks
- ✅ Line ending conversion (CRLF → LF)
- ✅ Final newline addition
- ✅ Trailing whitespace in all file types

## What Requires Manual Fixes

### Yamllint Errors
- ❌ Invalid YAML syntax
- ❌ Duplicate keys
- ❌ Invalid references

### Actionlint Errors
- ❌ Invalid workflow syntax
- ❌ Undefined variables/secrets
- ❌ Invalid job dependencies
- ❌ Incorrect action usage
- ❌ Expression syntax errors

## Common Issues and Solutions

### Issue 1: Line Too Long Warning
```yaml
# Warning: line too long (160 > 150 characters)
- run: some-very-long-command --with --many --flags --that --exceed --the --line --limit
```

**Solution**: Break into multiline
```yaml
- run: |
    some-very-long-command \
      --with --many --flags \
      --that --exceed \
      --the --line --limit
```

### Issue 2: GitHub Actions Expression Errors
```yaml
# Error: undefined variable 'test_strategy'
strategy: "${{ github.event.inputs.test_strategy || 'curated' }}"
```

**Solution**: Define inputs or use defaults
```yaml
strategy: "curated"  # Or define workflow_dispatch inputs
```

### Issue 3: Indentation After Auto-Fix
Sometimes Prettier and your manual edits conflict.

**Solution**: Let Prettier win, then adjust your content to work with its formatting.

## Maintenance

### Updating Hook Versions

```bash
# Update all hooks to latest versions
pre-commit autoupdate

# Commit the changes
git add .pre-commit-config.yaml
git commit -m "chore: update pre-commit hooks"
```

### Adding New Hooks

Edit `.pre-commit-config.yaml`:
```yaml
repos:
  # ... existing repos ...
  - repo: https://github.com/new/hook-repo
    rev: vX.Y.Z
    hooks:
      - id: new-hook
        args: [--some-arg]
```

Then run:
```bash
pre-commit install --install-hooks
```

## Philosophy

The configuration follows these principles:

1. **Auto-fix when possible**: Formatting should be automatic
2. **Validate what matters**: Focus on actual errors, not style preferences
3. **Fast feedback**: Hooks should run quickly
4. **Clear errors**: Error messages should indicate what needs fixing
5. **Escape hatch available**: Allow bypassing in emergencies

## Integration with CI/CD

The same tools run in CI to ensure consistency:

```yaml
# .github/workflows/ci.yml
- name: Run pre-commit
  run: |
    pip install pre-commit
    pre-commit run --all-files
```

This ensures that code passing local hooks will also pass CI checks.

## Troubleshooting

### Hooks Not Running

```bash
# Verify installation
ls -la .git/hooks/pre-commit

# Reinstall
pre-commit install
```

### Prettier Keeps Modifying Files

This usually means files weren't formatted before enabling pre-commit.

```bash
# One-time cleanup
pre-commit run prettier --all-files
git add -u
git commit -m "chore: format all YAML files"
```

### Permission Errors

```bash
# Fix permissions
chmod +x .git/hooks/pre-commit
```

### Cache Issues

```bash
# Clear pre-commit cache
pre-commit clean
pre-commit gc
```

## Benefits

1. **Consistency**: All YAML files follow the same format
2. **Error Prevention**: Catch workflow errors before pushing
3. **Time Saving**: No manual formatting needed
4. **Quality**: Enforces best practices
5. **Team Alignment**: Everyone uses the same standards

## Appendix: Full Configuration Files

See the actual configuration files:
- `.pre-commit-config.yaml` - Hook definitions
- `.yamllint.yaml` - YAML validation rules
- `.github/workflows/` - Example formatted workflows
