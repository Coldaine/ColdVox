
use crate::focus::{FocusStatus, FocusTracker};
use crate::tests::test_harness::{TestAppManager, TestEnvironment};
use crate::types::InjectionConfig;

#[tokio::test]
#[cfg_attr(
    not(feature = "live-hardware-tests"),
    ignore = "Skipping live hardware test for focus backend"
)]
async fn test_atspi_focus_backend_identifies_editable_text() {
    let env = TestEnvironment::current();
    if !env.can_run_real_tests() {
        println!("Skipping focus backend test: No display available.");
        return;
    }

    // Launch the GTK test application.
    let _test_app = TestAppManager::launch_gtk_app()
        .expect("Failed to launch GTK test app. Is 'build.rs' configured correctly?");

    // Give the app a moment to launch and gain focus.
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    // Create a FocusTracker and check the focus status.
    let config = InjectionConfig::default();
    let mut focus_tracker = FocusTracker::new(config);

    let status = focus_tracker
        .get_focus_status()
        .await
        .expect("Failed to get focus status");

    // The GTK test app should have a focused, editable text field.
    assert_eq!(
        status,
        FocusStatus::EditableText,
        "Focus backend should have identified the GTK app's focused text field as editable."
    );
}
