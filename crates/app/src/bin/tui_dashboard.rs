use clap::Parser;
use coldvox_app::audio::capture::AudioCaptureThread;
use coldvox_app::audio::chunker::{AudioChunker, ChunkerConfig};
use coldvox_app::audio::frame_reader::FrameReader;
use coldvox_app::audio::ring_buffer::AudioRingBuffer;
use coldvox_app::audio::vad_processor::{AudioFrame as VadFrame, VadProcessor};
use coldvox_app::foundation::error::AudioConfig;
use coldvox_app::telemetry::pipeline_metrics::{PipelineMetrics, PipelineStage};
use coldvox_app::vad::config::{UnifiedVadConfig, VadMode};
use coldvox_app::vad::types::VadEvent;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph, Sparkline},
    Frame, Terminal,
};
use std::collections::VecDeque;
use std::io;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{broadcast, mpsc};
use tokio::task::JoinHandle;

#[derive(Parser)]
#[command(author, version, about = "TUI Dashboard with real-time audio monitoring")]
struct Cli {
    /// Audio device name
    #[arg(short = 'D', long)]
    device: Option<String>,
}

enum AppEvent {
    Log(LogLevel, String),
    Vad(VadEvent),
    UpdateMetrics(PipelineMetricsSnapshot),
    PipelineStarted,
    PipelineStopped,
}

struct PipelineMetricsSnapshot {
    current_rms: u64,
    current_peak: i16,
    audio_level_db: i16,
    capture_fps: u64,
    chunker_fps: u64,
    vad_fps: u64,
    capture_buffer_fill: usize,
    chunker_buffer_fill: usize,
    vad_buffer_fill: usize,
    stage_capture: bool,
    stage_chunker: bool,
    stage_vad: bool,
    stage_output: bool,
}

struct DashboardState {
    level_history: VecDeque<u8>,
    peak_history: VecDeque<u8>,
    is_speaking: bool,
    speech_segments: u64,
    last_vad_event: Option<String>,
    is_running: bool,
    selected_device: String,
    start_time: Instant,
    vad_frames: u64,
    logs: VecDeque<LogEntry>,
    pipeline_handle: Option<JoinHandle<()>>,
    metrics: PipelineMetricsSnapshot,
}

#[derive(Clone)]
struct LogEntry {
    timestamp: Instant,
    level: LogLevel,
    message: String,
}

#[derive(Clone, Debug)]
#[allow(dead_code)] // Allow unused variants for now
enum LogLevel {
    Info,
    Success,
    Warning,
    Error,
    Debug,
}

impl Default for DashboardState {
    fn default() -> Self {
        let mut level_history = VecDeque::with_capacity(60);
        let mut peak_history = VecDeque::with_capacity(60);
        for _ in 0..60 {
            level_history.push_back(0);
            peak_history.push_back(0);
        }

        let mut logs = VecDeque::new();
        logs.push_back(LogEntry {
            timestamp: Instant::now(),
            level: LogLevel::Info,
            message: "Dashboard started. Press 'S' to start pipeline, 'Q' to quit.".to_string(),
        });

        Self {
            level_history,
            peak_history,
            is_speaking: false,
            speech_segments: 0,
            last_vad_event: None,
            is_running: false,
            selected_device: "default".to_string(),
            start_time: Instant::now(),
            vad_frames: 0,
            logs,
            pipeline_handle: None,
            metrics: PipelineMetricsSnapshot {
                current_rms: 0,
                current_peak: 0,
                audio_level_db: -900,  // -90.0 dB * 10
                capture_fps: 0,
                chunker_fps: 0,
                vad_fps: 0,
                capture_buffer_fill: 0,
                chunker_buffer_fill: 0,
                vad_buffer_fill: 0,
                stage_capture: false,
                stage_chunker: false,
                stage_vad: false,
                stage_output: false,
            },
        }
    }
}

