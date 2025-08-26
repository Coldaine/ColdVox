use clap::Parser;
use coldvox_app::audio::{AudioCapture, AudioChunker, ChunkerConfig};
use coldvox_app::audio::vad_processor::VadProcessor;
use coldvox_app::foundation::error::AudioConfig;
use coldvox_app::telemetry::pipeline_metrics::{PipelineMetrics, PipelineStage};
use coldvox_app::vad::config::{UnifiedVadConfig, VadMode};
use coldvox_app::vad::types::VadEvent;
use crossbeam_channel::bounded;
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
    widgets::{
        Block, Borders, Gauge, 
        Paragraph, Sparkline
    },
    Frame, Terminal,
};
use std::collections::VecDeque;
use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

#[derive(Parser)]
#[command(author, version, about = "TUI Dashboard with real-time audio monitoring")]
struct Cli {
    /// Audio device name
    #[arg(short = 'D', long)]
    device: Option<String>,
}

struct DashboardState {
    // Pipeline metrics
    metrics: Arc<PipelineMetrics>,
    
    // Audio level history (for sparkline)
    level_history: VecDeque<u8>,  // 0-100 scale
    peak_history: VecDeque<u8>,   // Peak levels
    
    // Pipeline flow indicators
    pipeline_pulses: Vec<Instant>, // Animation timing
    
    // VAD state
    is_speaking: bool,
    speech_segments: u64,
    last_vad_event: Option<String>,
    
    // System state
    is_running: bool,
    selected_device: String,
    start_time: Instant,
    
    // Frame counters
    capture_frames: u64,
    chunker_frames: u64,
    vad_frames: u64,
    
    // Logs
    logs: VecDeque<LogEntry>,
    
    // Shutdown signal
    shutdown: Arc<AtomicBool>,
    
    // Pipeline handle for cleanup
    pipeline_handle: Option<tokio::task::JoinHandle<()>>,
}

#[derive(Clone)]
struct LogEntry {
    timestamp: Instant,
    level: LogLevel,
    message: String,
}

