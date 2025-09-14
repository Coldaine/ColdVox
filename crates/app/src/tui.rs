//! TUI Dashboard for ColdVox
//!
//! This module provides a terminal user interface for monitoring and controlling
//! the ColdVox audio pipeline, including STT plugin management.

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
use tokio::sync::mpsc;
// Reuse global tracing subscriber initialized in `main.rs`.

#[cfg(feature = "vosk")]
use crate::stt::TranscriptionEvent;
use crate::runtime::ActivationMode;
use coldvox_vad::types::VadEvent;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Tab {
    Audio,
    Logs,
    Plugins,
}

#[allow(dead_code)]
enum AppEvent {
    Log(LogLevel, String),
    Vad(VadEvent),
    /// Internal control signal: runtime replaced (after restart)
    AppReplaced(std::sync::Arc<crate::runtime::AppHandle>),
    #[cfg(feature = "vosk")]
    Transcription(TranscriptionEvent),
    PluginLoad(String),
    PluginUnload(String),
    PluginSwitch(String),
    PluginStatusUpdate,
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
    capture_frames: u64,
    chunker_frames: u64,
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
    app: Option<std::sync::Arc<crate::runtime::AppHandle>>,
    activation_mode: ActivationMode,
    metrics: PipelineMetricsSnapshot,
    has_metrics_snapshot: bool,
    current_tab: Tab,
    /// Last final transcript (if STT enabled)
    #[cfg(feature = "vosk")]
    last_transcript: Option<String>,

    #[cfg(feature = "vosk")]
    plugin_manager: Option<Arc<tokio::sync::RwLock<crate::stt::plugin_manager::SttPluginManager>>>,

    #[cfg(feature = "vosk")]
    plugin_current: Option<String>,

    #[cfg(feature = "vosk")]
    plugin_active_count: usize,

    #[cfg(feature = "vosk")]
    plugin_transcription_requests: u64,

    #[cfg(feature = "vosk")]
    plugin_success: u64,

    #[cfg(feature = "vosk")]
    plugin_failures: u64,
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
            app: None,
            activation_mode: ActivationMode::Vad,
            metrics: PipelineMetricsSnapshot {
                current_rms: 0,
                current_peak: 0,
                audio_level_db: -900, // -90.0 dB * 10
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
                capture_frames: 0,
                chunker_frames: 0,
            },
            has_metrics_snapshot: false,
            current_tab: Tab::Audio,
            #[cfg(feature = "vosk")]
            last_transcript: None,
            #[cfg(feature = "vosk")]
            plugin_manager: None,
            #[cfg(feature = "vosk")]
            plugin_current: None,
            #[cfg(feature = "vosk")]
            plugin_active_count: 0,
            #[cfg(feature = "vosk")]
            plugin_transcription_requests: 0,
            #[cfg(feature = "vosk")]
            plugin_success: 0,
            #[cfg(feature = "vosk")]
            plugin_failures: 0,
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
        // current_rms is stored as RMS * 1000
        let rms = self.metrics.current_rms as f64 / 1000.0;
        let level = ((rms / 32768.0) * 100.0).min(100.0) as u8;

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

    fn toggle_activation_mode(&mut self) {
        self.activation_mode = match self.activation_mode {
            ActivationMode::Vad => ActivationMode::Hotkey,
            ActivationMode::Hotkey => ActivationMode::Vad,
        };
        self.log(
            LogLevel::Info,
            format!(
                "Switched activation mode to {}",
                match self.activation_mode {
                    ActivationMode::Vad => "VAD",
                    ActivationMode::Hotkey => "Push-to-talk",
                }
            ),
        );
    }
}

