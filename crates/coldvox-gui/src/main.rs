use eframe::egui::{
    self, vec2, Align, Color32, CornerRadius, Frame, Id, Layout, RichText, Sense, TopBottomPanel,
    Vec2,
};

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([700.0, 300.0])
            .with_decorations(false)
            .with_transparent(true)
            .with_resizable(false)
            .with_always_on_top(),
        ..Default::default()
    };
    eframe::run_native(
        "ColdVox",
        native_options,
        Box::new(|cc| Ok(Box::new(ColdVoxApp::new(cc)))),
    )
}

#[derive(PartialEq)]
enum RecordState {
    Ready,
    Recording,
    Processing,
}

#[derive(PartialEq)]
enum ThemeOption {
    Light,
    Dark,
    Auto,
}

struct ColdVoxApp {
    expanded: bool,
    state: RecordState,
    paused: bool,
    transcript: String,
    show_settings: bool,
    transparency: f32,
    audio_device: usize,
    language: usize,
    hotkey: String,
    auto_punctuation: bool,
    theme: ThemeOption,
    format: String,
    api_key: String,
}

impl ColdVoxApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            expanded: false,
            state: RecordState::Ready,
            paused: false,
            transcript: String::new(),
            show_settings: false,
            transparency: 0.3,
            audio_device: 0,
            language: 0,
            hotkey: String::from("Ctrl+Shift+Space"),
            auto_punctuation: true,
            theme: ThemeOption::Auto,
            format: "Plain".to_string(),
            api_key: String::new(),
        }
    }

    fn language_name(&self) -> &'static str {
        match self.language {
            0 => "English",
            1 => "Spanish",
            _ => "English",
        }
    }
}

impl eframe::App for ColdVoxApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.expanded {
            expanded_ui(self, ctx);
        } else {
            collapsed_ui(self, ctx);
        }

        let mut open = self.show_settings;
        egui::Window::new("Settings")
            .open(&mut open)
            .default_size(vec2(480.0, 600.0))
            .min_size(vec2(400.0, 500.0))
            .show(ctx, |ui| settings_ui(self, ui));
        self.show_settings = open;
    }
}

fn collapsed_ui(app: &mut ColdVoxApp, ctx: &egui::Context) {
    let color = Color32::from_rgba_unmultiplied(42, 42, 42, (app.transparency * 255.0) as u8);
    let frame = Frame::new()
        .fill(color)
        .corner_radius(CornerRadius::same(24));
    egui::CentralPanel::default().frame(frame).show(ctx, |ui| {
        ui.set_height(48.0);
        ui.set_width(240.0);
        let response = ui.interact(ui.max_rect(), Id::new("collapsed_bar"), Sense::click());
        if response.clicked() {
            app.expanded = true;
        }
        ui.horizontal(|ui| {
            ui.add_space(14.0);
            ui.label(RichText::new("🎤").size(20.0));
            ui.add_space(14.0);
            let dot_color = match app.state {
                RecordState::Ready => Color32::GREEN,
                RecordState::Recording => Color32::RED,
                RecordState::Processing => Color32::YELLOW,
            };
            let (rect, _) = ui.allocate_exact_size(Vec2::splat(8.0), Sense::hover());
            ui.painter().circle_filled(rect.center(), 4.0, dot_color);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add_space(14.0);
                if ui
                    .add(egui::Button::new(RichText::new("⚙").size(20.0)))
                    .clicked()
                {
                    app.show_settings = true;
                }
            });
        });
    });
}

fn expanded_ui(app: &mut ColdVoxApp, ctx: &egui::Context) {
    let color = Color32::from_rgba_unmultiplied(42, 42, 42, (app.transparency * 255.0) as u8);
    let frame = Frame::new()
        .fill(color)
        .corner_radius(CornerRadius::same(16))
        .stroke(egui::Stroke::new(
            1.0,
            Color32::from_rgba_unmultiplied(255, 255, 255, 25),
        ));
    egui::CentralPanel::default().frame(frame).show(ctx, |ui| {
        TopBottomPanel::top("activity")
            .exact_height(40.0)
            .show_inside(ui, |ui| {
                ui.centered_and_justified(|ui| {
                    ui.label("…activity…");
                });
            });
        TopBottomPanel::bottom("controls")
            .exact_height(40.0)
            .show_inside(ui, |ui| {
                ui.horizontal(|ui| {
                    if ui.button("◼ Stop").clicked() {
                        app.state = RecordState::Ready;
                        app.expanded = false;
                    }
                    if app.paused {
                        if ui.button("▶ Resume").clicked() {
                            app.paused = false;
                        }
                    } else if ui.button("⏸ Pause").clicked() {
                        app.paused = true;
                    }
                    if ui.button("🗑 Clear").clicked() {
                        app.transcript.clear();
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.button("⚙ Settings").clicked() {
                            app.show_settings = true;
                        }
                    });
                });
            });
        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.add_space(10.0);
            egui::ScrollArea::vertical()
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    ui.label(&app.transcript);
                });
        });
    });
}

fn settings_ui(app: &mut ColdVoxApp, ui: &mut egui::Ui) {
    ui.heading("Audio Input Device");
    egui::ComboBox::from_label("Device")
        .selected_text(format!("Device {}", app.audio_device))
        .show_ui(ui, |ui| {
            for i in 0..3 {
                ui.selectable_value(&mut app.audio_device, i, format!("Device {}", i));
            }
        });
    ui.separator();
    ui.heading("Language");
    egui::ComboBox::from_label("Language")
        .selected_text(app.language_name())
        .show_ui(ui, |ui| {
            ui.selectable_value(&mut app.language, 0, "English");
            ui.selectable_value(&mut app.language, 1, "Spanish");
        });
    ui.separator();
    ui.heading("Hotkey");
    ui.text_edit_singleline(&mut app.hotkey);
    ui.separator();
    ui.heading("Transparency");
    ui.add(egui::Slider::new(&mut app.transparency, 0.0..=1.0));
    ui.separator();
    ui.checkbox(&mut app.auto_punctuation, "Auto-punctuation");
    ui.separator();
    ui.heading("Theme");
    ui.horizontal(|ui| {
        ui.selectable_value(&mut app.theme, ThemeOption::Light, "Light");
        ui.selectable_value(&mut app.theme, ThemeOption::Dark, "Dark");
        ui.selectable_value(&mut app.theme, ThemeOption::Auto, "Auto");
    });
    ui.separator();
    ui.heading("Output Format");
    egui::ComboBox::from_label("Format")
        .selected_text(&app.format)
        .show_ui(ui, |ui| {
            ui.selectable_value(&mut app.format, "Plain".to_owned(), "Plain");
            ui.selectable_value(&mut app.format, "Rich".to_owned(), "Rich");
        });
    ui.separator();
    ui.heading("API Key");
    ui.add(egui::TextEdit::singleline(&mut app.api_key).password(true));
}
