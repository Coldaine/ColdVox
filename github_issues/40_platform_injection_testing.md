# Platform-Specific Text Injection Backend Testing

## Issue Type
Testing / Quality Assurance

## Priority
Medium-High

## Component
`crates/coldvox-text-injection`

## Description
Text injection backends need comprehensive testing across different Linux desktop environments, window managers, and configurations. Current testing is limited to developer environments and may not cover all edge cases.

## Current State
- Multiple backend implementations (AT-SPI, clipboard, ydotool, kdotool, enigo)
- Platform detection in build.rs
- Limited testing across environments
- Some backends work better in specific contexts

## Platforms to Test

### Desktop Environments
- **GNOME** (Wayland & X11)
  - Latest stable (45.x)
  - Ubuntu 24.04 default
  - Fedora 40 default

- **KDE Plasma** (Wayland & X11)
  - Plasma 6.x
  - Plasma 5.27 LTS
  - With/without KGlobalAccel

- **XFCE** (X11 only)
- **Cinnamon** (X11 primarily)
- **Sway** (Wayland compositor)
- **Hyprland** (Wayland compositor)
- **i3/i3-gaps** (X11 tiling WM)

### Test Matrix

| Backend | GNOME/Wayland | GNOME/X11 | KDE/Wayland | KDE/X11 | Sway | i3 |
|---------|--------------|-----------|-------------|---------|------|-----|
| AT-SPI | ? | ? | ? | ? | ? | ? |
| Clipboard | ? | ? | ? | ? | ? | ? |
| ydotool | ? | N/A | ? | N/A | ? | N/A |
| kdotool | N/A | ? | N/A | ? | N/A | ? |
| Combo | ? | ? | ? | ? | ? | ? |

### Applications to Test
- **Terminal Emulators**
  - gnome-terminal, konsole, alacritty, kitty, wezterm

- **Text Editors**
  - VS Code, Sublime Text, gedit, kate
  - Vim/Neovim (terminal & GUI)
  - Emacs

- **Web Browsers**
  - Firefox, Chrome/Chromium, Brave

- **Chat/Communication**
  - Slack, Discord, Element

- **Office Apps**
  - LibreOffice Writer, OnlyOffice

## Test Scenarios
1. **Basic text injection**
   - Single word
   - Multiple words
   - Special characters
   - Unicode/emoji
   - Large text blocks (>1000 chars)

2. **Performance tests**
   - Injection latency measurement
   - CPU usage during injection
   - Memory usage patterns

3. **Edge cases**
   - Rapid successive injections
   - Application focus changes
   - Screen locked/unlocked
   - Multiple monitors
   - Virtual desktops/workspaces

4. **Failure scenarios**
   - No clipboard manager
   - Disabled accessibility
   - Permission issues
   - Resource constraints

## Setup Requirements
- Document required packages per distro
- Permission configuration (uinput, etc.)
- Environment variables needed
- Accessibility settings

## Automation Opportunities
- Docker/Podman containers for different DEs
- CI matrix builds
- Automated test suite
- Performance regression detection

## Success Metrics
- Backend selection accuracy >95%
- Injection success rate >99%
- Fallback mechanism working
- Performance within targets (<10ms)

## Deliverables
- [ ] Completed test matrix with results
- [ ] Per-platform setup documentation
- [ ] Known issues/limitations list
- [ ] Recommended backends per environment
- [ ] CI testing infrastructure
