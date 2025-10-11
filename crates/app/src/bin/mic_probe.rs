use clap::{Parser, Subcommand};
use coldvox_app::probes::{
    common::{ensure_results_dir, write_result_json, LiveTestResult, TestContext},
    MicCaptureCheck, VadMicCheck,
};
use std::path::PathBuf;
use std::time::Duration;

#[derive(Parser)]
#[command(name = "mic-probe")]
#[command(version = "1.0")]
#[command(about = "ColdVox live audio testing tool")]
#[command(
    long_about = "Comprehensive audio testing tool for microphone capture, VAD processing, and system validation"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Audio device name
    #[arg(short = 'D', long, global = true)]
    device: Option<String>,

    /// Test duration in seconds
    #[arg(short = 'd', long, default_value = "10", global = true)]
    duration: u64,

    /// Output directory for test results
    #[arg(short = 'o', long, global = true)]
    output_dir: Option<PathBuf>,

    /// Verbose output
    #[arg(short = 'v', long, global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Run microphone capture test
    MicCapture,
    /// Run VAD microphone test
    VadMic,
    /// Run all available tests
    All,
    /// List available audio devices
    ListDevices,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::MicCapture => run_single_test(&cli, TestType::MicCapture).await,
        Commands::VadMic => run_single_test(&cli, TestType::VadMic).await,
        Commands::All => run_all_tests(&cli).await,
        Commands::ListDevices => list_devices().await,
    }
}

enum TestType {
    MicCapture,
    VadMic,
}

async fn run_single_test(cli: &Cli, test_type: TestType) -> Result<(), Box<dyn std::error::Error>> {
    let context = create_test_context(cli);
    let results_dir = ensure_results_dir(cli.output_dir.as_deref())?;

    if cli.verbose {
        println!("Starting {} test...", get_test_name(&test_type));
        println!("Device: {}", context.device.as_deref().unwrap_or("default"));
        println!("Duration: {}s", context.duration.as_secs());
        println!("Output directory: {}", results_dir.display());
        println!();
    }

    let result = match test_type {
        TestType::MicCapture => MicCaptureCheck::run(&context).await,
        TestType::VadMic => VadMicCheck::run(&context).await,
    };

    match result {
        Ok(test_result) => {
            // Save result to JSON file
            let result_path = write_result_json(&results_dir, &test_result)?;

            if cli.verbose {
                print_test_result(&test_result);
                println!("\nResult saved to: {}", result_path.display());
            } else {
                // Brief output for non-verbose mode
                let status = if test_result.pass { "PASS" } else { "FAIL" };
                println!(
                    "{}: {} - {}",
                    get_test_name(&test_type),
                    status,
                    test_result.notes.as_deref().unwrap_or("")
                );
            }
        }
        Err(e) => {
            eprintln!("Test failed: {}", e.message);
            std::process::exit(1);
        }
    }

    Ok(())
}

async fn run_all_tests(cli: &Cli) -> Result<(), Box<dyn std::error::Error>> {
    let context = create_test_context(cli);
    let results_dir = ensure_results_dir(cli.output_dir.as_deref())?;

    if cli.verbose {
        println!("Running all audio tests...");
        println!("Device: {}", context.device.as_deref().unwrap_or("default"));
        println!("Duration: {}s", context.duration.as_secs());
        println!("Output directory: {}", results_dir.display());
        println!();
    }

    let tests = vec![
        (TestType::MicCapture, "Microphone Capture"),
        (TestType::VadMic, "VAD Microphone"),
    ];

    let mut all_passed = true;
    let mut results = Vec::new();

    for (test_type, display_name) in tests {
        if cli.verbose {
            println!("Running {} test...", display_name);
        }

        let result = match test_type {
            TestType::MicCapture => MicCaptureCheck::run(&context).await,
            TestType::VadMic => VadMicCheck::run(&context).await,
        };

        match result {
            Ok(test_result) => {
                let status = if test_result.pass { "PASS" } else { "FAIL" };
                if !test_result.pass {
                    all_passed = false;
                }

                // Save individual result
                let result_path = write_result_json(&results_dir, &test_result)?;
                results.push((display_name, test_result, result_path));

                println!("{}: {}", display_name, status);
            }
            Err(e) => {
                all_passed = false;
                if cli.verbose {
                    println!("{}: FAIL - {}", display_name, e.message);
                } else {
                    println!("{}: FAIL", display_name);
                }
            }
        }
    }

    if cli.verbose {
        println!("\n=== SUMMARY ===");
        for (_name, result, path) in &results {
            print_test_result(result);
            println!("Saved to: {}", path.display());
            println!();
        }
    }

    println!(
        "\nOverall result: {}",
        if all_passed {
            "ALL TESTS PASSED"
        } else {
            "SOME TESTS FAILED"
        }
    );

    if !all_passed {
        std::process::exit(1);
    }

    Ok(())
}

async fn list_devices() -> Result<(), Box<dyn std::error::Error>> {
    use coldvox_audio::device::DeviceManager;
    use cpal::traits::{DeviceTrait, HostTrait};

    println!("Checking audio setup for PipeWire compatibility...");
    let device_manager =
        DeviceManager::new().map_err(|e| format!("Failed to create DeviceManager: {}", e))?;
    if let Err(e) = device_manager.check_audio_setup() {
        eprintln!("Audio setup check failed: {}", e);
    }
    println!("Audio setup check complete.\n");

    println!("Available audio devices:");
    println!();

    let host = cpal::default_host();

    // List input devices
    println!("Input Devices:");
    match host.input_devices() {
        Ok(devices) => {
            for device in devices {
                match device.name() {
                    Ok(name) => println!("  - {}", name),
                    Err(e) => println!("  - (unnamed device: {})", e),
                }
            }
        }
        Err(e) => {
            println!("  Error listing input devices: {}", e);
        }
    }

    // Show default input device
    println!();
    if let Some(device) = host.default_input_device() {
        match device.name() {
            Ok(name) => println!("Default input device: {}", name),
            Err(e) => println!("Default input device: (error getting name: {})", e),
        }
    } else {
        println!("No default input device found");
    }

    Ok(())
}

fn create_test_context(cli: &Cli) -> TestContext {
    TestContext {
        device: cli.device.clone(),
        duration: Duration::from_secs(cli.duration),
        thresholds: None, // Could be extended to support custom thresholds
        output_dir: cli.output_dir.clone(),
    }
}

fn get_test_name(test_type: &TestType) -> &str {
    match test_type {
        TestType::MicCapture => "mic_capture",
        TestType::VadMic => "vad_mic",
    }
}

fn print_test_result(result: &LiveTestResult) {
    let status = if result.pass { "PASS" } else { "FAIL" };
    println!("Test: {}", result.test);
    println!("Status: {}", status);
    println!("Notes: {}", result.notes.as_deref().unwrap_or("None"));

    if !result.metrics.is_empty() {
        println!("Metrics:");
        for (key, value) in &result.metrics {
            println!("  {}: {}", key, value);
        }
    }
}
