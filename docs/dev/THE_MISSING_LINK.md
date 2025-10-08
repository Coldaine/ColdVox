# The Missing Link: Why Vendoring Isn't Working

**Issue**: "Why do we need to set LD_LIBRARY_PATH if we're vendoring?"  
**Answer**: We're vendoring, but not telling the **linker** where to find it.

---

## What's Actually Happening

### ✅ What Works (Vendoring)
```bash
vendor/vosk/
├── lib/
│   └── libvosk.so -> /home/coldaine/ActionRunnerCache/libvosk-setup/.../libvosk.so
└── model/
    └── vosk-model-en-us-0.22 -> /home/coldaine/ActionRunnerCache/vosk-models/...
```

**Status**: Symlinks created ✅  
**Created by**: `scripts/ci/setup-vosk-cache.sh` ✅  
**Used by**: CI workflows via `LD_LIBRARY_PATH` ✅

### ❌ What's Missing (Link-Time Discovery)

When `cargo build --features vosk` runs:
1. Compiles `coldvox-stt-vosk` crate
2. Depends on `vosk = "0.3"` from crates.io
3. `vosk` crate's `build.rs` says: `println!("cargo:rustc-link-lib=vosk");`
4. Linker looks for `libvosk.so` in:
   - `/lib/`
   - `/usr/lib/`
   - `/usr/local/lib/`
   - Anywhere in `LD_LIBRARY_PATH` (but this is runtime, not link-time!)
5. **Doesn't look in** `vendor/vosk/lib/` ❌
6. Linker fails: `error: unable to find library -lvosk`

---

## The Problem: Compile vs Runtime

### Runtime (Works in CI)
```yaml
# .github/workflows/ci.yml
env:
  LD_LIBRARY_PATH: ${{ needs.setup-vosk-dependencies.outputs.lib_path }}
run: cargo test  # ← Can find libvosk.so at runtime
```

**Why it works**: Executable already compiled, just needs library at runtime

### Link-Time (Broken Locally)
```bash
$ cargo build --features vosk
# ← Linker looks for libvosk.so DURING compilation
# ← vendor/vosk/lib not in search path
# ← ERROR: cannot find -lvosk
```

**Why it fails**: Linker doesn't know `vendor/vosk/lib/` exists

---

## Why "Just Set LD_LIBRARY_PATH" Doesn't Work

```bash
# This doesn't help at link-time:
export LD_LIBRARY_PATH=/path/to/vendor/vosk/lib
cargo build  # ← Still fails!
```

**Reason**: `LD_LIBRARY_PATH` is for the **dynamic linker at runtime**, not the **static linker at compile-time**.

The linker (`ld`) needs `-L` flags, which come from either:
1. Environment: `RUSTFLAGS="-L /path/to/lib"`
2. Build script: `build.rs` with `println!("cargo:rustc-link-search=native=/path/to/lib")`
3. Cargo config: `.cargo/config.toml` with `rustflags = ["-L", "/path"]`

---

## The Missing Pieces

### Missing: build.rs in coldvox-stt-vosk
```
crates/coldvox-stt-vosk/
├── Cargo.toml        ✅ Exists
├── src/              ✅ Exists
└── build.rs          ❌ MISSING!
```

**What it should do**:
```rust
// crates/coldvox-stt-vosk/build.rs
fn main() {
    // Tell linker to look in vendor directory
    println!("cargo:rustc-link-search=native=../../vendor/vosk/lib");
    
    // Also check system locations
    if std::path::Path::new("/usr/local/lib/libvosk.so").exists() {
        println!("cargo:rustc-link-search=native=/usr/local/lib");
    }
    
    // Tell cargo to re-run if vendor dir changes
    println!("cargo:rerun-if-changed=../../vendor/vosk/lib");
}
```

### Missing: .cargo/config.toml (Alternative)
```
.cargo/
└── config.toml       ❌ MISSING!
```

