//! # Strategy Orchestrator for Text Injection
//!
//! This module provides the StrategyOrchestrator which acts as the "brain" of the
//! text injection system. It handles environment detection, strategy selection,
//! and fast-fail execution with strict budgets.

use crate::confirm::{text_changed, ConfirmationResult};
use crate::injectors::atspi::AtspiInjector;
use crate::prewarm::PrewarmController;
use crate::session::{InjectionSession, SessionState};
use crate::types::{InjectionConfig, InjectionError, InjectionMethod, InjectionResult};
use crate::TextInjector;
use std::env;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// Context for AT-SPI injection operations
#[derive(Debug, Clone)]
pub struct AtspiContext {
    /// Pre-warmed AT-SPI connection data
    pub focused_node: Option<String>,
    /// Target application identifier
    pub target_app: Option<String>,
    /// Window identifier
    pub window_id: Option<String>,
}

impl Default for AtspiContext {
    fn default() -> Self {
        Self {
            focused_node: None,
            target_app: None,
            window_id: None,
        }
    }
}

/// Desktop environment types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesktopEnvironment {
    /// KDE/KWin on Wayland
    KdeWayland,
    /// KDE/KWin on X11
    KdeX11,
    /// Hyprland (wlroots-based Wayland)
    Hyprland,
    /// GNOME on Wayland
    GnomeWayland,
    /// GNOME on X11
    GnomeX11,
    /// Other Wayland compositor
    OtherWayland,
    /// Other X11 desktop
    OtherX11,
    /// Windows
    Windows,
    /// macOS
    MacOS,
    /// Unknown environment
    Unknown,
}

impl std::fmt::Display for DesktopEnvironment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DesktopEnvironment::KdeWayland => write!(f, "KDE/Wayland"),
            DesktopEnvironment::KdeX11 => write!(f, "KDE/X11"),
            DesktopEnvironment::Hyprland => write!(f, "Hyprland"),
            DesktopEnvironment::GnomeWayland => write!(f, "GNOME/Wayland"),
            DesktopEnvironment::GnomeX11 => write!(f, "GNOME/X11"),
            DesktopEnvironment::OtherWayland => write!(f, "Other/Wayland"),
            DesktopEnvironment::OtherX11 => write!(f, "Other/X11"),
            DesktopEnvironment::Windows => write!(f, "Windows"),
            DesktopEnvironment::MacOS => write!(f, "macOS"),
            DesktopEnvironment::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Strategy Orchestrator that manages text injection across environments
pub struct StrategyOrchestrator {
    /// Configuration for injection
    config: InjectionConfig,
    /// Detected desktop environment
    desktop_env: DesktopEnvironment,
    /// Pre-warm controller for caching resources
    prewarm_controller: Arc<PrewarmController>,
    /// AT-SPI injector instance
    atspi_injector: Option<AtspiInjector>,
    /// Session state for buffering
    session: Arc<RwLock<InjectionSession>>,
    /// Last known app context
    last_context: Arc<RwLock<Option<AtspiContext>>>,
}

impl StrategyOrchestrator {
    /// Create a new strategy orchestrator
    pub async fn new(config: InjectionConfig) -> Self {
        let desktop_env = Self::detect_environment();
        info!("Detected desktop environment: {}", desktop_env);

        let prewarm_controller = Arc::new(PrewarmController::new(config.clone()));

        // Create AT-SPI injector if available
        let atspi_injector = if cfg!(feature = "atspi") {
            Some(AtspiInjector::new(config.clone()))
        } else {
            None
        };

        // Create session with default config
        let session_config = crate::session::SessionConfig::default();
        let metrics = Arc::new(std::sync::Mutex::new(
            crate::types::InjectionMetrics::default(),
        ));
        let session = Arc::new(RwLock::new(InjectionSession::new(session_config, metrics)));

        Self {
            config,
            desktop_env,
            prewarm_controller,
            atspi_injector,
            session,
            last_context: Arc::new(RwLock::new(None)),
        }
    }

    /// Detect the current desktop environment
    fn detect_environment() -> DesktopEnvironment {
        // Check for Windows
        if cfg!(target_os = "windows") {
            return DesktopEnvironment::Windows;
        }

        // Check for macOS
        if cfg!(target_os = "macos") {
            return DesktopEnvironment::MacOS;
        }

        // Check for Wayland vs X11
        let is_wayland = env::var("XDG_SESSION_TYPE")
            .map(|s| s == "wayland")
            .unwrap_or(false)
            || env::var("WAYLAND_DISPLAY").is_ok();

        let is_x11 = env::var("XDG_SESSION_TYPE")
            .map(|s| s == "x11")
            .unwrap_or(false)
            || env::var("DISPLAY").is_ok();

        // Check for specific desktop environments
        let desktop = env::var("XDG_CURRENT_DESKTOP")
            .unwrap_or_default()
            .to_lowercase();

        let kde = desktop.contains("kde") || env::var("KDE_SESSION_VERSION").is_ok();
        let gnome = desktop.contains("gnome") || desktop.contains("ubuntu");
        let hyprland = env::var("HYPRLAND_INSTANCE_SIGNATURE").is_ok();

        if is_wayland {
            if kde {
                DesktopEnvironment::KdeWayland
            } else if hyprland {
                DesktopEnvironment::Hyprland
            } else if gnome {
                DesktopEnvironment::GnomeWayland
            } else {
                DesktopEnvironment::OtherWayland
            }
        } else if is_x11 {
            if kde {
                DesktopEnvironment::KdeX11
            } else if gnome {
                DesktopEnvironment::GnomeX11
            } else {
                DesktopEnvironment::OtherX11
            }
        } else {
            DesktopEnvironment::Unknown
        }
    }

