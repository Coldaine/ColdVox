use std::env;

fn main() {
    // Detect Linux desktop environment at compile time
    if cfg!(target_os = "linux") {
        // Enable Linux-specific text injection features
        println!("cargo:rustc-cfg=text_injection_linux");

        // Check for Wayland
        if env::var("WAYLAND_DISPLAY").is_ok()
            || env::var("XDG_SESSION_TYPE")
                .map(|s| s == "wayland")
                .unwrap_or(false)
        {
            println!("cargo:rustc-cfg=wayland_session");
            println!("cargo:rustc-cfg=text_injection_atspi");
            println!("cargo:rustc-cfg=text_injection_clipboard");
            println!("cargo:rustc-cfg=text_injection_ydotool");
        }

        // Check for X11
        if env::var("DISPLAY").is_ok()
            || env::var("XDG_SESSION_TYPE")
                .map(|s| s == "x11")
                .unwrap_or(false)
        {
            println!("cargo:rustc-cfg=x11_session");
            println!("cargo:rustc-cfg=text_injection_atspi");
            println!("cargo:rustc-cfg=text_injection_clipboard");
            println!("cargo:rustc-cfg=text_injection_kdotool");
        }

        // If neither detected, enable all Linux backends
        if env::var("WAYLAND_DISPLAY").is_err()
            && env::var("DISPLAY").is_err()
            && env::var("XDG_SESSION_TYPE").is_err()
        {
            // Build environment might not have display vars, enable all
            println!("cargo:rustc-cfg=text_injection_atspi");
            println!("cargo:rustc-cfg=text_injection_clipboard");
            println!("cargo:rustc-cfg=text_injection_ydotool");
            println!("cargo:rustc-cfg=text_injection_kdotool");
        }

        // Always enable these on Linux
        println!("cargo:rustc-cfg=text_injection_mki");
        println!("cargo:rustc-cfg=text_injection_enigo");
    }

    // Windows
    if cfg!(target_os = "windows") {
        println!("cargo:rustc-cfg=text_injection_windows");
        println!("cargo:rustc-cfg=text_injection_enigo");
        println!("cargo:rustc-cfg=text_injection_mki");
    }

    // macOS
    if cfg!(target_os = "macos") {
        println!("cargo:rustc-cfg=text_injection_macos");
        println!("cargo:rustc-cfg=text_injection_enigo");
        println!("cargo:rustc-cfg=text_injection_mki");
    }
}