/// Run the TUI dashboard with the given app handle
pub async fn run_tui(app: std::sync::Arc<crate::runtime::AppHandle>) -> Result<(), Box<dyn std::error::Error>> {
    // TUI runs in the same process as `main` which already initializes tracing.
    // Avoid re-initializing the global subscriber here (double-init causes errors
    // and creating another file appender+guard can interfere with the main guard
    // lifecycle and file flushing). Rely on the main subscriber and its guard.

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let (tx, rx) = mpsc::channel(100);

    let mut state = DashboardState::default();
    state.app = Some(app.clone());

    // Set up plugin manager reference if available
    #[cfg(feature = "vosk")]
    if let Some(ref app) = state.app {
        if let Some(ref pm) = app.plugin_manager {
            state.plugin_manager = Some(pm.clone());
        }
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
        eprintln!("TUI Error: {}", err);
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
                                if let Some(app) = state.app.take() {
                                    tokio::spawn(async move { app.shutdown().await; });
                                }
                            }
                            return Ok(());
                        }
                        KeyCode::Char('s') | KeyCode::Char('S') => {
                            if !state.is_running {
                                state.log(LogLevel::Info, "Starting audio pipeline...".to_string());
                                // Pipeline is already started, just mark as running
                                state.is_running = true;
                                state.log(LogLevel::Success, "Pipeline already running".to_string());
                            }
                        }
                        KeyCode::Char('a') | KeyCode::Char('A') => {
                            state.toggle_activation_mode();
                            if state.is_running {
                                if let Some(app) = &mut state.app {
                                    let new_mode = state.activation_mode;
                                    if let Err(e) = app.set_activation_mode(new_mode).await {
                                        state.log(LogLevel::Error, format!("Failed to set activation mode: {}", e));
                                    } else {
                                        state.log(LogLevel::Info, "Activation mode updated".to_string());
                                    }
                                }
                            }
                        }
                        KeyCode::Char('r') | KeyCode::Char('R') => {
                            state.reset_metrics();
                        }
                        KeyCode::Char('p') | KeyCode::Char('P') => {
                            state.current_tab = match state.current_tab {
                                Tab::Audio => Tab::Logs,
                                Tab::Logs => Tab::Plugins,
                                Tab::Plugins => Tab::Audio,
                            };
                            state.log(LogLevel::Info, format!("Switched to {:?} tab", state.current_tab));
                        }
                        KeyCode::Char('l') | KeyCode::Char('L') => {
                            #[cfg(feature = "vosk")]
                            {
                                if let Some(ref pm) = state.plugin_manager {
                                    let pm_clone = pm.clone();
                                    let tx_clone = tx.clone();
                                    tokio::spawn(async move {
                                        let result = pm_clone.write().await.switch_plugin("mock").await;
                                        let _ = tx_clone.send(AppEvent::PluginSwitch("mock".to_string())).await;
                                        if result.is_ok() {
                                            let _ = tx_clone.send(AppEvent::PluginLoad("mock".to_string())).await;
                                        }
                                    });
                                }
                            }
                            state.log(LogLevel::Info, "Loading plugin...".to_string());
                        }
                        KeyCode::Char('u') | KeyCode::Char('U') => {
                            #[cfg(feature = "vosk")]
                            {
                                if let Some(ref pm) = state.plugin_manager {
                                    let pm_clone = pm.clone();
                                    let tx_clone = tx.clone();
                                    tokio::spawn(async move {
                                        let _ = pm_clone.write().await.unload_plugin("mock").await;
                                        let _ = tx_clone.send(AppEvent::PluginUnload("mock".to_string())).await;
                                    });
                                }
                            }
                            state.log(LogLevel::Info, "Unloading plugin...".to_string());
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
                    AppEvent::AppReplaced(app) => {
                        state.app = Some(app);
                        state.is_running = true;
                    }
                        #[cfg(feature = "vosk")]
                        AppEvent::Transcription(tevent) => {
                            tracing::debug!(target: "coldvox::tui", transcription_event = ?tevent, "Received TranscriptionEvent");
                            match tevent.clone() {
                                TranscriptionEvent::Partial { utterance_id, text, .. } => {
                                    if !text.trim().is_empty() {
                                        state.log(LogLevel::Info, format!("[STT partial:{}] {}", utterance_id, text));
                                    } else {
                                        tracing::debug!(target: "coldvox::tui", utterance_id, "Partial transcript empty - likely NoOp plugin");
                                    }
                                }
                                TranscriptionEvent::Final { utterance_id, text, .. } => {
                                    if !text.trim().is_empty() {
                                        state.log(LogLevel::Success, format!("[STT final:{}] {}", utterance_id, text));
                                        state.last_transcript = Some(text.clone());
                                        tracing::info!(target: "coldvox::tui", utterance_id, text_len = text.len(), "Final transcript displayed in Status tab");
                                    } else {
                                        tracing::warn!(target: "coldvox::tui", utterance_id, "Final transcript empty - check STT plugin");
                                    }
                                }
                                TranscriptionEvent::Error { code, message } => {
                                    state.log(LogLevel::Error, format!("[STT error:{}] {}", code, message));
                                    tracing::error!(target: "coldvox::tui", code, %message, "STT error in TUI");
                                }
                            }
                        }
                    AppEvent::PluginLoad(plugin_id) => {
                        state.plugin_current = Some(plugin_id.clone());
                        state.plugin_active_count += 1;
                        state.log(LogLevel::Success, format!("Plugin loaded: {}", plugin_id));
                    }
                    AppEvent::PluginUnload(plugin_id) => {
                        if state.plugin_current.as_ref() == Some(&plugin_id) {
                            state.plugin_current = None;
                        }
                        if state.plugin_active_count > 0 {
                            state.plugin_active_count -= 1;
                        }
                        state.log(LogLevel::Info, format!("Plugin unloaded: {}", plugin_id));
                    }
                    AppEvent::PluginSwitch(plugin_id) => {
                        state.plugin_current = Some(plugin_id.clone());
                        state.log(LogLevel::Info, format!("Switched to plugin: {}", plugin_id));
                    }
                    AppEvent::PluginStatusUpdate => {
                        state.log(LogLevel::Debug, "Plugin status updated".to_string());
                    }
                }
            }

            _ = ui_update_interval.tick() => {
                if state.is_running {
                    if let Some(app) = &state.app {
                        let m = &app.metrics;
                        state.metrics = PipelineMetricsSnapshot {
                            current_rms: m.current_rms.load(Ordering::Relaxed),
                            current_peak: m.current_peak.load(Ordering::Relaxed),
                            audio_level_db: m.audio_level_db.load(Ordering::Relaxed),
                            capture_fps: m.capture_fps.load(Ordering::Relaxed),
                            chunker_fps: m.chunker_fps.load(Ordering::Relaxed),
                            vad_fps: m.vad_fps.load(Ordering::Relaxed),
                            capture_buffer_fill: m.capture_buffer_fill.load(Ordering::Relaxed),
                            chunker_buffer_fill: m.chunker_buffer_fill.load(Ordering::Relaxed),
                            vad_buffer_fill: m.vad_buffer_fill.load(Ordering::Relaxed),
                            stage_capture: m.stage_capture.load(Ordering::Relaxed),
                            stage_chunker: m.stage_chunker.load(Ordering::Relaxed),
                            stage_vad: m.stage_vad.load(Ordering::Relaxed),
                            stage_output: m.stage_output.load(Ordering::Relaxed),
                            capture_frames: m.capture_frames.load(Ordering::Relaxed),
                            chunker_frames: m.chunker_frames.load(Ordering::Relaxed),
                        };
                        state.has_metrics_snapshot = true;
                        state.update_level_history();
                    }
                }
            }
        }
    }
}