impl DashboardState {
    fn log(&mut self, level: LogLevel, message: String) {
        self.logs.push_back(LogEntry {
            timestamp: Instant::now(),
            level,
            message,
        });

        while self.logs.len() > 100 {
            self.logs.pop_front();
        }
    }

    fn update_level_history(&mut self) {
        let rms = self.metrics.current_rms;
        let level = ((rms as f64 / 32768.0) * 100.0).min(100.0) as u8;

        let peak = self.metrics.current_peak;
        let peak_level = ((peak as f64 / 32768.0) * 100.0).min(100.0) as u8;

        self.level_history.pop_front();
        self.level_history.push_back(level);

        self.peak_history.pop_front();
        self.peak_history.push_back(peak_level);
    }

    fn reset_metrics(&mut self) {
        self.vad_frames = 0;
        self.speech_segments = 0;
        self.log(LogLevel::Info, "Metrics reset".to_string());
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let (tx, rx) = mpsc::channel(100);

    let mut state = DashboardState::default();
    if let Some(device) = cli.device {
        state.selected_device = device;
    }

    let res = run_app(&mut terminal, &mut state, tx, rx).await;

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
) -> io::Result<()> {
    let mut ui_update_interval = tokio::time::interval(Duration::from_millis(50));

    loop {
        terminal.draw(|f| draw_ui(f, state))?;

        tokio::select! {
            Some(event) = async {
                if event::poll(Duration::from_millis(10)).unwrap_or(false) {
                    event::read().ok()
                } else {
                    None
                }
            } => {
                if let Event::Key(key) = event {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Char('Q') => {
                            if state.is_running {
                                if let Some(handle) = state.pipeline_handle.take() {
                                    handle.abort();
                                }
                            }
                            return Ok(());
                        }
                        KeyCode::Char('s') | KeyCode::Char('S') => {
                            if !state.is_running {
                                state.log(LogLevel::Info, "Starting audio pipeline...".to_string());
                                let pipeline_tx = tx.clone();
                                let device = state.selected_device.clone();
                                state.pipeline_handle = Some(tokio::spawn(run_audio_pipeline(pipeline_tx, device)));
                            }
                        }
                        KeyCode::Char('r') | KeyCode::Char('R') => {
                            state.reset_metrics();
                        }
                        _ => {}
                    }
                }
            }

            Some(event) = rx.recv() => {
                match event {
                    AppEvent::Log(level, msg) => state.log(level, msg),
                    AppEvent::Vad(vad_event) => {
                        state.vad_frames += 1;
                        match vad_event {
                            VadEvent::SpeechStart { timestamp_ms, energy_db } => {
                                state.is_speaking = true;
                                state.speech_segments += 1;
                                state.last_vad_event = Some(format!("Speech START @ {}ms ({:.1}dB)", timestamp_ms, energy_db));
                                state.log(LogLevel::Success, format!("Speech detected @ {}ms", timestamp_ms));
                            }
                            VadEvent::SpeechEnd { timestamp_ms, duration_ms, energy_db } => {
                                state.is_speaking = false;
                                state.last_vad_event = Some(format!("Speech END @ {}ms ({}ms, {:.1}dB)", timestamp_ms, duration_ms, energy_db));
                                state.log(LogLevel::Info, format!("Speech ended, duration: {}ms", duration_ms));
                            }
                        }
                    }
                    AppEvent::UpdateMetrics(snapshot) => {
                        state.metrics = snapshot;
                    }
                    AppEvent::PipelineStarted => {
                        state.is_running = true;
                        state.log(LogLevel::Success, "Pipeline fully started".to_string());
                    }
                    AppEvent::PipelineStopped => {
                        state.is_running = false;
                        state.pipeline_handle = None;
                        state.log(LogLevel::Success, "Pipeline stopped".to_string());
                    }
                }
            }

            _ = ui_update_interval.tick() => {
                if state.is_running {
                    state.update_level_history();
                }
            }
        }
    }
}

