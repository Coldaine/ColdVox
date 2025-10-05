# ColdVox Runtime Flags

This document details the command-line flags and corresponding environment variables used to configure the ColdVox application at runtime.

## General Flags

These flags control the core behavior of the application.

| Flag | Environment Variable | Description | Default |
|---|---|---|---|
| `-D, --device <DEVICE>` | `COLDVOX_DEVICE` | Preferred input device name (exact or substring). | `None` |
| `--list-devices` | | List available input devices and exit. | `false` |
| `--resampler-quality <QUALITY>` | `COLDVOX_RESAMPLER_QUALITY` | Resampler quality. Can be `fast`, `balanced`, or `quality`. | `balanced` |
| `--save-transcriptions` | | Enable transcription persistence to disk. (Requires `vosk` feature) | `false` |
| `--save-audio` | | Save audio alongside transcriptions. (Requires `save-transcriptions`) | `false` |
| `--output-dir <DIR>` | | Output directory for transcriptions. (Requires `vosk` feature) | `transcriptions` |
| `--transcript-format <FORMAT>` | | Transcription format. Can be `json`, `csv`, or `text`. (Requires `vosk` feature) | `json` |
| `--retention-days <DAYS>` | | Keep transcription files for N days (0 = forever). (Requires `vosk` feature) | `30` |
| `--tui` | | Enable TUI dashboard. | `false` |
| `--enable-device-monitor` | `COLDVOX_ENABLE_DEVICE_MONITOR` | Enable background device monitoring / hotplug polling. | `false` |
| `--activation-mode <MODE>` | | Activation mode. Can be `vad` or `hotkey`. | `vad` |

## Speech-to-Text (STT) Flags

These flags configure the Speech-to-Text engine.

| Flag | Environment Variable | Description | Default |
|---|---|---|---|
| `--stt-preferred <PLUGIN>` | `COLDVOX_STT_PREFERRED` | Preferred STT plugin ID (e.g., "vosk", "whisper", "mock"). | `vosk` |
| `--stt-fallbacks <PLUGINS>` | `COLDVOX_STT_FALLBACKS` | Comma-separated list of fallback plugin IDs. | `vosk,mock` |
| `--stt-require-local` | `COLDVOX_STT_REQUIRE_LOCAL` | Require local processing (no cloud STT services). | `false` |
| `--stt-max-mem-mb <MB>` | `COLDVOX_STT_MAX_MEM_MB` | Maximum memory usage in MB. | `None` |
| `--stt-language <LANG>` | `COLDVOX_STT_LANGUAGE` | Required language (ISO 639-1 code, e.g., "en", "fr"). | `None` |
| `--stt-failover-threshold <NUM>` | `COLDVOX_STT_FAILOVER_THRESHOLD` | Number of consecutive errors before switching to fallback plugin. | `3` |
| `--stt-failover-cooldown-secs <SECS>` | `COLDVOX_STT_FAILOVER_COOLDOWN_SECS` | Cooldown period in seconds before retrying a failed plugin. | `30` |
| `--stt-model-ttl-secs <SECS>` | `COLDVOX_STT_MODEL_TTL_SECS` | Time to live in seconds for inactive models (GC threshold). | `300` |
| `--stt-disable-gc` | `COLDVOX_STT_DISABLE_GC` | Disable garbage collection of inactive models. | `false` |
| `--stt-metrics-log-interval-secs <SECS>` | `COLDVOX_STT_METRICS_LOG_INTERVAL_SECS` | Interval in seconds for periodic metrics logging (0 to disable). | `60` |
| `--stt-debug-dump-events` | `COLDVOX_STT_DEBUG_DUMP_EVENTS` | Enable debug dumping of transcription events to logs. | `false` |
| `--stt-auto-extract` | `COLDVOX_STT_AUTO_EXTRACT` | Automatically extract model from a zip archive if not found. | `true` |

## Text Injection Flags

These flags control the text injection behavior. (Requires `text-injection` feature)

| Flag | Environment Variable | Description | Default |
|---|---|---|---|
| `--enable-text-injection` | `COLDVOX_ENABLE_TEXT_INJECTION` | Enable text injection after transcription. | `true` |
| `--allow-kdotool` | `COLDVOX_ALLOW_KDOTOOL` | Allow kdotool as an injection fallback. | `false` |
| `--allow-enigo` | `COLDVOX_ALLOW_ENIGO` | Allow enigo as an injection fallback. | `false` |
| `--inject-on-unknown-focus` | `COLDVOX_INJECT_ON_UNKNOWN_FOCUS` | Attempt injection even if the focused application is unknown. | `true` |
| `--restore-clipboard` | `COLDVOX_RESTORE_CLIPBOARD` | Restore clipboard contents after injection. | `false` |
| `--max-total-latency-ms <MS>` | `COLDVOX_INJECTION_MAX_LATENCY_MS` | Max total latency for an injection call (ms). | `None` |
| `--per-method-timeout-ms <MS>` | `COLDVOX_INJECTION_METHOD_TIMEOUT_MS` | Timeout for each injection method (ms). | `None` |
| `--cooldown-initial-ms <MS>` | `COLDVOX_INJECTION_COOLDOWN_MS` | Initial cooldown on failure (ms). | `None` |

Clipboard-based paste fallback is now always enabled when the `text-injection` feature is built; it automatically tries AT-SPI and transparently falls back to `ydotool` when the daemon is available.