fn draw_ui(f: &mut Frame, state: &DashboardState) {
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(12),
            Constraint::Min(10),
            Constraint::Length(8),
        ])
        .split(f.area());

    let top_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(main_chunks[0]);

    draw_audio_levels(f, top_chunks[0], state);
    draw_pipeline_flow(f, top_chunks[1], state);

    let middle_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(main_chunks[1]);

    match state.current_tab {
        Tab::Audio => {
            draw_metrics(f, middle_chunks[0], state);
            draw_status(f, middle_chunks[1], state);
        }
        Tab::Logs => {
            draw_logs(f, middle_chunks[0], state);
            draw_status(f, middle_chunks[1], state);
        }
        Tab::Plugins => {
            draw_plugins(f, middle_chunks[0], state);
            draw_plugin_status(f, middle_chunks[1], state);
        }
    }

    draw_logs(f, main_chunks[2], state);
}

fn draw_audio_levels(f: &mut Frame, area: Rect, state: &DashboardState) {
    let block = Block::default().title("Audio Levels").borders(Borders::ALL);

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
        .gauge_style(if level_percent > 80 {
            Style::default().fg(Color::Red)
        } else if level_percent > 60 {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::Green)
        })
        .percent(level_percent)
        .label(format!("{:.1} dB", db));
    f.render_widget(gauge, chunks[0]);

    let rms_scaled = state.metrics.current_rms as f64 / 1000.0;
    let rms_db = if rms_scaled > 0.0 {
        20.0 * (rms_scaled / 32767.0).log10()
    } else {
        -90.0
    };
    let peak = state.metrics.current_peak as f64;
    let peak_db = if peak > 0.0 {
        20.0 * (peak / 32767.0).log10()
    } else {
        -90.0
    };

    let db_text = Paragraph::new(format!("Peak: {:.1} dB | RMS: {:.1} dB", peak_db, rms_db))
        .alignment(Alignment::Center);
    f.render_widget(db_text, chunks[1]);

    let sparkline_data: Vec<u64> = state.level_history.iter().map(|&v| v as u64).collect();

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
        ("1. Capture", state.metrics.stage_capture),
        ("2. Chunker", state.metrics.stage_chunker),
        ("3. VAD", state.metrics.stage_vad),
        ("4. Output", state.metrics.stage_output),
    ];

    for (i, (name, active)) in stages.iter().enumerate() {
        let color = if *active {
            Color::Green
        } else if state.is_running {
            Color::Gray
        } else {
            Color::DarkGray
        };

        let indicator = if *active { "●" } else { "○" };
        let count_text = match i {
            0 => {
                if state.has_metrics_snapshot {
                    format!("{} events", state.metrics.capture_frames)
                } else {
                    "N/A".to_string()
                }
            }
            1 => {
                if state.has_metrics_snapshot {
                    format!("{} events", state.metrics.chunker_frames)
                } else {
                    "N/A".to_string()
                }
            }
            2 => format!("{} events", state.vad_frames),
            3 => format!("{} events", state.speech_segments),
            _ => "".to_string(),
        };
        let text = format!("{} {} [{}]", indicator, name, count_text);

        let paragraph = Paragraph::new(text).style(Style::default().fg(color));
        f.render_widget(paragraph, chunks[i]);
    }
}

