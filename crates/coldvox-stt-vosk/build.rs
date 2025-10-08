use std::env;
use std::path::PathBuf;

fn main() {
    // Determine workspace root by going up two levels from the crate manifest dir
    let manifest_dir = env::var("CARGO_MANIFEST_DIR")
        .expect("CARGO_MANIFEST_DIR not set; this build script must be run by Cargo");

    let mut workspace_root = PathBuf::from(manifest_dir);
    // Pop crate dir -> workspace/crates -> repo root: go up two levels
    workspace_root.pop();
    workspace_root.pop();

    let vendor_lib = workspace_root.join("vendor/vosk/lib");

    // Priority 1: vendored library
    if vendor_lib.join("libvosk.so").exists() {
        println!("cargo:warning=Using vendored libvosk from {}", vendor_lib.display());
        println!("cargo:rustc-link-search=native={}", vendor_lib.display());
        // Add rpath so runtime can find the vendored library relative to the binary
        println!("cargo:rustc-link-arg=-Wl,-rpath,{}", vendor_lib.display());
    } else {
        // Fallback: check common system paths
        let system_locations = ["/usr/local/lib", "/usr/lib64", "/usr/lib"];
        for loc in &system_locations {
            let path = PathBuf::from(loc);
            if path.join("libvosk.so").exists() {
                println!("cargo:warning=Using system libvosk from {}", loc);
                println!("cargo:rustc-link-search=native={}", loc);
                break;
            }
        }
    }

    // Always link against vosk
    println!("cargo:rustc-link-lib=vosk");

    // Re-run build script when vendored lib changes
    println!("cargo:rerun-if-changed={}", vendor_lib.display());
}
