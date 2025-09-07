//! # Tests for coldvox-text-injection
//!
//! This module contains all the tests for the crate.
//! The tests are organized into submodules.

#[cfg(test)]
mod test_window_manager;

#[cfg(all(test, feature = "real-injection-tests"))]
mod real_injection;

#[cfg(test)]
mod real_injection_smoke;

#[cfg(all(test, feature = "real-injection-tests"))]
mod harness;

#[cfg(test)]
mod test_util;
