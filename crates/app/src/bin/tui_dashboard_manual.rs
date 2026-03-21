//! TUI Dashboard - Manual CLI version (no proc-macros)
//!
//! This version uses clap's builder API instead of derive macros to avoid
//! proc-macro DLL issues with Windows App Control.

use coldvox_app::runtime::{self as app_runtime, ActivationMode};
use coldvox_vad::types::VadEvent;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame, Terminal,
};
use std::io;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

#[derive(Clone, Copy, Debug)]
enum CliActivationMode {
    Vad,
    Hotkey,
}

impl std::str::FromStr for CliActivationMode {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "vad" => Ok(CliActivationMode::Vad),
            "hotkey" => Ok(CliActivationMode::Hotkey),
            _ => Err(format!("Unknown mode: {}", s)),
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum DumpFormat {
    Pcm,
    Wav,
}

impl std::str::FromStr for DumpFormat {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "pcm" => Ok(DumpFormat::Pcm),
            "wav" => Ok(DumpFormat::Wav),
            _ => Err(format!("Unknown format: {}", s)),
        }
    }
}

struct Cli {
    device: Option<String>,
    activation_mode: CliActivationMode,
    resampler_quality: String,
    log_level: String,
    dump_audio: bool,
    dump_dir: Option<String>,
    dump_format: DumpFormat,
}

impl Default for Cli {
    fn default() -> Self {
        Self {
            device: None,
            activation_mode: CliActivationMode::Vad,
            resampler_quality: "balanced".to_string(),
            log_level: "info".to_string(),
            dump_audio: true,
            dump_dir: None,
            dump_format: DumpFormat::Pcm,
        }
    }
}

fn parse_args() -> Cli {
    let mut cli = Cli::default();
    let args: Vec<String> = std::env::args().collect();
    
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-D" | "--device" => {
                i += 1;
                if i < args.len() {
                    cli.device = Some(args[i].clone());
                }
            }
            "--activation-mode" => {
                i += 1;
                if i < args.len() {
                    match args[i].parse() {
                        Ok(mode) => cli.activation_mode = mode,
                        Err(e) => eprintln!("Warning: {}", e),
                    }
                }
            }
            "--resampler-quality" => {
                i += 1;
                if i < args.len() {
                    cli.resampler_quality = args[i].clone();
                }
            }
            "--log-level" => {
                i += 1;
                if i < args.len() {
                    cli.log_level = args[i].clone();
                }
            }
            "--dump-audio" => {
                i += 1;
                if i < args.len() && !args[i].starts_with("-") {
                    cli.dump_audio = args[i].parse().unwrap_or(true);
                } else {
                    cli.dump_audio = true;
                    continue; // Don't increment i, this arg doesn't have a value
                }
            }
            "--no-dump-audio" => {
                cli.dump_audio = false;
            }
            "--dump-dir" => {
                i += 1;
                if i < args.len() {
                    cli.dump_dir = Some(args[i].clone());
                }
            }
            "--dump-format" => {
                i += 1;
                if i < args.len() {
                    match args[i].parse() {
                        Ok(fmt) => cli.dump_format = fmt,
                        Err(e) => eprintln!("Warning: {}", e),
                    }
                }
            }
            "-h" | "--help" => {
                print_help();
                std::process::exit(0);
            }
            "-V" | "--version" => {
                println!("tui_dashboard 0.1.0");
                std::process::exit(0);
            }
            _ => {
                if args[i].starts_with("-") {
                    eprintln!("Warning: Unknown flag: {}", args[i]);
                }
            }
        }
        i += 1;
    }
    
    cli
}

