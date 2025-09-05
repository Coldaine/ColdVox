# ColdVox GUI

This crate now includes groundwork for a Qt 6 + QML UI using CXX-Qt, gated behind a feature so default builds remain stub-only. Enable the `qt-ui` feature to link Qt and verify the setup.

## Current Status

By default this remains a stub implementation and does not link Qt. With `--features qt-ui`, the binary constructs a minimal `QGuiApplication` to validate Qt linkage, then exits.

## Goals

The ColdVox GUI will provide:

- **Real-time Transcription Display**: Live view of speech-to-text output with confidence indicators
- **Audio Input Configuration**: Device selection, sample rate settings, and input level monitoring
- **VAD Settings and Visualization**: Voice activity detection configuration with visual feedback
- **System Status and Metrics**: Performance monitoring, error reporting, and health checks
- **Text Injection Configuration**: Setup and testing of various text input methods
- **Accessibility Features**: High contrast modes, keyboard navigation, screen reader support

## GUI Toolkit Evaluation Criteria

The GUI framework selection will be based on:

### Technical Requirements
- **Cross-platform**: Linux (primary), Windows, macOS support
- **Performance**: Low latency for real-time audio visualization
- **Accessibility**: Screen reader compatibility, keyboard navigation
- **Rust Integration**: Native Rust support with good ecosystem integration
- **Packaging**: Easy distribution and deployment

### User Experience Requirements
- **Responsiveness**: Non-blocking UI during audio processing
- **Configurability**: Extensive customization options
- **Visual Feedback**: Clear indicators for system state and activity
- **Error Handling**: User-friendly error messages and recovery options

### Development Requirements
- **Documentation**: Good documentation and community support
- **Maintenance**: Active development and long-term viability
- **Learning Curve**: Reasonable complexity for the development team
- **Testing**: Good testing framework support

## Candidate GUI Toolkits

### egui
- **Pros**: Immediate mode, pure Rust, good performance, active development
- **Cons**: Younger ecosystem, limited widget set compared to mature toolkits
- **Use Case**: Good for rapid prototyping and Rust-first applications

### Tauri
- **Pros**: Web technologies (HTML/CSS/JS), cross-platform, good documentation
- **Cons**: Larger bundle size, potential web security concerns
- **Use Case**: Teams familiar with web development, complex layouts

### GTK4 (via gtk4-rs)
- **Pros**: Mature, excellent accessibility, native platform integration
- **Cons**: Large dependency tree, platform-specific quirks
- **Use Case**: Linux-first applications requiring deep platform integration

### Slint
- **Pros**: Rust-native, declarative UI, good performance, modern design
- **Cons**: Commercial licensing for some use cases, smaller community
- **Use Case**: Applications requiring custom styling and animations

### Iced
- **Pros**: Pure Rust, Elm-inspired architecture, good for reactive UIs
- **Cons**: Smaller widget ecosystem, less mature than alternatives
- **Use Case**: Applications with complex state management needs

## Development Phases

1. **Phase 1 (Current)**: Placeholder crate and requirements analysis
2. **Phase 2**: GUI toolkit selection and proof-of-concept
3. **Phase 3**: Basic transcription display and audio configuration
4. **Phase 4**: Advanced features (metrics, visualization, accessibility)
5. **Phase 5**: Polish, testing, and documentation

## Usage

Default (no Qt linkage):

```bash
cargo run -p coldvox-gui
```

With Qt + CXX-Qt enabled (requires Qt 6 dev packages):

```bash
cargo run -p coldvox-gui --features qt-ui
```

For actual ColdVox functionality, use the TUI dashboard:

```bash
cargo run -p coldvox-app --bin tui_dashboard
```

## Prerequisites for `qt-ui`

- Install Qt 6 development packages (Core, Gui, Qml, Quick):
  - Ubuntu/Debian: `sudo apt-get install qt6-base-dev qt6-declarative-dev qml-qt6`
  - macOS (Homebrew): `brew install qt@6`
  - Windows: Install Qt 6 via the Qt online installer and ensure `qmake` is on PATH.

## Contributing

GUI framework selection and implementation will be tracked in the main project issues. Input on toolkit selection is welcome, especially from users with accessibility requirements or cross-platform deployment experience.