#[derive(Clone, Debug)]
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
            metrics: Arc::new(PipelineMetrics::default()),
            level_history,
            peak_history,
            pipeline_pulses: vec![Instant::now(); 4],
            is_speaking: false,
            speech_segments: 0,
            last_vad_event: None,
            is_running: false,
            selected_device: "default".to_string(),
            start_time: Instant::now(),
            capture_frames: 0,
            chunker_frames: 0,
            vad_frames: 0,
            logs,
            shutdown: Arc::new(AtomicBool::new(false)),
            pipeline_handle: None,
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
        // Get current RMS level as percentage
        let rms = self.metrics.current_rms.load(Ordering::Relaxed);
        let level = ((rms as f64 / 32768.0) * 100.0).min(100.0) as u8;
        
        // Get peak level
        let peak = self.metrics.current_peak.load(Ordering::Relaxed);
        let peak_level = ((peak as f64 / 32768.0) * 100.0).min(100.0) as u8;
        
        self.level_history.pop_front();
        self.level_history.push_back(level);
        
        self.peak_history.pop_front();
        self.peak_history.push_back(peak_level);
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    
    // Initialize state
    let state = Arc::new(Mutex::new(DashboardState::default()));
    
    // Set device if provided
    if let Some(device) = cli.device {
        let mut dashboard = state.lock().await;
        dashboard.selected_device = device;
    }
    
    // Run the app
    let res = run_app(&mut terminal, state).await;
    
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
    state: Arc<Mutex<DashboardState>>,
) -> io::Result<()> {
    let mut update_interval = tokio::time::interval(Duration::from_millis(50));
    
    loop {
        // Draw UI
        let state_clone = state.clone();
        terminal.draw(|f| {
            let state = futures::executor::block_on(state_clone.lock());
            draw_ui(f, &state);
        })?;
        
        // Handle events
        if event::poll(Duration::from_millis(10))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Char('Q') => {
                        let mut dashboard = state.lock().await;
                        if dashboard.is_running {
                            dashboard.log(LogLevel::Info, "Shutting down audio pipeline...".to_string());
                            dashboard.shutdown.store(true, Ordering::Relaxed);
                            
                            // Wait for pipeline to finish
                            if let Some(handle) = dashboard.pipeline_handle.take() {
                                let _ = tokio::time::timeout(Duration::from_secs(2), handle).await;
                            }
                        }
                        return Ok(());
                    }
                    KeyCode::Char('s') | KeyCode::Char('S') => {
                        let mut dashboard = state.lock().await;
                        if !dashboard.is_running {
                            dashboard.log(LogLevel::Info, "Starting audio pipeline...".to_string());
                            dashboard.shutdown.store(false, Ordering::Relaxed);
                            dashboard.is_running = true;
                            
                            // Start pipeline in background thread
                            let state_clone = state.clone();
                            std::thread::spawn(move || {
                                let rt = tokio::runtime::Runtime::new().unwrap();
                                rt.block_on(async move {
                                    run_audio_pipeline(state_clone).await;
                                });
                            });
                            dashboard.pipeline_handle = None; // Can't track std::thread
                        }
                    }
                    KeyCode::Char('r') | KeyCode::Char('R') => {
                        let mut dashboard = state.lock().await;
                        // Reset metrics
                        dashboard.capture_frames = 0;
                        dashboard.chunker_frames = 0;
                        dashboard.vad_frames = 0;
                        dashboard.speech_segments = 0;
                        dashboard.log(LogLevel::Info, "Metrics reset".to_string());
                    }
                    _ => {}
                }
            }
        }
        
        // Update metrics periodically
        tokio::select! {
            _ = update_interval.tick() => {
                let mut dashboard = state.lock().await;
                dashboard.update_level_history();
                
                // Decay pipeline indicators for animation
                if dashboard.is_running {
                    let now = Instant::now();
                    let should_decay = dashboard.pipeline_pulses.iter()
                        .any(|pulse| now.duration_since(*pulse) > Duration::from_millis(500));
                    if should_decay {
                        dashboard.metrics.decay_stages();
                    }
                }
            }
        }
    }
}

