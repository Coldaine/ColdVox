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

#[cfg(test)]
mod test_allow_block;
#[cfg(test)]
mod test_async_processor;
#[cfg(test)]
mod test_focus_enforcement;
#[cfg(test)]
mod test_util;
#[cfg(test)]
mod test_harness;
#[cfg(all(test, feature = "real-injection-tests"))]
mod real_injection;