async fn run_audio_pipeline(tx: mpsc::Sender<AppEvent>, device: String) {
    let metrics = Arc::new(PipelineMetrics::default());

    // Convert "default" device to None for proper OS default selection
    let device_option = if device == "default" || device.is_empty() {
        None
    } else {
        Some(device)
    };

    let audio_config = AudioConfig::default();
    // Set up ring buffer for capture -> processing
    let rb = AudioRingBuffer::new(16_384);
    let (audio_producer, audio_consumer) = rb.split();
    let (audio_thread, sample_rate) = match AudioCaptureThread::spawn(audio_config, audio_producer, device_option) {
        Ok(thread_tuple) => thread_tuple,
        Err(e) => {
            let _ = tx.send(AppEvent::Log(LogLevel::Error, format!("Failed to create audio thread: {}", e))).await;
            let _ = tx.send(AppEvent::PipelineStopped).await;
            return;
        }
    };

    let _ = tx.send(AppEvent::Log(LogLevel::Success, "Audio capture started".to_string())).await;

    // Broadcast channel for audio frames
    let (audio_tx, _) = broadcast::channel::<VadFrame>(200);
    let (event_tx, mut event_rx) = mpsc::channel(200);

    let chunker_cfg = ChunkerConfig {
        frame_size_samples: 512,
        sample_rate_hz: sample_rate,
    };
    // Build FrameReader from ring buffer consumer and feed it to the chunker
    let frame_reader = FrameReader::new(audio_consumer, sample_rate);
    let chunker = AudioChunker::new(frame_reader, audio_tx.clone(), chunker_cfg).with_metrics(metrics.clone());
    let _chunker_handle = chunker.spawn();

    let vad_cfg = UnifiedVadConfig {
        mode: VadMode::Silero,
        frame_size_samples: 512,
        sample_rate_hz: 16000,  // Silero VAD requires 16kHz - resampler will handle conversion
        ..Default::default()
    };

    let vad_audio_rx = audio_tx.subscribe();
    let _vad_thread = match VadProcessor::spawn(vad_cfg, vad_audio_rx, event_tx) {
        Ok(h) => h,
        Err(e) => {
            let _ = tx.send(AppEvent::Log(LogLevel::Error, format!("Failed to spawn VAD: {}", e))).await;
            let _ = tx.send(AppEvent::PipelineStopped).await;
            return;
        }
    };

    let _ = tx.send(AppEvent::PipelineStarted).await;

    let mut metrics_update_interval = tokio::time::interval(Duration::from_millis(100));

    loop {
        tokio::select! {
            Some(event) = event_rx.recv() => {
                metrics.mark_stage_active(PipelineStage::Vad);
                metrics.mark_stage_active(PipelineStage::Output);
                if tx.send(AppEvent::Vad(event)).await.is_err() {
                    break;
                }
            }
            _ = metrics_update_interval.tick() => {
                let snapshot = PipelineMetricsSnapshot {
                    current_rms: metrics.current_rms.load(Ordering::Relaxed),
                    current_peak: metrics.current_peak.load(Ordering::Relaxed),
                    audio_level_db: metrics.audio_level_db.load(Ordering::Relaxed),
                    capture_fps: metrics.capture_fps.load(Ordering::Relaxed),
                    chunker_fps: metrics.chunker_fps.load(Ordering::Relaxed),
                    vad_fps: metrics.vad_fps.load(Ordering::Relaxed),
                    capture_buffer_fill: metrics.capture_buffer_fill.load(Ordering::Relaxed),
                    chunker_buffer_fill: metrics.chunker_buffer_fill.load(Ordering::Relaxed),
                    vad_buffer_fill: metrics.vad_buffer_fill.load(Ordering::Relaxed),
                    stage_capture: metrics.stage_capture.load(Ordering::Relaxed),
                    stage_chunker: metrics.stage_chunker.load(Ordering::Relaxed),
                    stage_vad: metrics.stage_vad.load(Ordering::Relaxed),
                    stage_output: metrics.stage_output.load(Ordering::Relaxed),
                };
                if tx.send(AppEvent::UpdateMetrics(snapshot)).await.is_err() {
                    break;
                }
                metrics.decay_stages();
            }
            else => { break; }
        }
    }

    let _ = tx.send(AppEvent::Log(LogLevel::Info, "Stopping pipeline...".to_string())).await;
    // Stop audio thread
    audio_thread.stop();

    let _ = tx.send(AppEvent::PipelineStopped).await;
}