**What it should do**:
```toml
[target.x86_64-unknown-linux-gnu]
rustflags = [
    "-L", "vendor/vosk/lib",
    "-C", "link-args=-Wl,-rpath,$ORIGIN/../vendor/vosk/lib"
]
```

---

## Why CI "Works" (Sort Of)

CI workflows use a different approach:

```yaml
setup-vosk-dependencies:
  runs-on: self-hosted
  outputs:
    model_path: ${{ steps.setup.outputs.model_path }}
    lib_path: ${{ steps.setup.outputs.lib_path }}      # ← Exports path
  steps:
    - run: bash scripts/ci/setup-vosk-cache.sh

build_and_check:
  needs: [setup-vosk-dependencies]
  env:
    LD_LIBRARY_PATH: ${{ needs.setup-vosk-dependencies.outputs.lib_path }}  # ← Used at runtime
```

**But** this only works because:
1. libvosk is **already installed** in `/usr/local/lib/` on the runner
2. Linker finds it there **at compile-time**
3. `LD_LIBRARY_PATH` is only needed for **runtime** (running tests)

**Proof**: CI never runs `cargo build` with `LD_LIBRARY_PATH` in the link-search path.

---

## The Real Solution

We have **three** options to make vendoring work:

### Option 1: Add build.rs (Recommended)

**File**: `crates/coldvox-stt-vosk/build.rs`
```rust
use std::env;
use std::path::PathBuf;

fn main() {
    // Workspace root
    let workspace_root = env::var("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .unwrap()
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.to_path_buf())
        .expect("Could not determine workspace root");
    
    let vendor_lib = workspace_root.join("vendor/vosk/lib");
    
    // 1. Check vendor directory first
    if vendor_lib.exists() && vendor_lib.join("libvosk.so").exists() {
        println!("cargo:rustc-link-search=native={}", vendor_lib.display());
        println!("cargo:rustc-link-arg=-Wl,-rpath,{}", vendor_lib.display());
    }
    
    // 2. Fallback to system locations
    let system_locations = [
        "/usr/local/lib",
        "/usr/lib64",
        "/usr/lib",
    ];
    
    for location in &system_locations {
        let path = PathBuf::from(location);
        if path.join("libvosk.so").exists() {
            println!("cargo:rustc-link-search=native={}", location);
            break;
        }
    }
    
    // 3. Re-run if vendor changes
    println!("cargo:rerun-if-changed=../../vendor/vosk/lib");
    
    println!("cargo:rustc-link-lib=vosk");
}
```

**Benefits**:
- ✅ Works locally without environment setup
- ✅ Works in CI without LD_LIBRARY_PATH hacks
- ✅ Self-contained (no external config needed)
- ✅ Portable (checks multiple locations)

---

### Option 2: Cargo Config (Simpler but Less Flexible)

**File**: `.cargo/config.toml` (at workspace root)
```toml
[target.x86_64-unknown-linux-gnu]
rustflags = [
    "-L", "vendor/vosk/lib",
    "-C", "link-args=-Wl,-rpath,$ORIGIN/../vendor/vosk/lib",
]

[env]
VOSK_MODEL_PATH = { value = "vendor/vosk/model/vosk-model-en-us-0.22", relative = true }
```

**Benefits**:
- ✅ Simple, no Rust code needed
- ✅ Applies to entire workspace
- ❌ Less flexible (single path only)
- ❌ May conflict with other platforms

---

### Option 3: Wrapper Script (Hacky but Works)

**File**: `run_with_vosk.sh`
```bash
#!/bin/bash
export LD_LIBRARY_PATH="$(pwd)/vendor/vosk/lib:${LD_LIBRARY_PATH:-}"
export VOSK_MODEL_PATH="$(pwd)/vendor/vosk/model/vosk-model-en-us-0.22"
export RUSTFLAGS="-L $(pwd)/vendor/vosk/lib -C link-args=-Wl,-rpath,$(pwd)/vendor/vosk/lib"
exec "$@"
```

