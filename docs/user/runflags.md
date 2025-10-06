# ColdVox Runtime Flags

This document details the command-line flags and corresponding environment variables used to configure the ColdVox application at runtime.

## General Flags

These flags control the core behavior of the application.

| Flag | Environment Variable | Description | Default |
| --- | --- | --- | --- |
| `-D, --device <DEVICE>` | `COLDVOX_DEVICE` | Preferred input device name (exact or substring). | `None` |
| `--list-devices` | | List available input devices and exit. | `false` |
| `--resampler-quality <QUALITY>` | `COLDVOX_RESAMPLER_QUALITY` | Resampler quality. Can be `fast`, `balanced`, or `quality`. | `balanced` |
| `--enable-device-monitor` | `COLDVOX_ENABLE_DEVICE_MONITOR` | Enable background device monitoring / hotplug polling. | `false` |
| `--activation-mode <MODE>` | `COLDVOX_ACTIVATION_MODE` | Activation mode. Can be `vad` or `hotkey`. | `vad` |
| `--save-transcriptions` | | Enable transcription persistence to disk. (Requires `vosk` feature) | `false` |
| `--save-audio` | | Save audio alongside transcriptions. (Requires `save-transcriptions`) | `false` |
| `--output-dir <DIR>` | | Output directory for transcriptions. (Requires `vosk` feature) | `transcriptions` |
| `--transcript-format <FORMAT>` | | Transcription format. Can be `json`, `csv`, or `text`. (Requires `vosk` feature) | `json` |
| `--retention-days <DAYS>` | | Keep transcription files for N days (0 = forever). (Requires `vosk` feature) | `30` |
| `--tui` | | Enable TUI dashboard. | `false` |

## Speech-to-Text (STT) Flags

These flags configure the Speech-to-Text engine under the [stt] section.

| Flag | Environment Variable | Description | Default |
| --- | --- | --- | --- |
| `--stt-preferred <PLUGIN>` | `COLDVOX_STT__PREFERRED` | Preferred STT plugin ID (e.g., "vosk", "whisper", "mock"). | `None` |
| `--stt-fallbacks <PLUGINS>` | `COLDVOX_STT__FALLBACKS` | Comma-separated list of fallback plugin IDs. | `[]` |
| `--stt-require-local` | `COLDVOX_STT__REQUIRE_LOCAL` | Require local processing (no cloud STT services). | `false` |
| `--stt-max-mem-mb <MB>` | `COLDVOX_STT__MAX_MEM_MB` | Maximum memory usage in MB. | `None` |
| `--stt-language <LANG>` | `COLDVOX_STT__LANGUAGE` | Required language (ISO 639-1 code, e.g., "en", "fr"). | `None` |
| `--stt-failover-threshold <NUM>` | `COLDVOX_STT__FAILOVER_THRESHOLD` | Number of consecutive errors before switching to fallback plugin. | `3` |
| `--stt-failover-cooldown-secs <SECS>` | `COLDVOX_STT__FAILOVER_COOLDOWN_SECS` | Cooldown period in seconds before retrying a failed plugin. | `30` |
| `--stt-model-ttl-secs <SECS>` | `COLDVOX_STT__MODEL_TTL_SECS` | Time to live in seconds for inactive models (GC threshold). | `300` |
| `--stt-disable-gc` | `COLDVOX_STT__DISABLE_GC` | Disable garbage collection of inactive models. | `false` |
| `--stt-metrics-log-interval-secs <SECS>` | `COLDVOX_STT__METRICS_LOG_INTERVAL_SECS` | Interval in seconds for periodic metrics logging (0 to disable). | `60` |
| `--stt-debug-dump-events` | `COLDVOX_STT__DEBUG_DUMP_EVENTS` | Enable debug dumping of transcription events to logs. | `false` |
| `--stt-auto-extract` | `COLDVOX_STT__AUTO_EXTRACT` | Automatically extract model from a zip archive if not found. | `true` |

## Text Injection Flags

These flags control the text injection behavior under the [injection] section. (Requires `text-injection` feature)