fn draw_ui(f: &mut Frame, state: &DashboardState) {
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(12),
            Constraint::Min(10),
            Constraint::Length(8),
        ])
        .split(f.size());

    let top_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(60),
            Constraint::Percentage(40),
        ])
        .split(main_chunks[0]);

    draw_audio_levels(f, top_chunks[0], state);
    draw_pipeline_flow(f, top_chunks[1], state);

    let middle_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(main_chunks[1]);

    draw_metrics(f, middle_chunks[0], state);
    draw_status(f, middle_chunks[1], state);

    draw_logs(f, main_chunks[2], state);
}

fn draw_audio_levels(f: &mut Frame, area: Rect, state: &DashboardState) {
    let block = Block::default()
        .title("Audio Levels")
        .borders(Borders::ALL);

    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Min(4),
        ])
        .split(inner);

    let db = state.metrics.audio_level_db as f64 / 10.0;
    let level_percent = ((db + 90.0) / 90.0 * 100.0).clamp(0.0, 100.0) as u16;

    let gauge = Gauge::default()
        .block(Block::default().title("Level"))
        .gauge_style(
            if level_percent > 80 {
                Style::default().fg(Color::Red)
            } else if level_percent > 60 {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::Green)
            }
        )
        .percent(level_percent)
        .label(format!("{:.1} dB", db));
    f.render_widget(gauge, chunks[0]);

    let rms_scaled = state.metrics.current_rms as f64 / 1000.0; // stored as RMS*1000
    let rms_db = if rms_scaled > 0.0 {
        20.0 * (rms_scaled / 32767.0).log10()
    } else { -90.0 };
    let peak = state.metrics.current_peak as f64;
    let peak_db = if peak > 0.0 {
        20.0 * (peak / 32767.0).log10()
    } else { -90.0 };
    
    let db_text = Paragraph::new(format!("Peak: {:.1} dB | RMS: {:.1} dB", peak_db, rms_db))
        .alignment(Alignment::Center);
    f.render_widget(db_text, chunks[1]);

    let sparkline_data: Vec<u64> = state.level_history.iter()
        .map(|&v| v as u64)
        .collect();

    let sparkline = Sparkline::default()
        .block(Block::default().title("History (60 samples)"))
        .data(&sparkline_data)
        .style(Style::default().fg(Color::Cyan))
        .max(100);
    f.render_widget(sparkline, chunks[2]);
}

fn draw_pipeline_flow(f: &mut Frame, area: Rect, state: &DashboardState) {
    let block = Block::default()
        .title("Pipeline Flow")
        .borders(Borders::ALL);

    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(2),
            Constraint::Length(2),
            Constraint::Length(2),
        ])
        .split(inner);

    let stages = [
        ("1. Capture", state.metrics.stage_capture, 0),
        ("2. Chunker", state.metrics.stage_chunker, 0),
        ("3. VAD", state.metrics.stage_vad, state.vad_frames),
        ("4. Output", state.metrics.stage_output, state.speech_segments),
    ];

    for (i, (name, active, count)) in stages.iter().enumerate() {
        let color = if *active {
            Color::Green
        } else if state.is_running {
            Color::Gray
        } else {
            Color::DarkGray
        };

        let indicator = if *active { "●" } else { "○" };
        let text = format!("{} {} [{} events]", indicator, name, count);

        let paragraph = Paragraph::new(text)
            .style(Style::default().fg(color));
        f.render_widget(paragraph, chunks[i]);
    }
}