**Usage**:
```bash
./run_with_vosk.sh cargo build --features vosk
./run_with_vosk.sh cargo test --features vosk
```

**Benefits**:
- ✅ No code changes
- ✅ Easy to understand
- ❌ Extra step every time
- ❌ Doesn't solve root cause

---

## Why We Have So Many Unset Variables

You asked: "Why do we have so many unset variables with paths etc... stuff shouldn't move"

**Current situation**:
```bash
# CI workflows manually set:
VOSK_MODEL_PATH=...      # Runtime: where model files are
LD_LIBRARY_PATH=...      # Runtime: where libvosk.so is

# But these don't help compilation!
# They're only for running tests after compilation succeeds
```

**The root problem**: We're treating this as a **runtime** problem when it's actually a **link-time** problem.

**Philosophy mismatch**:
- **CI thinking**: "Download/symlink stuff, set env vars, should work"
- **Rust reality**: "Linker needs to know at compile-time, not runtime"

**Stuff moves because**:
1. Developer machine: libvosk might be in `/usr/local/lib` or vendor or not installed
2. CI runner: libvosk is in `/usr/local/lib` (system install)
3. Cache location: Changes based on runner configuration
4. Vendor symlinks: Point to different locations on different machines

**Solution**: Make vendor directory the **source of truth** and configure Rust to always look there first.

---

## Recommended Fix

**Add** `crates/coldvox-stt-vosk/build.rs` with smart path detection:

```rust
use std::env;
use std::path::PathBuf;

fn main() {
    // Find workspace root (go up from coldvox-stt-vosk to repo root)
    let workspace_root = env::var("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .and_then(|p| p.parent().and_then(|p| p.parent()).map(|p| p.to_path_buf()))
        .expect("Could not determine workspace root");
    
    let vendor_lib = workspace_root.join("vendor/vosk/lib");
    let vendor_model = workspace_root.join("vendor/vosk/model");
    
    println!("cargo:warning=Looking for libvosk in: {}", vendor_lib.display());
    
    // Priority 1: Vendored library
    if vendor_lib.join("libvosk.so").exists() {
        println!("cargo:rustc-link-search=native={}", vendor_lib.display());
        println!("cargo:rustc-link-arg=-Wl,-rpath,{}", vendor_lib.display());
        println!("cargo:warning=Using vendored libvosk from {}", vendor_lib.display());
    } else {
        // Priority 2: System install
        for location in ["/usr/local/lib", "/usr/lib64", "/usr/lib"] {
            if PathBuf::from(location).join("libvosk.so").exists() {
                println!("cargo:rustc-link-search=native={}", location);
                println!("cargo:warning=Using system libvosk from {}", location);
                break;
            }
        }
    }
    
    println!("cargo:rustc-link-lib=vosk");
    println!("cargo:rerun-if-changed={}", vendor_lib.display());
}
```

**Result**:
- ✅ `cargo build` works locally without any env vars
- ✅ `cargo test` works locally without any env vars
- ✅ CI continues to work (uses vendor if present, system if not)
- ✅ No more "stuff moves" - vendor is source of truth

---

## Summary

**Your intuition was correct**: We **are** vendoring, and stuff **shouldn't move**.

**The bug**: We set up vendoring for **runtime** but forgot to tell the **linker** about it.

**The fix**: Add `build.rs` to tell Rust "look in vendor/vosk/lib first, then system".

**Bonus**: This fixes the local development experience AND makes CI more robust.

**Impact**: One 20-line `build.rs` file eliminates the need for:
- Setting `LD_LIBRARY_PATH` manually
- Setting `RUSTFLAGS` manually  
- Wrapper scripts
- CI-specific path hacks

The vendoring **works**, we just weren't **using** it correctly.
