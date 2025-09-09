# ColdVox GUI – Additional Feature Suggestions

This document proposes enhancements to evolve the prototype GUI into a fully featured speech‑to‑text assistant. Items are grouped by theme and include brief rationale.

Note: The current prototype scope is Linux only, targeting the Nobara distribution on KDE Plasma. Cross‑platform items are listed for future planning but are not in scope right now.

## Core UX

- Global hotkey: System‑wide activation (Ctrl+Shift+Space by default) via platform backends (X11/Wayland/KGlobalAccel, macOS EventTap, Windows hooks).
- Smart state machine: Smooth transitions across Idle → Recording → Processing → Complete with resilient error handling and retries.
- Position memory and multi‑monitor: Persist per‑screen coordinates and restore onto nearest available screen if missing.
- Click‑through mode: Optional “pass‑through when unfocused” for overlay while preserving always‑on‑top visibility.

## Transcription & Display

- Rich formatting: Headings, bullets, code blocks (Markdown → styled text). Toggle in Settings.
- Inline corrections: Visual diffing for partial hypotheses → final text with crossfade animations.
- Word timing: Highlight current word and display timestamps; optional karaoke‑style cursor.
- Confidence cues: Subtle underlines/opacity scaling by confidence; tooltips with numeric values.
- Autoscroll controls: Lock/hold toggle, jump to end, and “new text” indicator when paused.

## Audio & VAD

- Input device picker: Enumerate devices and remember a preferred default per OS profile.
- Live meters: Peak/RMS meters with overload indicator; calibration assistant.
- VAD tuning: Thresholds, hangover, and noise gating controls with live preview.
- Noise suppression: Optional denoiser (RNNoise/NSNet) with CPU/GPU preference.

## Hotkeys & Automation

- Context hotkeys: Start/stop/pause/clear shortcuts; toggle collapsed/expanded; copy latest snippet.
- Snippet actions: Copy to clipboard, paste into active app, or save to file with one keystroke.
- Phrase triggers: Map key phrases to actions (e.g., “new line”, “send”, “undo”).
- Profiles: Per‑app profiles with different hotkeys and output options.

## Text Injection

- Backend selection: Switch among native injection methods per platform; automatic fallback.
- Rich text output: Preserve formatting where supported (e.g., paste RTF/HTML when available).
- Rate limiting: Prevent flood on sensitive inputs; visual indicators during injection.

## Settings & Personalization

- Theme: Light/Dark/Auto, accent color, and transparency slider with live preview.
- Languages: Model/language selection and auto‑detect fallback with confidence reporting.
- Auto punctuation & casing: Toggle plus per‑language rules and custom dictionaries.
- Privacy: On‑device only mode, ephemeral buffers, configurable retention windows.
- Telemetry: Opt‑in metrics, diagnostics, and quick bug‑report bundler.

## Integrations

- Model backends: Vosk, Whisper, Parakeet, and remote providers; hot‑swap at runtime.
- Cloud API keys: Provider vault with validation and safe storage (OS keychain where possible).
- Export: Send to notes, email, or project tools (Markdown export, Obsidian vault, Joplin, etc.).

## Accessibility

- Keyboard‑only flow: Full navigation without a mouse; focus rings and shortcuts.
- Screen reader: Proper roles/labels for controls and live region for transcript updates.
- High contrast: WCAG‑compliant palette; user‑settable font size and line height.

## Developer & Ops

- Profiles as code: Export/import settings as TOML/JSON; versioned profiles per machine.
- Plugin API: Load STT/VAD/injection plugins dynamically; capability negotiation.
- Test harness: UI smoke tests and golden screenshots for animations and layout.

## Polish & Effects

- Acrylic/blur: Native per‑platform blur (Windows Acrylic, macOS Vibrancy) with fallbacks.
- Micro‑interactions: Hovers, press states, and elastic transitions tuned for 60/120Hz.
- Adaptive layout: DPI‑aware, font scaling, and compact/comfortable density modes.

These enhancements can be tracked as milestones to iteratively raise functionality and fit the project’s cross‑platform goals without sacrificing responsiveness or accessibility.
