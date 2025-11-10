# Testing Text Injection Backends

This document outlines the process for testing the text injection backends for the `coldvox-text-injection` crate.

## Prerequisites

To run these tests, you will need a graphical environment (X11 or Wayland) and the development packages for GTK3. The required packages for some common distributions are listed below:

**Debian/Ubuntu:**
```bash
sudo apt-get install libgtk-3-dev
```

**Fedora:**
```bash
sudo dnf install gtk3-devel
```

**Arch Linux:**
```bash
sudo pacman -S gtk3
```

You will also need to have the `at-spi2-core` package installed, which is usually included by default in most desktop environments.

## Running the Tests

A test runner script, `run_tests.sh`, is provided to simplify the process of running the tests. This script must be run from the root of the repository.

To run all available backend tests, simply execute the script without any arguments:
```bash
./crates/coldvox-text-injection/run_tests.sh
```

You can also run tests for a specific backend by passing the backend name as an argument to the script. The available backends are:

- `atspi`
- `ydotool`
- `kdotool`
- `clipboard`
- `enigo`

For example, to run only the AT-SPI backend tests, you would use the following command:
```bash
./crates/coldvox-text-injection/run_tests.sh atspi
```

## Test Matrix

The following matrix can be used to track the test results across different platforms and backends. Please fill in the results as you test each combination.

| Backend | GNOME/Wayland | GNOME/X11 | KDE/Wayland | KDE/X11 | Sway | i3 |
|---|---|---|---|---|---|---|
| AT-SPI | ? | ? | ? | ? | ? | ? |
| Clipboard | ? | ? | ? | ? | ? | ? |
| ydotool | ? | N/A | ? | N/A | ? | N/A |
| kdotool | N/A | ? | N/A | ? | N/A | ? |
| Combo | ? | ? | ? | ? | ? | ? |

## Known Issues

*   (Please add any known issues here)

## Platform-Specific Setup Notes

*   **(GNOME/Wayland)** (Please add any platform-specific setup notes here)
*   **(GNOME/X11)** (Please add any platform-specific setup notes here)
*   **(KDE/Wayland)** (Please add any platform-specific setup notes here)
*   **(KDE/X11)** (Please add any platform-specific setup notes here)
*   **(Sway)** (Please add any platform-specific setup notes here)
*   **(i3)** (Please add any platform-specific setup notes here)

## Recommended Backends

*   **(GNOME/Wayland)** (Please add your recommended backend for this platform here)
*   **(GNOME/X11)** (Please add your recommended backend for this platform here)
*   **(KDE/Wayland)** (Please add your recommended backend for this platform here)
*   **(KDE/X11)** (Please add your recommended backend for this platform here)
*   **(Sway)** (Please add your recommended backend for this platform here)
*   **(i3)** (Please add your recommended backend for this platform here)
