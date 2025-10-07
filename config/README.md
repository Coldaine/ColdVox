# Application Configuration

This directory contains the configuration files for the ColdVox application.

## `default.toml`

This is the primary configuration file for the application. It contains the default settings for all components, including text injection, VAD, and STT.

The application loads this file at startup. The values in this file can be overridden by environment variables or command-line arguments.

## Security Best Practices

**Important Security Note:** Do not store secrets, API keys, passwords, or any sensitive information in `default.toml` or any committed configuration files. This file is intended for default, non-sensitive values only and should be version-controlled.

- Use environment variables for overriding sensitive values (e.g., `COLDVOX_STT__PREFERRED=your_secret_plugin`). Refer to [docs/user/runflags.md](docs/user/runflags.md) for all overridable variables.
- For local development or production overrides, create a `config/overrides.toml` file with your custom settings. Add `config/overrides.toml` to your `.gitignore` to prevent accidental commits of sensitive data.
- If implementing custom loading, you can extend the config builder in `crates/app/src/main.rs` to include `overrides.toml` after `default.toml` for layered overrides.
- Always validate and sanitize configuration values at runtime to prevent injection attacks or invalid settings.

Example `overrides.toml` template (create this file for local use):

```toml
# Local overrides for default.toml - add to .gitignore!
# This file is not loaded by default; extend Settings::new() if needed.

# Example: Override injection settings
[Injection]
fail_fast = true  # Maps to COLDVOX_INJECTION__FAIL_FAST=true
max_total_latency_ms = 500  # Maps to COLDVOX_INJECTION__MAX_TOTAL_LATENCY_MS=500

# Example: STT preferences (avoid committing model paths with secrets)
[stt]
preferred = "local_whisper"  # Maps to COLDVOX_STT__PREFERRED=local_whisper
max_mem_mb = 2048  # Maps to COLDVOX_STT__MAX_MEM_MB=2048
```

## Deployment Considerations

When deploying ColdVox, handle configurations carefully to ensure security, flexibility, and reliability across environments.

### Including config/default.toml in Builds and Deployments
- **Repository**: Always commit `config/default.toml` as it holds safe, default values. Do not modify it for environment-specific needs.
- **Build Process**: The TOML is loaded at runtime, not embedded. In CI/CD (e.g., via `cargo build --release`), copy `config/default.toml` to the deployment artifact or container.
  - Example in Dockerfile:
    ```
    COPY config/default.toml /app/config/
    COPY target/release/coldvox-app /app/
    WORKDIR /app
    CMD ["./coldvox-app"]
    ```
  - For binary distributions: Include in a `config/` subdirectory next to the executable.
- **Runtime Loading**: The app loads `config/default.toml` relative to the working directory. XDG support not implemented; to add it, extend `Settings::new()` with XDG path lookup (see deployment docs for details).

### Environment-Specific Configurations
- **Overrides via Environment Variables**: Preferred for secrets and dynamic settings. Use `COLDVOX__` prefix:
  - Example for production: `export COLDVOX_STT__PREFERRED=cloud_whisper; export COLDVOX_INJECTION__FAIL_FAST=true`.
  - Nested: `COLDVOX_VAD__SENSITIVITY=0.8` overrides `[vad].sensitivity`.
  - Set in deployment tools: Systemd (`Environment=`), Docker (`-e`), Kubernetes (Secrets/ConfigMaps).
- **Separate TOML Files for Non-Secrets**: Use `overrides.toml` (or env-specific like `staging.toml`) for bulk overrides. Extend the loader in `crates/app/src/main.rs` to support `COLDVOX_CONFIG_OVERRIDE_PATH=/path/to/staging.toml`.
  - Template extension for staging:
    ```toml
    # staging.toml - non-sensitive overrides
    [stt]
    preferred = "vosk"
    language = "en"

    [injection]
    injection_mode = "keystroke"  # Staging: Test keystroke reliability
    ```
  - Current load order: CLI flags > Env vars > default.toml > hardcoded defaults. Note: `overrides.toml` is a template and NOT automatically loaded. To enable, add `.add_source(File::with_name("config/overrides.toml").required(false))` to `Settings::new()`.
- **Validation**: On deploy, validate configs (see [docs/deployment.md](docs/deployment.md) for steps, including parsing checks and tests).

### Best Practices
- **Secrets Management**: Use tools like HashiCorp Vault, AWS Secrets Manager, or env files (`.env` with `dotenv` if extended).
- **Rollback**: Backup configs before deploy; fallback to env vars if TOML fails.
- **CI Integration**: Test config loading in workflows (e.g., set mock env vars in `.github/workflows/ci.yml`).
- For full deployment details, including validation and rollback, refer to [docs/deployment.md](docs/deployment.md).

## `plugins.json`

This file contains the configuration for the STT (Speech-to-Text) plugin manager. It defines the preferred plugin, fallback plugins, and other settings related to plugin management.

While the main application configuration is in `default.toml`, this file is kept separate to potentially allow for dynamic updates or for management by external tools in the future.

## For Test Authors

Tests that need to load configuration should use `Settings::from_path()` with `CARGO_MANIFEST_DIR`:

```rust
#[cfg(test)]
use std::env;
use std::path::PathBuf;

fn get_test_config_path() -> PathBuf {
    // Try workspace root first (for integration tests)
    let workspace_config = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("config/default.toml");

    if workspace_config.exists() {
        return workspace_config;
    }

    // Fallback to relative path from crate root
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../config/default.toml")
}

#[test]
fn my_test() {
    let config_path = get_test_config_path();
    let settings = Settings::from_path(&config_path)?;
    // ... test logic
}
```

This ensures tests work regardless of working directory context.