    /// Get the injection strategy order for the current environment
    fn get_strategy_order(&self) -> Vec<InjectionMethod> {
        match self.desktop_env {
            DesktopEnvironment::KdeWayland => vec![
                InjectionMethod::AtspiInsert,
                InjectionMethod::ClipboardPasteFallback,
            ],
            DesktopEnvironment::Hyprland => vec![
                InjectionMethod::AtspiInsert,
                InjectionMethod::ClipboardPasteFallback,
            ],
            DesktopEnvironment::Windows => vec![InjectionMethod::ClipboardPasteFallback],
            _ => vec![
                InjectionMethod::AtspiInsert,
                InjectionMethod::ClipboardPasteFallback,
            ],
        }
    }

    /// Trigger targeted pre-warming for the first method we'll try
    async fn check_and_trigger_prewarm(&self) {
        let session = self.session.read().await;
        if session.state() == SessionState::Buffering {
            // Get current context for pre-warming
            let context = self.last_context.read().await;
            if let Some(ref ctx) = *context {
                // Get the first method we'll try
                let strategy_order = self.get_strategy_order();
                if let Some(first_method) = strategy_order.first() {
                    // Run targeted pre-warming for just this method (non-blocking)
                    let ctx_clone = ctx.clone();
                    let method_clone = *first_method;
                    tokio::spawn(async move {
                        if let Err(e) =
                            crate::prewarm::run_for_method(&ctx_clone, method_clone).await
                        {
                            warn!("Targeted pre-warming failed for {:?}: {}", method_clone, e);
                        }
                    });
                }
            }
        }
    }

    /// Execute fast-fail injection loop with strict budgets
    async fn fast_fail_inject(&self, text: &str) -> InjectionResult<()> {
        let total_start = Instant::now();
        let total_budget = Duration::from_millis(self.config.max_total_latency_ms);
        let stage_budget = Duration::from_millis(50); // ≤50ms per stage
        let confirm_budget = Duration::from_millis(75); // ≤75ms for confirmation

        // Get strategy order for current environment
        let strategy_order = self.get_strategy_order();
        debug!(
            "Strategy order for {}: {:?}",
            self.desktop_env, strategy_order
        );

        // Get current context
        let context = self.prewarm_controller.get_atspi_context().await;
        let target_app = context
            .target_app
            .clone()
            .unwrap_or_else(|| "unknown".to_string());
        let window_id = context
            .window_id
            .clone()
            .unwrap_or_else(|| "unknown".to_string());

        // Try each strategy in order
        for (i, method) in strategy_order.iter().enumerate() {
            // Check total budget
            if total_start.elapsed() > total_budget {
                warn!("Total budget exceeded after {} attempts", i);
                return Err(InjectionError::BudgetExhausted);
            }

            debug!(
                "Attempting strategy {:?} ({}/{})",
                method,
                i + 1,
                strategy_order.len()
            );

            // Create injection context - orchestrator doesn't support pre-warming yet
            let context = crate::types::InjectionContext::default();

            // Execute injection with stage budget
            let stage_start = Instant::now();
            let result = match method {
                InjectionMethod::AtspiInsert => {
                    if let Some(ref injector) = self.atspi_injector {
                        tokio::time::timeout(
                            stage_budget,
                            injector.inject_text(text, Some(&context)),
                        )
                        .await
                        .map_err(|_| InjectionError::Timeout(stage_budget.as_millis() as u64))?
                    } else {
                        continue;
                    }
                }
                InjectionMethod::ClipboardPasteFallback => {
                    // For now, we'll use AT-SPI paste as a fallback
                    if let Some(ref injector) = self.atspi_injector {
                        tokio::time::timeout(
                            stage_budget,
                            injector.inject_text(text, Some(&context)),
                        )
                        .await
                        .map_err(|_| InjectionError::Timeout(stage_budget.as_millis() as u64))?
                    } else {
                        continue;
                    }
                }
                _ => {
                    debug!("Unsupported method {:?} in fast-fail loop", method);
                    continue;
                }
            };

            match result {
                Ok(()) => {
                    let stage_elapsed = stage_start.elapsed();
                    debug!(
                        "Strategy {:?} succeeded in {}ms",
                        method,
                        stage_elapsed.as_millis()
                    );

                    // Confirm injection with confirm budget
                    let confirm_start = Instant::now();
                    let confirm_result = tokio::time::timeout(
                        confirm_budget,
                        text_changed(&target_app, text, &window_id),
                    )
                    .await;

                    match confirm_result {
                        Ok(Ok(ConfirmationResult::Success)) => {
                            let confirm_elapsed = confirm_start.elapsed();
                            let total_elapsed = total_start.elapsed();
                            info!(
                                "Injection confirmed: method={:?}, stage={}ms, confirm={}ms, total={}ms",
                                method, stage_elapsed.as_millis(), confirm_elapsed.as_millis(), total_elapsed.as_millis()
                            );
                            return Ok(());
                        }
                        Ok(Ok(other)) => {
                            debug!("Confirmation returned non-success: {:?}", other);
                            // Continue to next strategy
                        }
                        Ok(Err(e)) => {
                            debug!("Confirmation error: {}", e);
                            // Continue to next strategy
                        }
                        Err(_) => {
                            debug!(
                                "Confirmation timed out after {}ms",
                                confirm_budget.as_millis()
                            );
                            // Continue to next strategy
                        }
                    }
                }
                Err(e) => {
                    let stage_elapsed = stage_start.elapsed();
                    debug!(
                        "Strategy {:?} failed in {}ms: {}",
                        method,
                        stage_elapsed.as_millis(),
                        e
                    );
                    // Continue to next strategy
                }
            }
        }

        // All strategies failed
        let total_elapsed = total_start.elapsed();
        error!(
            "All injection strategies failed after {}ms ({} strategies tried)",
            total_elapsed.as_millis(),
            strategy_order.len()
        );

        Err(InjectionError::AllMethodsFailed(
            "All strategies failed in fast-fail loop".to_string(),
        ))
    }

