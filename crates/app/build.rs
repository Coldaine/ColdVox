use std::env;

fn main() {
    // Declare custom cfg names for the compiler
    println!("cargo::rustc-check-cfg=cfg(kde_globalaccel)");

    println!("cargo::rustc-check-cfg=cfg(wayland_session)");
    println!("cargo::rustc-check-cfg=cfg(x11_session)");

    // Detect Linux desktop environment at compile time
    if cfg!(target_os = "linux") {
        // Enable Linux-specific text injection features

        // Check for KDE Plasma and enable KGlobalAccel backend
        if env::var("KDE_FULL_SESSION").is_ok()
            || env::var("PLASMA_SESSION").is_ok()
            || env::var("XDG_CURRENT_DESKTOP")
                .map(|s| s.to_lowercase().contains("kde") || s.to_lowercase().contains("plasma"))
                .unwrap_or(false)
        {
            println!("cargo:rustc-cfg=kde_globalaccel");
        }

        // Check for Wayland
        if env::var("WAYLAND_DISPLAY").is_ok()
            || env::var("XDG_SESSION_TYPE")
                .map(|s| s == "wayland")
                .unwrap_or(false)
        {
            println!("cargo:rustc-cfg=wayland_session");
        }

        // Check for X11
        if env::var("DISPLAY").is_ok()
            || env::var("XDG_SESSION_TYPE")
                .map(|s| s == "x11")
                .unwrap_or(false)
        {
            println!("cargo:rustc-cfg=x11_session");
        }

        // If neither detected, enable all Linux backends
        if env::var("WAYLAND_DISPLAY").is_err()
            && env::var("DISPLAY").is_err()
            && env::var("XDG_SESSION_TYPE").is_err()
        {
            // Build environment might not have display vars, enable all
        }

        // Always enable these on Linux
    }

    // Windows - 2025-09-04: Currently not targeting Windows builds

    // macOS
    if cfg!(target_os = "macos") {}
}