| Flag | Environment Variable | Description | Default |
| --- | --- | --- | --- |
| `--injection-fail-fast` | `COLDVOX_INJECTION__FAIL_FAST` | Exit immediately if all injection methods fail. | `false` |
| `--injection-allow-kdotool` | `COLDVOX_INJECTION__ALLOW_KDOTOOL` | Enable kdotool fallback (KDE/X11). | `false` |
| `--injection-allow-enigo` | `COLDVOX_INJECTION__ALLOW_ENIGO` | Enable enigo fallback (input simulation). | `false` |
| `--injection-inject-on-unknown-focus` | `COLDVOX_INJECTION__INJECT_ON_UNKNOWN_FOCUS` | Allow injection when focus is unknown. | `true` |
| `--injection-require-focus` | `COLDVOX_INJECTION__REQUIRE_FOCUS` | Require editable focus for injection. | `false` |
| `--injection-pause-hotkey <KEY>` | `COLDVOX_INJECTION__PAUSE_HOTKEY` | Hotkey to pause/resume injection (e.g., "Ctrl+Alt+P"). | `""` |
| `--injection-redact-logs` | `COLDVOX_INJECTION__REDACT_LOGS` | Redact text in logs for privacy. | `true` |
| `--injection-max-total-latency-ms <MS>` | `COLDVOX_INJECTION__MAX_TOTAL_LATENCY_MS` | Max latency for a single injection call (ms). | `800` |
| `--injection-per-method-timeout-ms <MS>` | `COLDVOX_INJECTION__PER_METHOD_TIMEOUT_MS` | Timeout for each method attempt (ms). | `250` |
| `--injection-paste-action-timeout-ms <MS>` | `COLDVOX_INJECTION__PASTE_ACTION_TIMEOUT_MS` | Timeout for paste actions (ms). | `200` |
| `--injection-cooldown-initial-ms <MS>` | `COLDVOX_INJECTION__COOLDOWN_INITIAL_MS` | Initial cooldown after failure (ms). | `10000` |
| `--injection-cooldown-backoff-factor <FACTOR>` | `COLDVOX_INJECTION__COOLDOWN_BACKOFF_FACTOR` | Exponential backoff factor. | `2.0` |
| `--injection-cooldown-max-ms <MS>` | `COLDVOX_INJECTION__COOLDOWN_MAX_MS` | Max cooldown period (ms). | `300000` |
| `--injection-injection-mode <MODE>` | `COLDVOX_INJECTION__INJECTION_MODE` | "keystroke", "paste", or "auto". | `"auto"` |
| `--injection-keystroke-rate-cps <CPS>` | `COLDVOX_INJECTION__KEYSTROKE_RATE_CPS` | Keystroke rate (chars/sec). | `20` |
| `--injection-max-burst-chars <CHARS>` | `COLDVOX_INJECTION__MAX_BURST_CHARS` | Max chars per burst. | `50` |
| `--injection-paste-chunk-chars <CHARS>` | `COLDVOX_INJECTION__PASTE_CHUNK_CHARS` | Chunk size for paste ops. | `500` |
| `--injection-chunk-delay-ms <MS>` | `COLDVOX_INJECTION__CHUNK_DELAY_MS` | Delay between paste chunks (ms). | `30` |
| `--injection-focus-cache-duration-ms <MS>` | `COLDVOX_INJECTION__FOCUS_CACHE_DURATION_MS` | Cache duration for focus status (ms). | `200` |
| `--injection-enable-window-detection` | `COLDVOX_INJECTION__ENABLE_WINDOW_DETECTION` | Enable window manager integration. | `true` |
| `--injection-clipboard-restore-delay-ms <MS>` | `COLDVOX_INJECTION__CLIPBOARD_RESTORE_DELAY_MS` | Delay before restoring clipboard (ms). | `500` |
| `--injection-discovery-timeout-ms <MS>` | `COLDVOX_INJECTION__DISCOVERY_TIMEOUT_MS` | Timeout for window discovery (ms). | `1000` |
| `--injection-allowlist <PATTERNS>` | `COLDVOX_INJECTION__ALLOWLIST` | List of allowed app patterns (regex). | `[]` |
| `--injection-blocklist <PATTERNS>` | `COLDVOX_INJECTION__BLOCKLIST` | List of blocked app patterns (regex). | `[]` |
| `--injection-min-success-rate <RATE>` | `COLDVOX_INJECTION__MIN_SUCCESS_RATE` | Minimum success rate before fallback. | `0.3` |
| `--injection-min-sample-size <SIZE>` | `COLDVOX_INJECTION__MIN_SAMPLE_SIZE` | Samples before trusting success rate. | `5` |

Clipboard-based paste fallback is now always enabled when the `text-injection` feature is built; it automatically tries AT-SPI and transparently falls back to `ydotool` when the daemon is available. Clipboard contents are restored automatically after paste.

All configuration values can also be set in `config/default.toml` and overridden by the corresponding environment variables using the `COLDVOX_` prefix and `__` separator for nested sections.

## Deployment Usage of Environment Variables

Environment variables are crucial for deployment scenarios, allowing secure overrides without modifying committed configs. Use them to adapt settings for staging, production, or CI environments.

### Key Principles
- **Precedence**: CLI flags > Env vars > `config/default.toml` > defaults.
- **Security**: Store secrets (e.g., API keys) in env vars or secret managers; never in TOML.
- **Nesting**: Use `__` for sections, e.g., `COLDVOX_STT__PREFERRED=vosk` overrides `[stt].preferred`.

### Examples in Deployment Contexts
- **CI/CD (GitHub Actions)**: Set in workflow `.env` or job steps:
  ```yaml
  env:
    COLDVOX_STT__PREFERRED: vosk  # Use local model in CI
    COLDVOX_INJECTION__FAIL_FAST: true  # Fail fast in tests
  ```
  Validate in workflows: See [docs/self-hosted-runner-complete-setup.md](docs/self-hosted-runner-complete-setup.md) for integration.

- **Systemd Service** (e.g., `/etc/systemd/system/coldvox.service`):
  ```
  [Service]
  ExecStart=/opt/coldvox/coldvox-app
  Environment="COLDVOX_DEVICE=default"
  Environment="COLDVOX_STT__LANGUAGE=en"
  EnvironmentFile=/etc/coldvox/prod.env  # For bulk secrets
  ```

- **Docker/Kubernetes**:
  - Docker: `docker run -e COLDVOX_INJECTION__INJECTION_MODE=paste -e COLDVOX_STT__MAX_MEM_MB=2048 coldvox-image`
  - K8s: Use ConfigMap for non-secrets, Secret for sensitive vars like `COLDVOX_STT__API_KEY`.

- **Rollback**: If env overrides cause issues, unset vars and restart; fallback to `default.toml`.

For comprehensive deployment steps, including validation and rollback with these vars, refer to [docs/deployment.md](docs/deployment.md). Additional VAD flags (e.g., `COLDVOX_VAD__SENSITIVITY`) follow the same pattern; extend as needed in code.