fn print_help() {
    println!("TUI Dashboard with real-time audio monitoring");
    println!();
    println!("Usage: tui_dashboard_manual [OPTIONS]");
    println!();
    println!("Options:");
    println!("  -D, --device <DEVICE>              Audio device name");
    println!("      --activation-mode <MODE>       Activation mode: vad or hotkey [default: vad]");
    println!("      --resampler-quality <QUALITY>  Resampler quality: fast, balanced, quality [default: balanced]");
    println!("      --log-level <LEVEL>            Log level filter [default: info]");
    println!("      --dump-audio [BOOL]            Enable/disable audio dump [default: true]");
    println!("      --no-dump-audio                Disable audio dump");
    println!("      --dump-dir <DIR>               Directory to save audio dumps");
    println!("      --dump-format <FORMAT>         Dump format: pcm or wav [default: pcm]");
    println!("  -h, --help                         Print help");
    println!("  -V, --version                      Print version");
}

#[allow(dead_code)]
enum AppEvent {
    Log(LogLevel, String),
    Vad(VadEvent),
}

#[derive(Clone, Debug)]
enum LogLevel {
    Info,
    Success,
    Warning,
    Error,
}

struct DashboardState {
    is_running: bool,
    start_time: Instant,
    logs: Vec<(Instant, LogLevel, String)>,
    audio_frames: u64,
    vad_events: u64,
    last_vad_event: Option<String>,
    current_level: u8,
}

impl Default for DashboardState {
    fn default() -> Self {
        Self {
            is_running: false,
            start_time: Instant::now(),
            logs: vec![(Instant::now(), LogLevel::Info, "Dashboard started. Press 'S' to start pipeline, 'Q' to quit.".to_string())],
            audio_frames: 0,
            vad_events: 0,
            last_vad_event: None,
            current_level: 0,
        }
    }
}

