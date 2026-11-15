use eframe::egui;
use std::time::{Duration, Instant};

const DEFAULT_SECONDS: u64 = 60;

struct CountdownApp {
    seconds_input: String,
    remaining: Duration,
    running: bool,
    last_update: Option<Instant>,
}

impl Default for CountdownApp {
    fn default() -> Self {
        Self {
            seconds_input: DEFAULT_SECONDS.to_string(),
            remaining: Duration::from_secs(DEFAULT_SECONDS),
            running: false,
            last_update: None,
        }
    }
}

impl CountdownApp {
    fn configured_duration(&self) -> Duration {
        let seconds = self
            .seconds_input
            .trim()
            .parse::<u64>()
            .ok()
            .filter(|&seconds| seconds > 0)
            .unwrap_or(DEFAULT_SECONDS);
        Duration::from_secs(seconds)
    }

    fn reset(&mut self) {
        self.remaining = self.configured_duration();
        self.running = false;
        self.last_update = None;
    }

    fn toggle_running(&mut self) {
        if self.running {
            self.running = false;
            self.last_update = None;
            return;
        }

        if self.remaining.is_zero() {
            self.remaining = self.configured_duration();
        }
        self.running = true;
        self.last_update = Some(Instant::now());
    }

    fn tick(&mut self) {
        let now = Instant::now();
        if !self.running {
            return;
        }

        if let Some(last_update) = self.last_update {
            self.remaining = self
                .remaining
                .saturating_sub(now.saturating_duration_since(last_update));
        }

        if self.remaining.is_zero() {
            self.running = false;
            self.last_update = None;
        } else {
            self.last_update = Some(now);
        }
    }

    fn display_time(&self) -> String {
        if self.remaining.is_zero() {
            return "TIME'S UP!".to_owned();
        }

        let seconds = self.remaining.as_secs() + u64::from(self.remaining.subsec_nanos() > 0);
        format!("{:02}:{:02}", seconds / 60, seconds % 60)
    }
}

impl eframe::App for CountdownApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.tick();

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(12.0);
                ui.label(egui::RichText::new(self.display_time()).size(48.0));
                ui.add_space(12.0);

                ui.horizontal(|ui| {
                    ui.label("sec:");
                    ui.add(egui::TextEdit::singleline(&mut self.seconds_input).desired_width(60.0));
                });
                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    let start_label = if self.running { "Pause" } else { "Start" };
                    if ui.button(start_label).clicked() {
                        self.toggle_running();
                    }
                    if ui.button("Reset").clicked() {
                        self.reset();
                    }
                });
            });
        });

        ctx.request_repaint_after(Duration::from_millis(100));
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([300.0, 180.0])
            .with_resizable(false),
        ..Default::default()
    };

    eframe::run_native(
        "Countdown",
        options,
        Box::new(|_cc| Ok(Box::new(CountdownApp::default()))),
    )
}
