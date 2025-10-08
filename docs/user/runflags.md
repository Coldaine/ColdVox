# ColdVox Configuration & Runtime Controls

The current application relies on a layered configuration system:

1. **`config/default.toml`** – committed defaults.
2. **Environment variables** – override any key using the `COLDVOX_` prefix and `__` for nested tables.
3. **Command-line flags** – limited to a few operational toggles.

## Command-line Flags

| Flag | Description | Notes |
| --- | --- | --- |
| `--list-devices` | Enumerate input devices via CPAL and exit. | Useful for discovering the value to set via `COLDVOX_DEVICE`. |
| `--tui` | Launch the TUI dashboard on top of the shared runtime. | Keyboard shortcuts: `S` start, `A` toggle VAD/hotkey, `R` reset, `Q` quit. |
| `--injection-fail-fast` | Force the process to exit if all injection methods fail. | Mirrors `injection.fail_fast` in the config file. |

Other behaviour is controlled through configuration or environment variables.

## `config/default.toml`

### Top-level keys

| Key | Default | Purpose |
| --- | --- | --- |
| `resampler_quality` | `"balanced"` | Selects resampler quality (`fast`, `balanced`, `quality`). |
| `activation_mode` | `"vad"` | Chooses between VAD-driven (`vad`) and hotkey (`hotkey`) activation. |
| `enable_device_monitor` | `true` | Enables ALSA/PipeWire hotplug polling. |
| `device` | *(unset)* | Optional preferred input device; fallbacks are used when omitted. |

### `[injection]`

| Key | Default | Description |
| --- | --- | --- |
| `fail_fast` | `false` | Abort if all injection methods fail (same effect as CLI flag). |
| `allow_kdotool` / `allow_enigo` | `false` | Opt-in backends for KDE/X11 and cross-platform input. |
| `inject_on_unknown_focus` | `true` | Permit injection when focus cannot be determined. |
| `require_focus` | `false` | Require editable focus before injecting. |
| `max_total_latency_ms` | `800` | Total latency budget per injection attempt. |
| `per_method_timeout_ms` | `250` | Timeout per backend attempt. |
| `clipboard_restore_delay_ms` | `500` | Delay before restoring clipboard contents after paste. |
| `cooldown_*` settings | `10000`, `2.0`, `300000` | Failure cooldown initial value, backoff factor, and max cap. |
| `allowlist` / `blocklist` | `[]` | Regex (when compiled with `regex`) or substring filters for app IDs. |

### `[stt]`

| Key | Default | Description |
| --- | --- | --- |
| `preferred` | `null` | Preferred STT plugin ID (e.g., `"vosk"`). |
| `fallbacks` | `[]` | Ordered fallback plugin IDs. |
| `require_local` | `false` | Reject remote/cloud STT providers. |
| `max_mem_mb` | `null` | Optional memory ceiling for STT plugins. |
| `failover_threshold` / `failover_cooldown_secs` | `5` / `10` | Consecutive error threshold and cooldown before failover. |
| `model_ttl_secs` | `300` | Idle model lifetime before GC. |
| `disable_gc` | `false` | Disable model garbage collection. |
| `metrics_log_interval_secs` | `30` | Periodic STT metrics logging interval (0 disables). |
| `debug_dump_events` | `false` | Emit verbose transcription events to logs. |
| `auto_extract` | `true` | Allow automatic extraction of Vosk models from zip archives. |

## Environment Overrides

Use the `COLDVOX_` prefix, replacing dots with double underscores:

- `COLDVOX_DEVICE="USB Microphone"` overrides the capture device.
- `COLDVOX_INJECTION__FAIL_FAST=true` mirrors the CLI flag.
- `COLDVOX_STT__PREFERRED=vosk` selects Vosk as the primary STT plugin.

Boolean overrides accept common truthy values (`true`, `1`, `yes`). List values (e.g., `fallbacks`) can be provided as comma-separated strings: `COLDVOX_STT__FALLBACKS=vosk,mock`.

## Examples

```bash
# Run with a specific microphone and fail-fast injection policy
COLDVOX_DEVICE="HyperX QuadCast" \
COLDVOX_INJECTION__FAIL_FAST=true \
cargo run

# Switch STT preference for a single TUI session
COLDVOX_STT__PREFERRED=mock cargo run --bin tui_dashboard
```

For long-lived overrides, create an `overrides.toml` file (not loaded by default) or use your service manager’s environment configuration. Ensure sensitive values remain out of version control.
