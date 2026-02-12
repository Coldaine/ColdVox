//! Unit tests for ydotool_injector.rs
use crate::ydotool_injector::{candidate_socket_paths, locate_existing_socket, YdotoolInjector};
use crate::types::InjectionConfig;
use anyhow::Result;
use serial_test::serial;
use std::env;
use std::fs::{self, File};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use tempfile::{tempdir, TempDir};

/// A test harness to create a controlled environment for ydotool tests.
struct TestHarness {
    _temp_dir: TempDir,
    bin_dir: PathBuf,
    home_dir: PathBuf,
    runtime_dir: PathBuf,
    original_path: String,
    original_home: Option<String>,
    original_xdg_runtime_dir: Option<String>,
    /// Path to a file that mock binaries can use to report arguments.
    output_file: PathBuf,
}

impl TestHarness {
    fn new() -> Result<Self> {
        let temp_dir = tempdir()?;
        let base_path = temp_dir.path();

        let bin_dir = base_path.join("bin");
        let home_dir = base_path.join("home");
        let runtime_dir = base_path.join("run");
        let uinput_path = base_path.join("uinput");
        let output_file = base_path.join("output.log");

        fs::create_dir_all(&bin_dir)?;
        fs::create_dir_all(&home_dir)?;
        fs::create_dir_all(&runtime_dir)?;
        File::create(&uinput_path)?;
        File::create(&output_file)?;

        let original_path = env::var("PATH").unwrap_or_default();
        let original_home = env::var("HOME").ok();
        let original_xdg_runtime_dir = env::var("XDG_RUNTIME_DIR").ok();

        let new_path = format!("{}:{}", bin_dir.display(), original_path);
        env::set_var("PATH", new_path);
        env::set_var("HOME", &home_dir);
        env::set_var("XDG_RUNTIME_DIR", &runtime_dir);
        env::set_var("UINPUT_PATH_OVERRIDE", &uinput_path);

        env::remove_var("YDOTOOL_SOCKET");
        env::remove_var("UID");

        Ok(Self {
            _temp_dir: temp_dir,
            bin_dir,
            home_dir,
            runtime_dir,
            original_path,
            original_home,
            original_xdg_runtime_dir,
            output_file,
        })
    }

    /// Creates a mock executable file that echoes a specific path.
    fn create_which_mock(&self, target_binary: &Path) -> Result<()> {
        let content = format!("#!/bin/sh\necho {}", target_binary.display());
        self.create_mock_binary("which", &content, true)?;
        Ok(())
    }

    fn create_mock_binary(&self, name: &str, content: &str, executable: bool) -> Result<PathBuf> {
        let path = self.bin_dir.join(name);
        fs::write(&path, content)?;
        if executable {
            fs::set_permissions(&path, fs::Permissions::from_mode(0o755))?;
        }
        Ok(path)
    }

    fn create_mock_socket(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        File::create(path)?;
        Ok(())
    }

    /// Reads the content of the argument log file.
    fn read_output(&self) -> Result<String> {
        Ok(fs::read_to_string(&self.output_file)?)
    }
}

impl Drop for TestHarness {
    fn drop(&mut self) {
        env::set_var("PATH", &self.original_path);

        // Restore or remove HOME
        if let Some(ref original_home) = self.original_home {
            env::set_var("HOME", original_home);
        } else {
            env::remove_var("HOME");
        }

        // Restore or remove XDG_RUNTIME_DIR
        if let Some(ref original_xdg_runtime_dir) = self.original_xdg_runtime_dir {
            env::set_var("XDG_RUNTIME_DIR", original_xdg_runtime_dir);
        } else {
            env::remove_var("XDG_RUNTIME_DIR");
        }

        env::remove_var("YDOTOOL_SOCKET");
        env::remove_var("UID");
        env::remove_var("UINPUT_PATH_OVERRIDE");
    }
}

#[test]
#[serial]
fn test_candidate_socket_paths_priority() {
    let _harness = TestHarness::new().unwrap();
    env::set_var("YDOTOOL_SOCKET", "/custom/socket");
    env::set_var("UID", "1001");

    let paths = candidate_socket_paths();
    assert_eq!(paths.len(), 4);
    assert_eq!(paths[0], PathBuf::from("/custom/socket"));
}

#[test]
#[serial]
fn test_locate_existing_socket_finds_first_available() {
    let harness = TestHarness::new().unwrap();
    let runtime_socket = harness.runtime_dir.join(".ydotool_socket");
    harness.create_mock_socket(&runtime_socket).unwrap();
    let expected_socket = harness.home_dir.join(".ydotool").join("socket");
    harness.create_mock_socket(&expected_socket).unwrap();

    let located = locate_existing_socket();
    assert_eq!(located, Some(expected_socket));
}

#[test]
#[serial]
fn test_check_binary_permissions_success() {
    let harness = TestHarness::new().unwrap();
    let ydotool_path = harness
        .create_mock_binary("ydotool", "#!/bin/sh\nexit 0", true)
        .unwrap();
    harness.create_which_mock(&ydotool_path).unwrap();

    let result = YdotoolInjector::check_binary_permissions("ydotool");
    assert!(result.is_ok());
}

#[tokio::test]
#[serial]
async fn test_check_ydotool_available_when_binary_and_socket_present() {
    let harness = TestHarness::new().unwrap();
    let ydotool_path = harness
        .create_mock_binary("ydotool", "", true)
        .unwrap();
    harness.create_which_mock(&ydotool_path).unwrap();
    let socket_path = harness.home_dir.join(".ydotool/socket");
    harness.create_mock_socket(&socket_path).unwrap();

    let injector = YdotoolInjector::new(InjectionConfig::default());
    assert!(injector.is_available().await);
}

#[tokio::test]
#[serial]
async fn test_inject_text_uses_paste_by_default() {
    let harness = TestHarness::new().unwrap();
    let ydotool_script = format!(
        "#!/bin/sh\necho \"$@\" > {}",
        harness.output_file.display()
    );
    let ydotool_path = harness
        .create_mock_binary("ydotool", &ydotool_script, true)
        .unwrap();
    harness.create_which_mock(&ydotool_path).unwrap();
    let socket_path = harness.home_dir.join(".ydotool/socket");
    harness.create_mock_socket(&socket_path).unwrap();

    let injector = YdotoolInjector::new(InjectionConfig::default());
    let result = injector.inject_text("hello", None).await;

    assert!(result.is_ok());
    let output = harness.read_output().unwrap();
    assert!(output.contains("key ctrl+v"));
}

#[tokio::test]
#[serial]
async fn test_inject_text_falls_back_to_type() {
    let harness = TestHarness::new().unwrap();
    // This mock fails for 'key' command, but succeeds for 'type'
    let ydotool_script = format!(
        r#"#!/bin/sh
if [ "$1" = "key" ]; then
  exit 1
else
  echo "$@" > {}
fi
"#,
        harness.output_file.display()
    );
    let ydotool_path = harness
        .create_mock_binary("ydotool", &ydotool_script, true)
        .unwrap();
    harness.create_which_mock(&ydotool_path).unwrap();
    let socket_path = harness.home_dir.join(".ydotool/socket");
    harness.create_mock_socket(&socket_path).unwrap();

    let injector = YdotoolInjector::new(InjectionConfig::default());
    let result = injector.inject_text("world", None).await;

    assert!(result.is_ok());
    let output = harness.read_output().unwrap();
    assert!(output.contains("type --delay 10 world"));
}
