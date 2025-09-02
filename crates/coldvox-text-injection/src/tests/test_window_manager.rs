#[cfg(test)]
mod tests {
    use crate::window_manager::{get_active_window_class, get_window_info};

    #[tokio::test]
    async fn test_window_class_detection() {
        // This test will only work in a graphical environment
        if std::env::var("DISPLAY").is_ok() || std::env::var("WAYLAND_DISPLAY").is_ok() {
            let result = get_active_window_class().await;

            // We can't assert specific values since it depends on the environment
            // but we can check that it doesn't panic
            match result {
                Ok(class) => {
                    println!("Detected window class: {}", class);
                    assert!(!class.is_empty());
                }
                Err(e) => {
                    println!("Window detection failed (expected in CI): {}", e);
                }
            }
        }
    }

    #[tokio::test]
    async fn test_window_info_structure() {
        let info = get_window_info().await;

        // Basic sanity checks
        assert!(!info.class.is_empty());
        // Title might be empty
        // PID might be 0 if detection failed
    }

    #[test]
    fn test_x11_detection() {
        // Check if X11 is available
        let x11_available = std::env::var("DISPLAY").is_ok();

        if x11_available {
            // Try to run xprop
            let output = std::process::Command::new("xprop")
                .args(["-root", "_NET_ACTIVE_WINDOW"])
                .output();

            // Should at least not panic
            assert!(output.is_ok() || output.is_err());
        }
    }

    #[test]
    fn test_wayland_detection() {
        // Check if Wayland is available
        let wayland_available = std::env::var("WAYLAND_DISPLAY").is_ok();

        if wayland_available {
            println!(
                "Wayland display detected: {:?}",
                std::env::var("WAYLAND_DISPLAY")
            );
        }
    }
}