fn draw_ui(f: &mut Frame, state: &DashboardState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(6),
        ])
        .split(f.area());

    // Status bar
    let status = if state.is_running { "RUNNING" } else { "STOPPED" };
    let status_color = if state.is_running { Color::Green } else { Color::Gray };
    let elapsed = state.start_time.elapsed().as_secs();
    
    let status_text = vec![
        Line::from(vec![
            Span::raw("Status: "),
            Span::styled(status, Style::default().fg(status_color)),
            Span::raw(format!(" | Runtime: {}s", elapsed)),
        ]),
        Line::from(format!("Audio Frames: {} | VAD Events: {}", state.audio_frames, state.vad_events)),
    ];
    
    let status_widget = Paragraph::new(status_text)
        .block(Block::default().title("Status").borders(Borders::ALL));
    f.render_widget(status_widget, chunks[0]);

    // Logs area
    let log_lines: Vec<Line> = state.logs.iter()
        .rev()
        .take(chunks[1].height as usize - 2)
        .rev()
        .map(|(_, level, msg)| {
            let color = match level {
                LogLevel::Info => Color::White,
                LogLevel::Success => Color::Green,
                LogLevel::Warning => Color::Yellow,
                LogLevel::Error => Color::Red,
            };
            Line::from(Span::styled(msg.clone(), Style::default().fg(color)))
        })
        .collect();
    
    let logs_widget = Paragraph::new(log_lines)
        .block(Block::default().title("Logs").borders(Borders::ALL));
    f.render_widget(logs_widget, chunks[1]);

    // Controls
    let controls = vec![
        Line::from("Controls:"),
        Line::from("[S] Start  [Q] Quit"),
        Line::from(format!("Last VAD: {}", state.last_vad_event.as_deref().unwrap_or("None"))),
    ];
    let controls_widget = Paragraph::new(controls)
        .block(Block::default().title("Controls").borders(Borders::ALL));
    f.render_widget(controls_widget, chunks[2]);
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = parse_args();
    
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let (tx, mut rx) = mpsc::channel(100);
    let mut state = DashboardState::default();

    let res = run_app(&mut terminal, &mut state, tx, rx, cli).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("Error: {}", err);
    }

    Ok(())
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    state: &mut DashboardState,
    tx: mpsc::Sender<AppEvent>,
    mut rx: mpsc::Receiver<AppEvent>,
    cli: Cli,
) -> io::Result<()> {
    let mut ui_update_interval = tokio::time::interval(Duration::from_millis(100));
    let mut app_handle: Option<app_runtime::AppHandle> = None;

    loop {
        terminal.draw(|f| draw_ui(f, state))?;

        tokio::select! {
            Some(event) = async {
                if event::poll(Duration::from_millis(50)).unwrap_or(false) {
                    event::read().ok()
                } else {
                    None
                }
            } => {
                if let Event::Key(key) = event {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Char('Q') => {
                            if let Some(app) = app_handle.take() {
                                let _ = Arc::new(app).shutdown().await;
                            }
                            return Ok(());
                        }
                        KeyCode::Char('s') | KeyCode::Char('S') => {
                            if !state.is_running {
                                state.logs.push((Instant::now(), LogLevel::Info, "Starting audio pipeline...".to_string()));
                                
                                let opts = app_runtime::AppRuntimeOptions {
                                    device: cli.device.clone(),
                                    activation_mode: match cli.activation_mode {
                                        CliActivationMode::Vad => ActivationMode::Vad,
                                        CliActivationMode::Hotkey => ActivationMode::Hotkey,
                                    },
                                    resampler_quality: match cli.resampler_quality.to_lowercase().as_str() {
                                        "fast" => coldvox_audio::ResamplerQuality::Fast,
                                        "quality" => coldvox_audio::ResamplerQuality::Quality,
                                        _ => coldvox_audio::ResamplerQuality::Balanced,
                                    },
                                    stt_selection: None, // Disable STT for simplicity
                                    enable_device_monitor: false,
                                    capture_buffer_samples: 65_536,
                                    ..Default::default()
                                };

                                match app_runtime::start(opts).await {
                                    Ok(app) => {
                                        // Subscribe to VAD events
                                        let mut vad_rx = app.subscribe_vad();
                                        let ui_tx = tx.clone();
                                        tokio::spawn(async move {
                                            while let Ok(ev) = vad_rx.recv().await {
                                                let _ = ui_tx.send(AppEvent::Vad(ev)).await;
                                            }
                                        });

                                        // Subscribe to audio
                                        let mut audio_rx = app.subscribe_audio();
                                        let ui_tx2 = tx.clone();
                                        tokio::spawn(async move {
                                            loop {
                                                match audio_rx.recv().await {
                                                    Ok(_) => {
                                                        let _ = ui_tx2.send(AppEvent::Log(LogLevel::Info, "Audio frame".to_string())).await;
                                                    }
                                                    Err(_) => break,
                                                }
                                            }
                                        });

                                        app_handle = Some(app);
                                        state.is_running = true;
                                        state.logs.push((Instant::now(), LogLevel::Success, "Pipeline started!".to_string()));
                                    }
                                    Err(e) => {
                                        state.logs.push((Instant::now(), LogLevel::Error, format!("Failed to start: {}", e)));
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }

            Some(event) = rx.recv() => {
                match event {
                    AppEvent::Log(level, msg) => {
                        state.logs.push((Instant::now(), level, msg));
                    }
                    AppEvent::Vad(vad_event) => {
                        state.vad_events += 1;
                        match vad_event {
                            VadEvent::SpeechStart { timestamp_ms, energy_db } => {
                                state.last_vad_event = Some(format!("Speech START @ {}ms ({:.1}dB)", timestamp_ms, energy_db));
                                state.logs.push((Instant::now(), LogLevel::Success, "Speech detected!".to_string()));
                            }
                            VadEvent::SpeechEnd { timestamp_ms, duration_ms, energy_db } => {
                                state.last_vad_event = Some(format!("Speech END @ {}ms ({}ms)", timestamp_ms, duration_ms));
                                state.logs.push((Instant::now(), LogLevel::Info, format!("Speech ended, {}ms", duration_ms)));
                            }
                        }
                    }
                }
            }

            _ = ui_update_interval.tick() => {
                if state.is_running {
                    if let Some(ref app) = app_handle {
                        let m = &app.metrics;
                        state.audio_frames = m.capture_frames.load(Ordering::Relaxed);
                    }
                }
            }
        }
    }
}
