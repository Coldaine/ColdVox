//! # Adaptive Strategy Manager for Text Injection
//!
//! This module contains the `StrategyManager`, the central orchestrator for
//! text injection. It is responsible for probing the environment, selecting
//! an appropriate injection backend, executing the injection with strict
//! timeouts, and handling retries and fallbacks.

#[cfg(feature = "atspi")]
use crate::atspi_injector::AtspiInjector;
#[cfg(feature = "wl_clipboard")]
use crate::clipboard_injector::ClipboardInjector;
use crate::constants::{GLOBAL_INJECTION_BUDGET_MS, PER_BACKEND_SOFT_TIMEOUT_MS};
use crate::error::{InjectionError, UnavailableCause};
use crate::metrics::MetricsSink;
use crate::outcome::InjectionOutcome;
use crate::probe::{probe_environment, BackendId, ProbeState};
use crate::{InjectionConfig, TextInjector};
use std::time::{Duration, Instant};
use tokio::time::timeout;
use tracing::{debug, info, warn};

/// The `StrategyManager` orchestrates the text injection process.
pub struct StrategyManager {
    /// Shared configuration for all injectors.
    config: InjectionConfig,
    /// The total time budget for a single injection call.
    global_budget_ms: u64,
    /// The soft time budget for a single backend attempt.
    per_backend_soft_ms: u64,
}

impl StrategyManager {
    /// Creates a new `StrategyManager`.
    pub fn new(config: InjectionConfig) -> Self {
        Self {
            config,
            global_budget_ms: GLOBAL_INJECTION_BUDGET_MS,
            per_backend_soft_ms: PER_BACKEND_SOFT_TIMEOUT_MS,
        }
    }

    /// The main entry point for injecting text.
    ///
    /// This function performs the following steps:
    /// 1. Probes the environment to determine available backends.
    /// 2. Fails fast if the environment is completely unsuitable.
    /// 3. Tries each available backend in a deterministic order.
    /// 4. Wraps each attempt in a global budget and per-backend timeouts.
    /// 5. Performs a single retry on specific transient errors.
    pub async fn inject_with_fail_fast(
        &self,
        text: &str,
        metrics: &mut dyn MetricsSink,
    ) -> Result<InjectionOutcome, InjectionError> {
        let overall_start = Instant::now();
        let global_budget = Duration::from_millis(self.global_budget_ms);

        // Wrap the entire operation in a global timeout.
        match timeout(global_budget, async {
            // 1. Probe the environment.
            let probe = probe_environment().await;

            // 2. Fast triage based on probe results.
            if let ProbeState::Missing { causes } = &probe {
                let formatted_causes = causes.iter().map(|(c, r)| format!("{}: {}", c, r)).collect();
                return Err(InjectionError::Unavailable {
                    backend: BackendId::Fallback,
                    cause: UnavailableCause::Environment {
                        causes: formatted_causes,
                    },
                });
            }

            // 3. Get an ordered list of candidate backends.
            let candidates = self.order_backends_from_probe(&probe);
            if candidates.is_empty() {
                return Err(InjectionError::Unavailable {
                    backend: BackendId::Fallback,
                    cause: UnavailableCause::Exhausted,
                });
            }
            info!("Injection candidates: {:?}", candidates);

            // 4. Try each candidate.
            for &backend in &candidates {
                metrics.emit_start(backend);
                let attempt = self
                    .try_backend_with_retry(backend, text, self.per_backend_soft_ms)
                    .await;

                match attempt {
                    Ok(outcome) => {
                        info!(
                            "Injection success with backend {:?} in {}ms.",
                            backend, outcome.latency_ms
                        );
                        metrics.emit_success(backend, outcome.latency_ms);
                        return Ok(outcome);
                    }
                    Err(e) => {
                        warn!("Backend {:?} failed: {}", backend, e);
                        metrics.emit_fail(backend, &e);
                        // If the error is definitive (e.g., not a transient focus issue),
                        // we continue to the next backend. If it was transient, the retry
                        // is already handled inside `try_backend_with_retry`.
                        continue;
                    }
                }
            }

            Err(InjectionError::Unavailable {
                backend: BackendId::Fallback,
                cause: UnavailableCause::Exhausted,
            })
        }).await {
            Ok(result) => result,
            Err(_) => Err(InjectionError::Timeout {
                backend: BackendId::Fallback,
                phase: "global",
                elapsed_ms: overall_start.elapsed().as_millis() as u32,
            }),
        }
    }

    /// Tries a backend, with a single retry on a transient, retryable error.
    async fn try_backend_with_retry(
        &self,
        backend: BackendId,
        text: &str,
        soft_ms: u64,
    ) -> Result<InjectionOutcome, InjectionError> {
        let first_attempt = self.try_once(backend, text, soft_ms).await;
        match first_attempt {
            Err(InjectionError::Transient { retryable: true, .. }) => {
                debug!("Transient error with {:?}, retrying once.", backend);
                tokio::time::sleep(Duration::from_millis(50)).await; // Small delay before retry
                self.try_once(backend, text, soft_ms).await
            }
            other => other,
        }
    }

    /// Tries a single backend injection attempt, wrapped in a soft timeout.
    async fn try_once(
        &self,
        backend: BackendId,
        text: &str,
        soft_ms: u64,
    ) -> Result<InjectionOutcome, InjectionError> {
        let backend_budget = Duration::from_millis(soft_ms);
        match timeout(backend_budget, async {
            // Construct the injector on-demand.
            match backend {
                #[cfg(feature = "atspi")]
                BackendId::Atspi => {
                    let injector = AtspiInjector::new(self.config.clone());
                    injector.inject_text(text).await
                }
                #[cfg(feature = "wl_clipboard")]
                BackendId::ClipboardWayland | BackendId::ClipboardX11 => {
                    if let Some(injector) = ClipboardInjector::new(self.config.clone()) {
                        injector.inject_text(text).await
                    } else {
                        Err(InjectionError::Unavailable {
                            backend,
                            cause: UnavailableCause::Environment {
                                causes: vec!["Clipboard session type mismatch".to_string()],
                            },
                        })
                    }
                }
                // This is a catch-all for other backends when their features are not enabled.
                _ => Err(InjectionError::Unavailable {
                    backend,
                    cause: UnavailableCause::Environment {
                        causes: vec![format!("Backend {:?} feature not enabled", backend)],
                    },
                }),
            }
        }).await {
            Ok(result) => result,
            Err(_) => Err(InjectionError::Timeout {
                backend,
                phase: "backend",
                elapsed_ms: soft_ms as u32,
            }),
        }
    }

    /// Determines the order of backends to try based on the probe results.
    fn order_backends_from_probe(&self, probe: &ProbeState) -> Vec<BackendId> {
        let usable = match probe {
            ProbeState::FullyAvailable { usable, .. } => usable,
            ProbeState::Degraded { usable, .. } => usable,
            ProbeState::Missing { .. } => return vec![],
        };

        // A simple, deterministic priority order.
        let priority = [
            BackendId::Atspi,
            BackendId::ClipboardWayland,
            BackendId::ClipboardX11,
            BackendId::Ydotool,
        ];

        let mut ordered = Vec::new();
        for &p in &priority {
            if usable.contains(&p) {
                ordered.push(p);
            }
        }
        ordered
    }
}
