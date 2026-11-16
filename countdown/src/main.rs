use eframe::egui;
use std::time::{Duration, Instant};

const DEFAULT_MINS: u64 = 1;
const DEFAULT_SECS: u64 = 0;

struct CountdownApp {
    mins_input: String,
    secs_input: String,
    remaining: Duration,
    running: bool,
    last_update: Option<Instant>,
}

impl Default for CountdownApp {
    fn default() -> Self {
        Self {
            mins_input: format!("{DEFAULT_MINS:02}"),
            secs_input: format!("{DEFAULT_SECS:02}"),
            remaining: Duration::from_secs(DEFAULT_MINS * 60 + DEFAULT_SECS),
            running: false,
            last_update: None,
        }
    }
}

impl CountdownApp {
    fn parse_input(&self) -> Duration {
        let mins = self
            .mins_input
            .trim()
            .parse::<u64>()
            .unwrap_or(DEFAULT_MINS);
        let mut secs = self
            .secs_input
            .trim()
            .parse::<u64>()
            .unwrap_or(DEFAULT_SECS);
        if secs > 59 {
            secs = 59;
        }
        let total = mins.saturating_mul(60).saturating_add(secs);
        if total == 0 {
            Duration::from_secs(DEFAULT_MINS * 60 + DEFAULT_SECS)
        } else {
            Duration::from_secs(total)
        }
    }

    fn set_inputs_from_remaining(&mut self) {
        let total = self.remaining.as_secs();
        self.mins_input = format!("{:02}", total / 60);
        self.secs_input = format!("{:02}", total % 60);
    }

    fn toggle_running(&mut self) {
        if self.running {
            self.running = false;
            self.last_update = None;
            self.remaining = self.parse_input();
            self.set_inputs_from_remaining();
            return;
        }
        let total = self.parse_input();
        if total.is_zero() {
            return;
        }
        self.remaining = total;
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
            self.set_inputs_from_remaining();
        } else {
            self.last_update = Some(now);
            self.set_inputs_from_remaining();
        }
    }
}

impl eframe::App for CountdownApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.tick();

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(24.0);

                let big = egui::FontId::monospace(60.0);
                let sep = egui::RichText::new(":").font(big.clone());

                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.mins_input)
                            .font(big.clone())
                            .desired_width(110.0),
                    );

                    ui.add_space(2.0);
                    ui.label(sep);
                    ui.add_space(2.0);

                    ui.add(
                        egui::TextEdit::singleline(&mut self.secs_input)
                            .font(big.clone())
                            .desired_width(110.0),
                    );

                    ui.add_space(20.0);

                    let btn_label = if self.running { "Stop" } else { "Start" };
                    if ui
                        .add(egui::Button::new(btn_label).min_size(egui::vec2(92.0, 56.0)))
                        .clicked()
                    {
                        self.toggle_running();
                    }
                });

                if self.remaining.is_zero() && !self.running {
                    ui.add_space(16.0);
                    ui.label(
                        egui::RichText::new("TIME'S UP!")
                            .size(22.0)
                            .color(egui::Color32::from_rgb(220, 80, 80)),
                    );
                }
            });
        });

        ctx.request_repaint_after(Duration::from_millis(100));
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([440.0, 210.0])
            .with_resizable(false),
        ..Default::default()
    };

    eframe::run_native(
        "Countdown",
        options,
        Box::new(|_cc| Ok(Box::new(CountdownApp::default()))),
    )
}