fn draw_metrics(f: &mut Frame, area: Rect, state: &DashboardState) {
    let block = Block::default().title("Metrics").borders(Borders::ALL);

    let inner = block.inner(area);
    f.render_widget(block, area);

    let elapsed = state.start_time.elapsed().as_secs();
    let metrics_text = vec![
        Line::from(format!("Runtime: {}s", elapsed)),
        Line::from(""),
        Line::from(format!(
            "Capture FPS: {:.1}",
            state.metrics.capture_fps as f64 / 10.0
        )),
        Line::from(format!(
            "Chunker FPS: {:.1}",
            state.metrics.chunker_fps as f64 / 10.0
        )),
        Line::from(format!(
            "VAD FPS: {:.1}",
            state.metrics.vad_fps as f64 / 10.0
        )),
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
    let block = Block::default().title("Status & VAD").borders(Borders::ALL);

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

    let mut status_text: Vec<Line> = Vec::new();
    status_text.push(Line::from(vec![
        Span::raw("Pipeline: "),
        Span::styled(
            if state.is_running {
                "RUNNING"
            } else {
                "STOPPED"
            },
            Style::default()
                .fg(status_color)
                .add_modifier(Modifier::BOLD),
        ),
    ]));
    status_text.push(Line::from(format!("Device: {}", state.selected_device)));
    status_text.push(Line::from(format!(
        "Activation: {}",
        match state.activation_mode {
            ActivationMode::Vad => "VAD",
            ActivationMode::Hotkey => "Push-to-talk",
        }
    )));
    status_text.push(Line::from(""));
    status_text.push(Line::from(vec![
        Span::raw("Speaking: "),
        Span::styled(
            if state.is_speaking { "YES" } else { "NO" },
            Style::default().fg(if state.is_speaking {
                Color::Green
            } else {
                Color::Gray
            }),
        ),
    ]));
    status_text.push(Line::from(format!(
        "Speech Segments: {}",
        state.speech_segments
    )));
    status_text.push(Line::from(""));
    status_text.push(Line::from("Last VAD Event:"));
    status_text.push(Line::from(
        state.last_vad_event.as_deref().unwrap_or("None"),
    ));
    #[cfg(feature = "vosk")]
    {
        status_text.push(Line::from(""));
        status_text.push(Line::from("Last Transcript (final):"));
        let txt = state.last_transcript.as_deref().unwrap_or("None");
        let trunc = if txt.len() > 80 {
            format!("{}…", &txt[..80])
        } else {
            txt.to_string()
        };
        status_text.push(Line::from(trunc));
    }
    status_text.push(Line::from(""));
    status_text.push(Line::from("Controls:"));
    status_text.push(Line::from(
        "[S] Start  [A] Toggle VAD/PTT  [R] Reset  [Q] Quit",
    ));

    let paragraph = Paragraph::new(status_text);
    f.render_widget(paragraph, inner);
}

fn draw_logs(f: &mut Frame, area: Rect, state: &DashboardState) {
    let block = Block::default().title("Logs").borders(Borders::ALL);

    let inner = block.inner(area);
    f.render_widget(block, area);

    let start_time = state
        .logs
        .front()
        .map(|e| e.timestamp)
        .unwrap_or_else(Instant::now);

    let log_lines: Vec<Line> = state
        .logs
        .iter()
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
                Span::styled(&entry.message, Style::default().fg(color)),
            ])
        })
        .collect();

    let paragraph = Paragraph::new(log_lines);
    f.render_widget(paragraph, inner);
}

