#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use eframe::egui;
use std::path::PathBuf;
use std::time::{Duration, Instant};

const DEFAULT_MINS: u64 = 1;
const DEFAULT_SECS: u64 = 0;
const MENU_FILE: &str = "menu.txt";
const MENU_SEPARATOR: &str = "---";

struct MenuItem {
    title: String,
    body: String,
}

fn menu_file_path() -> PathBuf {
    if let Some(dir) = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
    {
        let candidate = dir.join(MENU_FILE);
        if candidate.is_file() {
            return candidate;
        }
    }
    PathBuf::from(MENU_FILE)
}

fn load_menu() -> Vec<MenuItem> {
    match std::fs::read_to_string(menu_file_path()) {
        Ok(text) => parse_menu(&text),
        Err(_) => Vec::new(),
    }
}

fn parse_menu(text: &str) -> Vec<MenuItem> {
    let mut blocks: Vec<Vec<&str>> = Vec::new();
    let mut current: Vec<&str> = Vec::new();
    for line in text.lines() {
        if line.trim() == MENU_SEPARATOR {
            if !current.is_empty() {
                blocks.push(std::mem::take(&mut current));
            }
        } else {
            current.push(line);
        }
    }
    if !current.is_empty() {
        blocks.push(current);
    }

    blocks
        .into_iter()
        .filter_map(|block| {
            let title = block.first()?.trim().to_string();
            if title.is_empty() {
                return None;
            }
            Some(MenuItem {
                title,
                body: block[1..].join("\n").trim().to_string(),
            })
        })
        .collect()
}

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
            let big = egui::FontId::monospace(60.0);
            let sep = egui::RichText::new(":").font(big.clone());

            ui.horizontal_centered(|ui| {
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

                ui.add_space(16.0);

                let btn_label = if self.running { "Stop" } else { "Start" };
                if ui
                    .add(egui::Button::new(btn_label).min_size(egui::vec2(88.0, 56.0)))
                    .clicked()
                {
                    self.toggle_running();
                }

                ui.add_space(8.0);

                ui.scope(|ui| {
                    ui.spacing_mut().interact_size.y = 56.0;
                    ui.menu_button("Menu", |ui| {
                        let items = load_menu();
                        if items.is_empty() {
                            ui.label(format!("no {MENU_FILE}"));
                            return;
                        }
                        for item in &items {
                            if ui.button(&item.title).clicked() {
                                ui.ctx().copy_text(item.body.clone());
                                ui.close();
                            }
                        }
                    });
                });
            });

            if self.remaining.is_zero() && !self.running {
                ui.add_space(10.0);
                ui.horizontal_centered(|ui| {
                    ui.label(
                        egui::RichText::new("TIME'S UP!")
                            .size(22.0)
                            .color(egui::Color32::from_rgb(220, 80, 80)),
                    );
                });
            }
        });

        ctx.request_repaint_after(Duration::from_millis(100));
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([540.0, 160.0])
            .with_resizable(false),
        ..Default::default()
    };

    eframe::run_native(
        "Countdown",
        options,
        Box::new(|_cc| Ok(Box::new(CountdownApp::default()))),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_title_and_body() {
        let items = parse_menu("Hello\nHello, World!\n---\nSig\nBest regards,\nMe\n");
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].title, "Hello");
        assert_eq!(items[0].body, "Hello, World!");
        assert_eq!(items[1].title, "Sig");
        assert_eq!(items[1].body, "Best regards,\nMe");
    }

    #[test]
    fn ignores_separator_fragments_and_blank_blocks() {
        let items = parse_menu("only title\n---\n---\n  T  \nbody\n");
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].title, "only title");
        assert_eq!(items[0].body, "");
        assert_eq!(items[1].title, "T");
    }
}
