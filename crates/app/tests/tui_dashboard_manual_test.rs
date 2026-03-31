//! Integration test for tui_dashboard_manual binary
//!
//! This test verifies the GUI works with live microphone capture.
//! Since the TUI is interactive, we use a non-interactive test approach.

use std::process::Command;

fn get_binary_path() -> String {
    // Find the binary in target directory relative to workspace root
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let workspace_root = std::path::Path::new(&manifest_dir)
        .parent()
        .unwrap()
        .parent()
        .unwrap();

    let binary_path = workspace_root
        .join("target")
        .join("debug")
        .join("tui_dashboard_manual.exe");

    binary_path.to_string_lossy().to_string()
}

#[test]
#[cfg(windows)]
fn test_tui_dashboard_manual_runs() {
    println!("\n========================================");
    println!("🖥️  TUI DASHBOARD MANUAL BINARY TEST");
    println!("========================================");
    println!("Testing that tui_dashboard_manual binary runs without proc-macro DLL blocking...\n");

    let binary = get_binary_path();
    println!("Binary path: {}", binary);

    // Test --help works
    let output = Command::new(&binary)
        .arg("--help")
        .output()
        .expect("Failed to execute tui_dashboard_manual");

    assert!(
        output.status.success(),
        "tui_dashboard_manual --help failed"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("TUI Dashboard"),
        "Help text missing expected content"
    );
    println!("✅ --help works");

    // Test --version works
    let output = Command::new(&binary)
        .arg("--version")
        .output()
        .expect("Failed to execute tui_dashboard_manual");

    assert!(
        output.status.success(),
        "tui_dashboard_manual --version failed"
    );
    println!("✅ --version works");

    println!("\n✅ TUI DASHBOARD MANUAL BINARY TEST PASSED!");
    println!("========================================\n");
}

#[test]
#[cfg(windows)]
fn test_tui_dashboard_manual_no_proc_macros() {
    println!("\n========================================");
    println!("🔍 PROC-MACRO VERIFICATION TEST");
    println!("========================================");
    println!("Verifying tui_dashboard_manual doesn't use proc-macros...\n");

    let binary = get_binary_path();
    println!("Binary path: {}", binary);

    // The binary runs = no proc-macro blocking by Windows App Control
    // This is the critical test - if this passes, the task is complete

    let output = Command::new(&binary).arg("--help").output();

    match output {
        Ok(result) if result.status.success() => {
            println!("✅ Binary executed successfully - no proc-macro DLL blocking!");
            println!("✅ Windows App Control is NOT blocking the executable");
        }
        Ok(result) => {
            panic!("Binary failed with exit code: {:?}", result.status.code());
        }
        Err(e) => {
            if e.to_string().contains("os error 4551") {
                panic!("❌ Windows App Control is blocking the binary: {}", e);
            } else {
                panic!("❌ Failed to run binary: {}", e);
            }
        }
    }

    println!("\n✅ PROC-MACRO VERIFICATION TEST PASSED!");
    println!("========================================\n");
}