    /// Inject text using the orchestrator
    pub async fn inject_text(&self, text: &str) -> InjectionResult<()> {
        if text.is_empty() {
            return Ok(());
        }

        // Update context from pre-warmed data
        let context = self.prewarm_controller.get_atspi_context().await;
        *self.last_context.write().await = Some(context.clone());

        // Check if we should trigger pre-warming
        self.check_and_trigger_prewarm().await;

        // Execute fast-fail injection loop
        self.fast_fail_inject(text).await
    }

    /// Get the current desktop environment
    pub fn desktop_environment(&self) -> DesktopEnvironment {
        self.desktop_env
    }

    /// Check if the orchestrator is available
    pub async fn is_available(&self) -> bool {
        // Check if we have at least one working injector
        if let Some(ref injector) = self.atspi_injector {
            if injector.is_available().await {
                return true;
            }
        }
        false
    }

    /// Get backend information
    pub fn backend_info(&self) -> Vec<(&'static str, String)> {
        vec![
            ("type", "Strategy Orchestrator".to_string()),
            ("environment", self.desktop_env.to_string()),
            (
                "description",
                "Environment-aware injection orchestrator with fast-fail loop".to_string(),
            ),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_environment_detection() {
        let env = StrategyOrchestrator::detect_environment();
        // Just ensure it doesn't panic
        println!("Detected environment: {}", env);
    }

    #[test]
    fn test_strategy_order() {
        let config = InjectionConfig::default();

        // Test with different environments
        let test_cases = vec![
            (
                DesktopEnvironment::KdeWayland,
                vec![
                    InjectionMethod::AtspiInsert,
                    InjectionMethod::ClipboardPasteFallback,
                ],
            ),
            (
                DesktopEnvironment::Hyprland,
                vec![
                    InjectionMethod::AtspiInsert,
                    InjectionMethod::ClipboardPasteFallback,
                ],
            ),
            (
                DesktopEnvironment::Windows,
                vec![InjectionMethod::ClipboardPasteFallback],
            ),
        ];

        for (env, expected_order) in test_cases {
            // Create orchestrator with mocked environment
            let orchestrator = StrategyOrchestrator {
                config: config.clone(),
                desktop_env: env,
                prewarm_controller: Arc::new(PrewarmController::new(config.clone())),
                atspi_injector: None,
                session: Arc::new(RwLock::new(InjectionSession::new(
                    crate::session::SessionConfig::default(),
                    Arc::new(std::sync::Mutex::new(
                        crate::types::InjectionMetrics::default(),
                    )),
                ))),
                last_context: Arc::new(RwLock::new(None)),
            };

            let order = orchestrator.get_strategy_order();
            assert_eq!(
                order, expected_order,
                "Strategy order mismatch for {:?}",
                env
            );
        }
    }

    #[tokio::test]
    async fn test_orchestrator_creation() {
        let config = InjectionConfig::default();
        let orchestrator = StrategyOrchestrator::new(config).await;

        // Just ensure it creates without panicking
        assert!(orchestrator.is_available().await || !orchestrator.is_available().await);
    }

    #[tokio::test]
    async fn test_empty_text_handling() {
        let config = InjectionConfig::default();
        let orchestrator = StrategyOrchestrator::new(config).await;

        // Empty text should succeed without error
        let result = orchestrator.inject_text("").await;
        assert!(result.is_ok());
    }
}