fn draw_metrics(f: &mut Frame, area: Rect, state: &DashboardState) {
    let block = Block::default()
        .title("Metrics")
        .borders(Borders::ALL);

    let inner = block.inner(area);
    f.render_widget(block, area);

    let elapsed = state.start_time.elapsed().as_secs();
    let metrics_text = vec![
        Line::from(format!("Runtime: {}s", elapsed)),
        Line::from(""),
        Line::from(format!("Capture FPS: {:.1}", state.metrics.capture_fps as f64 / 10.0)),
        Line::from(format!("Chunker FPS: {:.1}", state.metrics.chunker_fps as f64 / 10.0)),
        Line::from(format!("VAD FPS: {:.1}", state.metrics.vad_fps as f64 / 10.0)),
        Line::from(""),
        Line::from("Buffer Fill:"),
        Line::from(format!("  Capture: {}%", state.metrics.capture_buffer_fill)),
        Line::from(format!("  Chunker: {}%", state.metrics.chunker_buffer_fill)),
        Line::from(format!("  VAD: {}%", state.metrics.vad_buffer_fill)),
    ];

    let paragraph = Paragraph::new(metrics_text);
    f.render_widget(paragraph, inner);
}

fn draw_status(f: &mut Frame, area: Rect, state: &DashboardState) {
    let block = Block::default()
        .title("Status & VAD")
        .borders(Borders::ALL);

    let inner = block.inner(area);
    f.render_widget(block, area);

    let status_color = if state.is_running {
        if state.is_speaking {
            Color::Yellow
        } else {
            Color::Green
        }
    } else {
        Color::Gray
    };

    let status_text = vec![
        Line::from(vec![
            Span::raw("Pipeline: "),
            Span::styled(
                if state.is_running { "RUNNING" } else { "STOPPED" },
                Style::default().fg(status_color).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(format!("Device: {}", state.selected_device)),
        Line::from(""),
        Line::from(vec![
            Span::raw("Speaking: "),
            Span::styled(
                if state.is_speaking { "YES" } else { "NO" },
                Style::default().fg(if state.is_speaking { Color::Green } else { Color::Gray }),
            ),
        ]),
        Line::from(format!("Speech Segments: {}", state.speech_segments)),
        Line::from(""),
        Line::from("Last VAD Event:"),
        Line::from(state.last_vad_event.as_deref().unwrap_or("None")),
        Line::from(""),
        Line::from("Controls:"),
        Line::from("[S] Start  [R] Reset  [Q] Quit"),
    ];

    let paragraph = Paragraph::new(status_text);
    f.render_widget(paragraph, inner);
}

fn draw_logs(f: &mut Frame, area: Rect, state: &DashboardState) {
    let block = Block::default()
        .title("Logs")
        .borders(Borders::ALL);

    let inner = block.inner(area);
    f.render_widget(block, area);

    let start_time = state.logs.front()
        .map(|e| e.timestamp)
        .unwrap_or_else(Instant::now);

    let log_lines: Vec<Line> = state.logs.iter()
        .rev()
        .take(inner.height as usize)
        .rev()
        .map(|entry| {
            let elapsed = entry.timestamp.duration_since(start_time).as_secs_f64();
            let color = match entry.level {
                LogLevel::Info => Color::White,
                LogLevel::Success => Color::Green,
                LogLevel::Warning => Color::Yellow,
                LogLevel::Error => Color::Red,
                LogLevel::Debug => Color::Cyan,
            };

            Line::from(vec![
                Span::styled(
                    format!("[{:7.2}s] ", elapsed),
                    Style::default().fg(Color::Gray),
                ),
                Span::styled(
                    &entry.message,
                    Style::default().fg(color),
                ),
            ])
        })
        .collect();

    let paragraph = Paragraph::new(log_lines);
    f.render_widget(paragraph, inner);
}