async fn run_audio_pipeline(state: Arc<Mutex<DashboardState>>) {
    let (metrics, shutdown) = {
        let dashboard = state.lock().await;
        (dashboard.metrics.clone(), dashboard.shutdown.clone())
    };
    
    // Create audio capture
    let audio_config = AudioConfig::default();
    let capture_result = AudioCapture::new(audio_config);
    let audio_capture = match capture_result {
        Ok(cap) => Arc::new(Mutex::new(cap)),
        Err(e) => {
            let mut dashboard = state.lock().await;
            dashboard.log(LogLevel::Error, format!("Failed to create audio capture: {}", e));
            dashboard.is_running = false;
            return;
        }
    };
    
    // Start capture
    let device = {
        let dashboard = state.lock().await;
        Some(dashboard.selected_device.clone())
    };
    
    {
        let mut capture = audio_capture.lock().await;
        if let Err(e) = capture.start(device.as_deref()).await {
            let mut dashboard = state.lock().await;
            dashboard.log(LogLevel::Error, format!("Failed to start capture: {}", e));
            dashboard.is_running = false;
            return;
        }
    }
    
    {
        let mut dashboard = state.lock().await;
        dashboard.log(LogLevel::Success, "Audio capture started".to_string());
    }
    
    // Set up pipeline channels
    let capture_rx = {
        let capture = audio_capture.lock().await;
        capture.get_receiver()
    };
    let (vad_in_tx, vad_in_rx) = bounded(100);
    let (event_tx, event_rx) = bounded(200);
    
    // Start chunker
    let chunker_cfg = ChunkerConfig { 
        frame_size_samples: 512, 
        sample_rate_hz: 16_000 
    };
    let chunker = AudioChunker::new(capture_rx.clone(), vad_in_tx, chunker_cfg);
    let _chunker_handle = chunker.spawn();
    
    // Start VAD processor
    let mut vad_cfg = UnifiedVadConfig::default();
    vad_cfg.mode = VadMode::Silero;
    vad_cfg.frame_size_samples = 512;
    vad_cfg.sample_rate_hz = 16_000;
    
    let vad_shutdown = Arc::new(AtomicBool::new(false));
    let _vad_thread = match VadProcessor::spawn(
        vad_cfg,
        vad_in_rx,
        event_tx,
        vad_shutdown.clone(),
    ) {
        Ok(h) => h,
        Err(e) => {
            let mut dashboard = state.lock().await;
            dashboard.log(LogLevel::Error, format!("Failed to spawn VAD: {}", e));
            dashboard.is_running = false;
            return;
        }
    };
    
    {
        let mut dashboard = state.lock().await;
        dashboard.log(LogLevel::Success, "Pipeline fully started".to_string());
    }
    
    // Monitor VAD events
    let metrics_vad = metrics.clone();
    let state_vad = state.clone();
    let shutdown_monitor = shutdown.clone();
    let monitor_handle = tokio::spawn(async move {
        loop {
            if shutdown_monitor.load(Ordering::Relaxed) {
                break;
            }
            match event_rx.recv_timeout(Duration::from_millis(100)) {
                Ok(event) => {
                    metrics_vad.mark_stage_active(PipelineStage::Vad);
                    metrics_vad.mark_stage_active(PipelineStage::Output);
                    
                    let mut dashboard = state_vad.lock().await;
                    dashboard.vad_frames += 1;
                    
                    match event {
                        VadEvent::SpeechStart { timestamp_ms, energy_db } => {
                            dashboard.is_speaking = true;
                            dashboard.speech_segments += 1;
                            dashboard.last_vad_event = Some(format!("Speech START @ {}ms ({:.1}dB)", timestamp_ms, energy_db));
                            dashboard.log(LogLevel::Success, format!("Speech detected @ {}ms", timestamp_ms));
                            
                            metrics_vad.is_speaking.store(true, Ordering::Relaxed);
                            metrics_vad.speech_segments_count.fetch_add(1, Ordering::Relaxed);
                        }
                        VadEvent::SpeechEnd { timestamp_ms, duration_ms, energy_db } => {
                            dashboard.is_speaking = false;
                            dashboard.last_vad_event = Some(format!("Speech END @ {}ms ({}ms, {:.1}dB)", timestamp_ms, duration_ms, energy_db));
                            dashboard.log(LogLevel::Info, format!("Speech ended, duration: {}ms", duration_ms));
                            
                            metrics_vad.is_speaking.store(false, Ordering::Relaxed);
                        }
                    }
                }
                Err(_) => {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            }
        }
    });
    
    // Wait for shutdown signal
    while !shutdown.load(Ordering::Relaxed) {
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    
    // Shutdown pipeline
    {
        let mut dashboard = state.lock().await;
        dashboard.log(LogLevel::Info, "Stopping pipeline...".to_string());
    }
    
    // Signal VAD to stop
    vad_shutdown.store(true, Ordering::Relaxed);
    
    // Stop audio capture
    {
        let mut capture = audio_capture.lock().await;
        capture.stop();
    }
    
    // Wait for monitor thread to finish
    let _ = tokio::time::timeout(Duration::from_secs(2), monitor_handle).await;
    
    {
        let mut dashboard = state.lock().await;
        dashboard.is_running = false;
        dashboard.log(LogLevel::Success, "Pipeline stopped".to_string());
    }
}

fn draw_ui(f: &mut Frame, state: &DashboardState) {
    // Main layout: 3 rows
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(12),  // Top: Audio levels & pipeline
            Constraint::Min(10),      // Middle: Metrics & status
            Constraint::Length(8),    // Bottom: Logs
        ])
        .split(f.size());
    
    // Top section: Audio levels and pipeline flow
    let top_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(60),  // Audio levels
            Constraint::Percentage(40),  // Pipeline flow
        ])
        .split(main_chunks[0]);
    
    // Audio levels panel
    draw_audio_levels(f, top_chunks[0], state);
    
    // Pipeline flow panel
    draw_pipeline_flow(f, top_chunks[1], state);
    
    // Middle section: Metrics and status
    let middle_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50),  // Metrics
            Constraint::Percentage(50),  // Status & VAD
        ])
        .split(main_chunks[1]);
    
    draw_metrics(f, middle_chunks[0], state);
    draw_status(f, middle_chunks[1], state);
    
    // Bottom: Logs
    draw_logs(f, main_chunks[2], state);
}

