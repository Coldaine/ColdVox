//! Interactive diagnostics for the text injection strategy manager.
//!
//! This example helps troubleshoot live injection issues by loading the
//! standard `InjectionConfig`, printing the computed fallback chain, and
//! performing a real injection attempt. Run with
//!
//! ```bash
//! cargo run -p coldvox-text-injection --example injection_diagnostics \
//!     -- --config ../../config/default.toml --text "test phrase"
//! ```

use std::env;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use anyhow::{bail, Context, Result};
use coldvox_text_injection::types::InjectionMetrics;
use coldvox_text_injection::{InjectionConfig, StrategyManager};
use tracing::{debug, info, warn};
use tracing_subscriber::{fmt, EnvFilter};

fn init_tracing() {
    let _ = fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_target(true)
        .with_thread_ids(true)
        .with_line_number(true)
        .compact()
        .try_init();
}

fn print_usage(program: &str) {
    eprintln!(
        "Usage: {program} [--config <path>] [--text <string>] [--mode <auto|paste|keystroke>] \
         [--no-redact]\n\n\
         Examples:\n  {program} --text 'diagnostic ping'\n  {program} --config ../../config/default.toml --no-redact"
    );
}

#[derive(Debug, Default)]
struct CliOptions {
    text: Option<String>,
    config_path: Option<PathBuf>,
    mode_override: Option<String>,
    no_redact: bool,
}

fn parse_args() -> Result<CliOptions> {
    let mut args = env::args().skip(1);
    let mut options = CliOptions::default();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--config" => {
                let path = args.next().context("--config requires a path argument")?;
                options.config_path = Some(PathBuf::from(path));
            }
            "--text" => {
                let text = args.next().context("--text requires a string argument")?;
                options.text = Some(text);
            }
            "--mode" => {
                let mode = args.next().context("--mode requires a value")?;
                options.mode_override = Some(mode);
            }
            "--no-redact" => options.no_redact = true,
            "--help" | "-h" => {
                print_usage(&env::args().next().unwrap_or_default());
                std::process::exit(0);
            }
            other if other.starts_with('-') => {
                bail!("Unknown option: {other}");
            }
            positional => {
                options.text = Some(match options.text.take() {
                    Some(existing) => format!("{existing} {positional}"),
                    None => positional.to_string(),
                });
            }
        }
    }

    Ok(options)
}

fn load_config(options: &CliOptions) -> Result<InjectionConfig> {
    if let Some(path) = &options.config_path {
        let config_str = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file at {}", path.display()))?;
        let mut config: InjectionConfig = toml::from_str(&config_str)
            .with_context(|| format!("Failed to parse config file at {}", path.display()))?;
        if let Some(mode) = &options.mode_override {
            config.injection_mode = mode.clone();
        }
        if options.no_redact {
            config.redact_logs = false;
        }
        Ok(config)
    } else {
        let mut config = InjectionConfig::default();
        if let Some(mode) = &options.mode_override {
            config.injection_mode = mode.clone();
        }
        if options.no_redact {
            config.redact_logs = false;
        }
        Ok(config)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();

    let options = parse_args()?;
    debug!(?options, "Parsed CLI options");

    let config = load_config(&options)?;
    info!(
        redact_logs = config.redact_logs,
        mode = %config.injection_mode,
        "Loaded injection configuration"
    );

    let metrics = Arc::new(Mutex::new(InjectionMetrics::default()));
    let mut manager = StrategyManager::new(config.clone(), metrics.clone()).await;

    let order_preview = manager.get_method_order_uncached();
    info!(?order_preview, "Computed base fallback order");

    let text = options
        .text
        .unwrap_or_else(|| "diagnostic ping from injection_diagnostics".to_string());

    info!(char_count = text.len(), "Attempting live injection");
    match manager.inject(&text).await {
        Ok(()) => info!("Injection completed successfully"),
        Err(err) => warn!(error = %err, "Injection failed"),
    }

    Ok(())
}
