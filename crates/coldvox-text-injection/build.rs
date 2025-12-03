use std::env;
use std::path::Path;
use std::process::Command;

fn main() {
    // Only run this build script if the `real-injection-tests` feature is enabled.
    if env::var("CARGO_CFG_FEATURE_REAL_INJECTION_TESTS").is_err() {
        println!("cargo:warning=Skipping build of test apps, real-injection-tests feature is not enabled.");
        return;
    }

    let out_dir = env::var("OUT_DIR").expect("OUT_DIR environment variable not set.");
    let src_path = Path::new("test-apps").join("text-capture-app.c");

    // --- Compile GTK Test App ---
    let gtk_app_out = Path::new(&out_dir).join("gtk_test_app");

    // Use pkg-config to get GTK flags.
    let gtk_cflags = pkg_config::Config::new().probe("gtk+-3.0").unwrap();
    let mut command = Command::new("cc");
    command.arg("-o").arg(&gtk_app_out);
    for flag in &gtk_cflags.cflags {
        command.arg(flag);
    }
    command.arg(&src_path);
    for lib_path in &gtk_cflags.link_paths {
        command.arg(format!("-L{}", lib_path.to_string_lossy()));
    }
    for lib in &gtk_cflags.libs {
        command.arg(format!("-l{}", lib));
    }

    let status = command
        .status()
        .expect("Failed to compile GTK test app with cc. Is a C compiler and GTK dev libraries installed?");

    if !status.success() {
        panic!("Failed to compile the GTK test app.");
    }

    println!(
        "cargo:warning=Successfully compiled GTK test app to: {:?}",
        gtk_app_out
    );

    // --- Compile Terminal Test App ---
    let term_app_out = Path::new(&out_dir).join("terminal_test_app");
    let status = Command::new("cc")
        .arg("-o")
        .arg(&term_app_out)
        .arg("-DTERMINAL_MODE")
        .arg(&src_path)
        .status()
        .expect("Failed to compile terminal test app with cc. Is a C compiler installed?");

    if !status.success() {
        panic!("Failed to compile the terminal test app.");
    }

    println!(
        "cargo:warning=Successfully compiled terminal test app to: {:?}",
        term_app_out
    );

    println!("cargo:rerun-if-changed=test-apps/text-capture-app.c");
}