fn draw_audio_levels(f: &mut Frame, area: Rect, state: &DashboardState) {
    let block = Block::default()
        .title("Audio Levels")
        .borders(Borders::ALL);
    
    let inner = block.inner(area);
    f.render_widget(block, area);
    
    // Split into meter and history
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),   // Current level gauge
            Constraint::Length(1),   // dB display
            Constraint::Min(4),      // History sparkline
        ])
        .split(inner);
    
    // Current level gauge
    let db = state.metrics.audio_level_db.load(Ordering::Relaxed) as f64 / 10.0;
    let level_percent = ((db + 90.0) / 90.0 * 100.0).max(0.0).min(100.0) as u16;
    
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
    
    // dB text display
    let db_text = Paragraph::new(format!("Peak: {:.1} dB | RMS: {:.1} dB", 
        db,
        (state.metrics.current_rms.load(Ordering::Relaxed) as f64 / 1000.0).log10() * 20.0
    ))
    .alignment(Alignment::Center);
    f.render_widget(db_text, chunks[1]);
    
    // History sparkline
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
    
    // Create pipeline stage indicators
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(2),
            Constraint::Length(2),
            Constraint::Length(2),
        ])
        .split(inner);
    
    // Stage indicators with activity status
    let stages = [
        ("1. Capture", state.metrics.stage_capture.load(Ordering::Relaxed), state.capture_frames),
        ("2. Chunker", state.metrics.stage_chunker.load(Ordering::Relaxed), state.chunker_frames),
        ("3. VAD", state.metrics.stage_vad.load(Ordering::Relaxed), state.vad_frames),
        ("4. Output", state.metrics.stage_output.load(Ordering::Relaxed), state.speech_segments),
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
        let text = format!("{} {} [{} frames]", indicator, name, count);
        
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
        Line::from(format!("Capture FPS: {:.1}", 
            state.metrics.capture_fps.load(Ordering::Relaxed) as f64 / 10.0)),
        Line::from(format!("Chunker FPS: {:.1}", 
            state.metrics.chunker_fps.load(Ordering::Relaxed) as f64 / 10.0)),
        Line::from(format!("VAD FPS: {:.1}", 
            state.metrics.vad_fps.load(Ordering::Relaxed) as f64 / 10.0)),
        Line::from(""),
        Line::from(format!("Buffer Fill:")),
        Line::from(format!("  Capture: {}%", 
            state.metrics.capture_buffer_fill.load(Ordering::Relaxed))),
        Line::from(format!("  Chunker: {}%", 
            state.metrics.chunker_buffer_fill.load(Ordering::Relaxed))),
        Line::from(format!("  VAD: {}%", 
            state.metrics.vad_buffer_fill.load(Ordering::Relaxed))),
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
                Style::default().fg(status_color).add_modifier(Modifier::BOLD)
            ),
        ]),
        Line::from(format!("Device: {}", state.selected_device)),
        Line::from(""),
        Line::from(vec![
            Span::raw("Speaking: "),
            Span::styled(
                if state.is_speaking { "YES" } else { "NO" },
                Style::default().fg(if state.is_speaking { Color::Green } else { Color::Gray })
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
        .take(5)
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