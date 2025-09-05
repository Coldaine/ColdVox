# Additional GUI Feature Ideas

The current ColdVox GUI implements a minimal transcription interface. The following ideas could further enhance the user experience:

- **Waveform Visualization Improvements**
  - Frequency spectrum or energy heatmap
  - Customizable color themes for activity indicator
- **Transcript Management**
  - Export transcripts to text/Markdown files
  - Search and highlight within transcript
  - Undo/redo support for manual edits
- **Recording Controls**
  - Push-to-talk mode with visual feedback
  - Configurable silence auto-stop thresholds
  - Audio level meters with calibration
- **Settings Enhancements**
  - Profiles for different languages or microphones
  - Cloud sync for preferences and API keys
  - Detailed hotkey editor with per-action mapping
- **Accessibility & Localization**
  - High‑contrast and large-font modes
  - Right-to-left language support
  - Full screen reader navigation
- **Integration & Automation**
  - Output to clipboard or selected application in real time
  - Webhook or scripting hooks after transcription completes
  - Plugin system for custom post-processing
- **Testing & Diagnostics**
  - Built-in network latency and CPU usage monitors
  - Diagnostic logs viewer with export options

These features could be explored in future development phases as the GUI matures.

## Platform Notes: Fedora (Wayland + KDE Plasma)
- Ensure `WAYLAND_DISPLAY` is set and XWayland compatibility packages are installed for legacy apps.
- Test global hotkeys with KDE's shortcuts system; some combinations may be reserved.
- Consider packaging via Flatpak or RPM to align with Fedora distribution practices.

## Framework Choice: Tauri vs. eframe
The prototype uses **eframe** for simplicity, but a production release may adopt **Tauri**.

**Why Tauri?**
- Uses system webview, yielding very small binaries and easy theming with web tech.
- Mature packaging and auto-update ecosystem for Linux desktops.

**Pros**
- Lower memory footprint than Electron-style apps.
- Strong security model with Rust backend and isolated frontend.

**Cons**
- Requires web development stack (HTML/CSS/JS) alongside Rust.
- Access to advanced window features (like transparent always-on-top windows) can be trickier than native egui/eframe.

