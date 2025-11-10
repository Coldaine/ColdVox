use std::env;
use std::path::Path;
use std::process::Command;

fn main() {
    if env::var("CARGO_FEATURE_REAL_INJECTION_TESTS").is_ok() {
        build_gtk_test_app();
        build_terminal_test_app();
    }
}

fn build_gtk_test_app() {
    println!("cargo:rerun-if-changed=test-apps/gtk_test_app.c");

    let out_dir = env::var("OUT_DIR").unwrap();
    let executable_path = Path::new(&out_dir).join("gtk_test_app");

    // Get compiler and linker flags from pkg-config.
    let cflags = match Command::new("pkg-config").arg("--cflags").arg("gtk+-3.0").output() {
        Ok(output) if output.status.success() => String::from_utf8(output.stdout).unwrap(),
        _ => {
            println!("cargo:warning=Skipping GTK test app build: pkg-config could not get cflags for gtk+-3.0.");
            return;
        }
    };

    let libs = match Command::new("pkg-config").arg("--libs").arg("gtk+-3.0").output() {
        Ok(output) if output.status.success() => String::from_utf8(output.stdout).unwrap(),
        _ => {
            println!("cargo:warning=Skipping GTK test app build: pkg-config could not get libs for gtk+-3.0.");
            return;
        }
    };

    // Compile the test app using gcc, capturing all output.
    let gcc_output = Command::new("gcc")
        .arg("test-apps/gtk_test_app.c")
        .arg("-o")
        .arg(&executable_path)
        .args(cflags.split_whitespace())
        .args(libs.split_whitespace())
        .output() // Capture output instead of just status
        .expect("Failed to execute gcc");

    if !gcc_output.status.success() {
        println!("cargo:warning=Failed to compile GTK test app.");
        println!("cargo:warning=GCC stdout: {}", String::from_utf8_lossy(&gcc_output.stdout));
        println!("cargo:warning=GCC stderr: {}", String::from_utf8_lossy(&gcc_output.stderr));
    }
}

fn build_terminal_test_app() {
    println!("cargo:rerun-if-changed=test-apps/terminal-test-app/src/main.rs");
    println!("cargo:rerun-if-changed=test-apps/terminal-test-app/Cargo.toml");

    let out_dir = env::var("OUT_DIR").unwrap();
    let target_dir = Path::new(&out_dir).join("terminal-test-app-target");
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();

    let build_output = Command::new(env::var("CARGO").unwrap_or_else(|_| "cargo".to_string()))
        .current_dir(Path::new(&crate_dir).join("test-apps/terminal-test-app"))
        .arg("build")
        .arg("--release")
        .arg("--target-dir")
        .arg(&target_dir)
        .output() // Capture output
        .expect("Failed to execute cargo build for terminal-test-app");

    if !build_output.status.success() {
        println!("cargo:warning=Failed to build the terminal test app.");
        println!("cargo:warning=Cargo stdout: {}", String::from_utf8_lossy(&build_output.stdout));
        println!("cargo:warning=Cargo stderr: {}", String::from_utf8_lossy(&build_output.stderr));
    } else {
        let src_path = target_dir.join("release/terminal-test-app");
        let dest_path = Path::new(&out_dir).join("terminal-test-app");
        if let Err(e) = std::fs::copy(&src_path, &dest_path) {
            println!(
                "cargo:warning=Failed to copy terminal test app executable from {:?} to {:?}: {}",
                src_path, dest_path, e
            );
        }
    }
}
