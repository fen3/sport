#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use eframe::egui;
use std::path::PathBuf;
use std::time::{Duration, Instant};

const DEFAULT_MINS: u64 = 1;
const DEFAULT_SECS: u64 = 0;
const MENU_FILE: &str = "menu.txt";
const MENU_SEPARATOR: &str = "---";
const CJK_FONT_NAME: &str = "cjk";
const WINDOW_TIMER: [f32; 2] = [540.0, 160.0];
const WINDOW_LIST: [f32; 2] = [540.0, 400.0];

struct MenuItem {
    title: String,
    body: String,
}

fn exe_dir() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
}

fn menu_file_path() -> PathBuf {
    if let Some(dir) = exe_dir() {
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

// ---------- CJK font loading ----------

fn try_read_font(path: &std::path::Path) -> Option<Vec<u8>> {
    std::fs::read(path).ok().filter(|b| !b.is_empty())
}

fn find_cjk_font() -> Option<Vec<u8>> {
    // 1) User override: font.ttf / font.otf / font.ttc next to the exe
    if let Some(dir) = exe_dir() {
        for ext in ["ttf", "otf", "ttc"] {
            let p = dir.join(format!("font.{ext}"));
            if let Some(bytes) = try_read_font(&p) {
                return Some(bytes);
            }
        }
    }

    // 2) Common system paths, ordered by likelihood
    #[cfg(target_os = "windows")]
    let candidates: &[&str] = &[
        "C:\\Windows\\Fonts\\msyh.ttc",
        "C:\\Windows\\Fonts\\msyh.ttf",
        "C:\\Windows\\Fonts\\simhei.ttf",
        "C:\\Windows\\Fonts\\simsun.ttc",
        "C:\\Windows\\Fonts\\SIMSUN.TTC",
    ];

    #[cfg(target_os = "macos")]
    let candidates: &[&str] = &[
        "/System/Library/Fonts/PingFang.ttc",
        "/System/Library/Fonts/STHeiti Medium.ttc",
        "/System/Library/Fonts/Heiti SC.ttc",
        "/System/Library/Fonts/Hiragino Sans GB.ttc",
        "/Library/Fonts/Arial Unicode.ttf",
    ];

    #[cfg(all(unix, not(target_os = "macos")))]
    let candidates: &[&str] = &[
        // Noto CJK SC (Simplified Chinese) — most precise match
        "/usr/share/fonts/opentype/noto-cjk-otf/NotoSansCJKsc-Regular.otf",
        "/usr/share/fonts/truetype/noto-cjk-otf/NotoSansCJKsc-Regular.otf",
        // Catch-all TTC
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Black.ttc",
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.otf",
        "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttf",
        // WenQuanYi — common on Debian/Ubuntu
        "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc",
        "/usr/share/fonts/truetype/wqy-microhei/wqy-microhei.ttc",
        "/usr/share/fonts/truetype/wqy/wqy-zenhei.ttc",
        "/usr/share/fonts/truetype/wqy-zenhei/wqy-zenhei.ttc",
        // Arphic / legacy
        "/usr/share/fonts/truetype/arphic/uming.ttc",
        "/usr/share/fonts/truetype/arphic/ukai.ttc",
        "/usr/local/share/fonts/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
    ];

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    let candidates: &[&str] = &[];

    for path in candidates {
        if let Some(bytes) = try_read_font(std::path::Path::new(path)) {
            return Some(bytes);
        }
    }

    None
}

fn install_cjk_font(ctx: &egui::Context) {
    let Some(data) = find_cjk_font() else {
        return;
    };

    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        CJK_FONT_NAME.to_string(),
        egui::FontData::from_owned(data).into(),
    );

    // Put CJK font at the front of the Proportional fallback chain so Chinese
    // characters in menu titles display correctly.
    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(0, CJK_FONT_NAME.to_string());

    // Also append to Monospace — egui's default Mono font already has ASCII,
    // we just let it fall through to CJK for non-Latin glyphs.
    fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default()
        .push(CJK_FONT_NAME.to_string());

    ctx.set_fonts(fonts);
}

// ---------- App ----------

struct CountdownApp {
    mins_input: String,
    secs_input: String,
    remaining: Duration,
    running: bool,
    last_update: Option<Instant>,
    fonts_installed: bool,
    list_view: bool,
    menu_items: Vec<MenuItem>,
}

impl Default for CountdownApp {
    fn default() -> Self {
        Self {
            mins_input: format!("{DEFAULT_MINS:02}"),
            secs_input: format!("{DEFAULT_SECS:02}"),
            remaining: Duration::from_secs(DEFAULT_MINS * 60 + DEFAULT_SECS),
            running: false,
            last_update: None,
            fonts_installed: false,
            list_view: false,
            menu_items: Vec::new(),
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

    fn open_list(&mut self, ctx: &egui::Context) {
        self.menu_items = load_menu();
        self.list_view = true;
        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
            WINDOW_LIST[0],
            WINDOW_LIST[1],
        )));
    }

    fn close_list(&mut self, ctx: &egui::Context) {
        self.list_view = false;
        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
            WINDOW_TIMER[0],
            WINDOW_TIMER[1],
        )));
    }
}

impl eframe::App for CountdownApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if !self.fonts_installed {
            install_cjk_font(ctx);
            self.fonts_installed = true;
        }

        self.tick();

        let mut open_list = false;
        let mut back_to_timer = false;
        let mut copied: Option<String> = None;

        if self.list_view {
            if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                back_to_timer = true;
            }

            egui::CentralPanel::default().show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.heading("Menu");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Back").clicked() {
                            back_to_timer = true;
                        }
                    });
                });
                ui.separator();

                if self.menu_items.is_empty() {
                    ui.label(format!("({MENU_FILE} not found or empty)"));
                    return;
                }

                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for item in &self.menu_items {
                            let resp = ui.add_sized(
                                egui::vec2(ui.available_width(), 40.0),
                                egui::Button::new(egui::RichText::new(&item.title).size(18.0)),
                            );
                            if resp.clicked() {
                                copied = Some(item.body.clone());
                            }
                        }
                    });
            });
        } else {
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

                    if ui
                        .add(egui::Button::new("Menu").min_size(egui::vec2(88.0, 56.0)))
                        .clicked()
                    {
                        open_list = true;
                    }
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
        }

        if let Some(text) = copied {
            ctx.copy_text(text);
            back_to_timer = true;
        }
        if open_list {
            self.open_list(ctx);
        }
        if back_to_timer {
            self.close_list(ctx);
        }

        ctx.request_repaint_after(Duration::from_millis(100));
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size(WINDOW_TIMER)
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
