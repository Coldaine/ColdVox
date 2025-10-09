use std::sync::Once;
use tracing_subscriber::{fmt, EnvFilter};

/// Initialize tracing for tests with debug level
fn init_test_tracing() {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("debug"));

        fmt().with_env_filter(filter).with_test_writer().init();
    });
}

/// Initialize test infrastructure (tracing, sleep observer)
pub fn init_test_infrastructure() {
    init_test_tracing();
    crate::sleep_instrumentation::init_sleep_observer();
}
