//! # Always-On Smoke Test
//!
//! This test runs in all environments, including headless CI. It does not
//! perform any real I/O but verifies that the core components of the
//! injection system can be initialized without panicking.

use crate::manager::StrategyManager;
use crate::probe::probe_environment;
use crate::InjectionConfig;

/// This test ensures that the `StrategyManager` can be created and that the
/// environment probe can be run without panicking. It's a basic sanity check.
#[tokio::test]
async fn smoke_test_manager_init_and_probe() {
    // Create a default config.
    let config = InjectionConfig::default();

    // Create the manager. This ensures its constructor logic is sound.
    let _manager = StrategyManager::new(config);

    // Run the environment probe.
    // This is the most critical part of the smoke test. It executes the probe
    // logic, including the subprocess checks (with very short timeouts), to
    // ensure they don't hang or panic.
    let probe_result = probe_environment().await;

    // We don't assert on the *result* of the probe, as it will vary by
    // environment. We just assert that it completed and returned a valid enum.
    // The simple act of getting here without a panic is the success condition.
    println!("Smoke test probe completed with result: {:?}", probe_result);
    assert!(matches!(
        probe_result,
        crate::probe::ProbeState::FullyAvailable { .. }
            | crate::probe::ProbeState::Degraded { .. }
            | crate::probe::ProbeState::Missing { .. }
    ));
}