fn draw_plugins(f: &mut Frame, area: Rect, _state: &DashboardState) {
    let block = Block::default().title("Available Plugins").borders(Borders::ALL);

    let inner = block.inner(area);
    f.render_widget(block, area);

    let mut plugin_lines: Vec<Line> = Vec::new();

    #[cfg(feature = "vosk")]
    {
        plugin_lines.push(Line::from("noop - NoOp Plugin"));
        plugin_lines.push(Line::from("mock - Mock Plugin"));
        plugin_lines.push(Line::from("whisper - Whisper Plugin [STUB]"));

        #[cfg(feature = "parakeet")]
        plugin_lines.push(Line::from("parakeet - Parakeet Plugin"));
    }

    #[cfg(not(feature = "vosk"))]
    {
        plugin_lines.push(Line::from("STT plugins require 'vosk' feature"));
    }

    let paragraph = Paragraph::new(plugin_lines);
    f.render_widget(paragraph, inner);
}

fn draw_plugin_status(f: &mut Frame, area: Rect, state: &DashboardState) {
    let block = Block::default().title("Plugin Status").borders(Borders::ALL);

    let inner = block.inner(area);
    f.render_widget(block, area);

    let mut status_lines: Vec<Line> = Vec::new();

    #[cfg(feature = "vosk")]
    {
        let current = state.plugin_current.as_deref().unwrap_or("None");
        status_lines.push(Line::from(vec![
            Span::raw("Current: "),
            Span::styled(current, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        ]));

        status_lines.push(Line::from(format!("Active: {}", state.plugin_active_count)));

        status_lines.push(Line::from(format!("Requests: {}", state.plugin_transcription_requests)));
        status_lines.push(Line::from(format!("Success: {}", state.plugin_success)));
        status_lines.push(Line::from(format!("Failures: {}", state.plugin_failures)));

        status_lines.push(Line::from(""));
        status_lines.push(Line::from("Controls:"));
        status_lines.push(Line::from("[P] Toggle Tab  [L] Load Plugin"));
        status_lines.push(Line::from("[U] Unload Plugin  [S] Switch"));
    }

    #[cfg(not(feature = "vosk"))]
    {
        status_lines.push(Line::from("STT plugins require 'vosk' feature"));
    }

    let paragraph = Paragraph::new(status_lines);
    f.render_widget(paragraph, inner);
}