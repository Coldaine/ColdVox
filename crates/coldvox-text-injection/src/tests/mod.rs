#[cfg(test)]
mod test_adaptive_strategy;
#[cfg(test)]
mod test_focus_tracking;
#[cfg(test)]
mod test_integration;
#[cfg(test)]
mod test_permission_checking;
#[cfg(test)]
mod test_window_manager;

#[cfg(all(test, feature = "real-injection-tests"))]
mod real_injection;
#[cfg(all(test, feature = "real-injection-tests"))]
mod real_injection_smoke;
#[cfg(test)]
mod test_allow_block;
#[cfg(test)]
mod test_async_processor;
#[cfg(test)]
mod test_focus_enforcement;
#[cfg(all(test, feature = "real-injection-tests"))]
mod test_harness;
#[cfg(test)]
mod test_util;
