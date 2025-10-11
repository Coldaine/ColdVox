# Injection Policy Cheatsheet

ColdVox merges configuration from three locations before each run:

1. `config/default.toml` – project defaults, checked into git
2. Environment overrides using the `COLDVOX_INJECTION__*` prefix (double underscore becomes a dot)
3. CLI flags (`--injection-fail-fast`, `--tui`, `--list-devices`) for behaviour that cannot safely live in config files

Later sources always win. For example the environment variable
`COLDVOX_INJECTION__INJECT_ON_UNKNOWN_FOCUS=false` overrides the default, and a CLI flag would override both.

## Allow & block lists

`InjectionConfig` includes `allowlist` and `blocklist` vectors that accept regular expressions. Entries are matched against the
window class reported by the window manager helper.

```toml
# ~/.config/coldvox/injection.toml
allowlist = ["^(code|sublime_text)$", "jetbrains-.*"]
blocklist = ["^(org\.gnome\.Terminal|gnome-shell)$", "^com\.discordapp\.Discord$"]
```

* If the allow list is non-empty, only applications matching at least one pattern are eligible.
* The block list always wins when a window matches both lists.
* Patterns are compiled lazily; invalid expressions are logged and ignored rather than crashing the runtime.

## Recommended defaults

| Setting | Why it changed | New guidance |
| --- | --- | --- |
| `max_total_latency_ms = 600` | Keeps clipboard fallbacks snappy while leaving room for AT-SPI retries. | Increase only when targeting high-latency remoting setups. |
| `paste_action_timeout_ms = 350` | Matches the average compositor paste acknowledgement on GNOME/KDE. | Raise when remote desktops routinely exceed 0.3 s. |
| `focus_cache_duration_ms = 200` | Ensures the focus tracker does not flood the accessibility bus. | Lower to 100 ms for rapid-fire UI automation. |

## Troubleshooting playbook

| Symptom | Quick check | Mitigation |
| --- | --- | --- |
| Clipboard never restores | Ensure `clipboard_restore_delay_ms` is >= 100 ms and wl-clipboard is installed. | Increase delay or disable clipboard method in policy for sensitive apps. |
| Focus reported as `Unknown` | Focus detection currently defaults to `Unknown` for safety. | Keep `inject_on_unknown_focus=false` for sensitive apps; rely on clipboard-only path otherwise. |
| ydotool not available | Run `ydotoold` and confirm `/run/user/$UID/.ydotool_socket` exists. | Install the daemon or remove the backend from `allow_methods`. |

Store team-specific policies under `config/policies/` and document their intent so that CI and local profiles share the same
expectations.</content>
<parameter name="filePath">/home/coldaine/Desktop/ColdVoxRefactorTwo/ColdVox/docs/user/injection-policies.md