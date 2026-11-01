//! tuf-crate — Armoury-Crate-equivalent control center for the ASUS TUF Gaming
//! F17 FX706HF, built exclusively on kernel-sanctioned interfaces.
//!
//! Safety model:
//! - If the privileged `tuf-crated` daemon is running (installed by
//!   scripts/install.sh), hardware controls work out of the box — writes go
//!   through the daemon's closed-enum, clamped, audited funnel.
//! - Without the daemon, the app is monitoring-only (`--writes` enables direct
//!   sysfs writes, which need root and will fail as a normal user).
//! - `--mock` runs against the simulated machine for development/demo.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub mod music;
pub mod profiles;

use std::path::{Path, PathBuf};

use eframe::egui;

use tuf_core::Profile;
use tuf_hal::{
    Backend, HalError, MockBackend, SocketBackend, SysfsBackend, Telemetry, DEFAULT_SOCKET,
};

fn main() -> eframe::Result {
    let args: Vec<String> = std::env::args().collect();
    let mock = args.iter().any(|a| a == "--mock");
    let writes = args.iter().any(|a| a == "--writes");

    let audit = Some(
        PathBuf::from(
            std::env::var("XDG_STATE_HOME")
                .unwrap_or_else(|_| format!("{}/.local/state", std::env::var("HOME").unwrap())),
        )
        .join("tuf-crate/audit.jsonl"),
    );
    let backend: Box<dyn Backend> = if mock {
        Box::new(MockBackend::with_writes(writes))
    } else if Path::new(DEFAULT_SOCKET).exists() {
        // Controls work out of the box: writes go through the root daemon.
        let sys = match SysfsBackend::real(false, audit) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("tuf-crate: cannot initialise hardware backend: {e}");
                std::process::exit(1);
            }
        };
        Box::new(SocketBackend::new(sys, PathBuf::from(DEFAULT_SOCKET)))
    } else {
        match SysfsBackend::real(writes, audit) {
            Ok(b) => Box::new(b),
            Err(e) => {
                eprintln!("tuf-crate: cannot initialise hardware backend: {e}");
                std::process::exit(1);
            }
        }
    };

    let writes_label = if backend.name() == "daemon" {
        "controls: daemon (verified ops)"
    } else if mock {
        if writes {
            "writes ENABLED (mock)"
        } else {
            "monitoring-only (mock)"
        }
    } else if writes {
        "writes ENABLED (direct sysfs)"
    } else {
        "monitoring-only (no daemon)"
    };
    eprintln!("tuf-crate: backend={}, mode={writes_label}", backend.name());

    let app = TufApp::new(backend, writes);
    let opts = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1040.0, 700.0])
            .with_min_inner_size([880.0, 560.0]),
        ..Default::default()
    };
    eframe::run_native(
        "tuf-crate — TUF FX706HF Control Center",
        opts,
        Box::new(move |cc| {
            egui_extras_style(cc);
            Ok(Box::new(app))
        }),
    )
}

fn egui_extras_style(cc: &eframe::CreationContext<'_>) {
    // Armoury-Crate-like dark theme with TUF-orange accents
    let mut v = egui::Visuals::dark();
    v.panel_fill = egui::Color32::from_rgb(17, 18, 22);
    v.window_fill = egui::Color32::from_rgb(22, 24, 28);
    v.extreme_bg_color = egui::Color32::from_rgb(11, 12, 15);
    v.selection.bg_fill = egui::Color32::from_rgb(64, 44, 20);
    v.selection.stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(235, 140, 50));
    v.widgets.inactive.bg_fill = egui::Color32::from_rgb(28, 30, 36);
    v.widgets.inactive.weak_bg_fill = egui::Color32::from_rgb(28, 30, 36);
    v.widgets.hovered.bg_fill = egui::Color32::from_rgb(40, 42, 48);
    v.widgets.hovered.weak_bg_fill = egui::Color32::from_rgb(40, 42, 48);
    v.widgets.active.bg_fill = egui::Color32::from_rgb(64, 44, 20);
    cc.egui_ctx.set_visuals(v);
}

/// Usage card with orange title (Armoury System Usage style).
fn usage_card(ui: &mut egui::Ui, title: &str, content: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::none()
        .fill(egui::Color32::from_rgb(21, 23, 28))
        .rounding(6.0)
        .inner_margin(12.0)
        .show(ui, |ui| {
            ui.set_min_width(292.0);
            ui.set_max_width(296.0);
            ui.colored_label(
                egui::Color32::from_rgb(235, 140, 50),
                egui::RichText::new(format!("⋙ {title}")).strong(),
            );
            ui.add_space(2.0);
            content(ui);
        });
}

/// Armoury-style ring gauge (CPU/GPU temps, fan RPM, battery).
fn ring_gauge(ui: &mut egui::Ui, fraction: f32, value: &str, caption: &str, color: egui::Color32) {
    let size = 92.0;
    let (rect, _) = ui.allocate_exact_size(egui::vec2(size, size + 18.0), egui::Sense::hover());
    let center = rect.center_top() + egui::vec2(0.0, size / 2.0);
    let radius = size / 2.0 - 7.0;
    let painter = ui.painter_at(rect);
    painter.circle_stroke(
        center,
        radius,
        egui::Stroke::new(7.0, egui::Color32::from_rgb(44, 46, 54)),
    );
    let frac = fraction.clamp(0.0, 1.0);
    if frac > 0.004 {
        let start = -std::f32::consts::FRAC_PI_2;
        let end = start + frac * std::f32::consts::TAU;
        let steps = 48;
        let points: Vec<egui::Pos2> = (0..=steps)
            .map(|i| {
                let a = start + (end - start) * (i as f32 / steps as f32);
                center + egui::vec2(radius * a.cos(), radius * a.sin())
            })
            .collect();
        painter.add(egui::Shape::line(points, egui::Stroke::new(7.0, color)));
    }
    painter.text(
        center,
        egui::Align2::CENTER_CENTER,
        value,
        egui::FontId::proportional(15.0),
        egui::Color32::WHITE,
    );
    painter.text(
        egui::pos2(center.x, rect.bottom() - 9.0),
        egui::Align2::CENTER_CENTER,
        caption,
        egui::FontId::proportional(11.0),
        egui::Color32::GRAY,
    );
}

fn temp_color(mdeg: i64, warn_mdeg: i64) -> egui::Color32 {
    let f = (mdeg as f32 / warn_mdeg as f32).clamp(0.0, 1.0);
    if f < 0.6 {
        egui::Color32::from_rgb(110, 200, 120)
    } else if f < 0.85 {
        egui::Color32::from_rgb(235, 180, 60)
    } else {
        egui::Color32::from_rgb(235, 100, 80)
    }
}

/// Inputs for the per-key preview animation.
#[derive(Clone, Copy)]
struct EffectParams {
    mode: u32,
    bh: f32,
    bs: f32,
    bv: f32,
    cols: usize,
    t: f32,
    rate: f32,
}

struct TufApp {
    backend: Box<dyn Backend>,
    telemetry: Option<Telemetry>,
    telemetry_error: Option<String>,
    last_poll: std::time::Instant,
    writes_enabled: bool,
    status_msg: Option<(String, bool)>, // (message, is_error)
    confirm_dgpu: bool,
    pending_threshold: Option<u32>,
    pending_pl1: Option<u32>,
    boot_config: Option<serde_json::Value>,
    rgb_hue: f32,
    rgb_sat: f32,
    rgb_val: f32,
    rgb_color: [u8; 3],
    rgb_mode: u32,
    rgb_speed: u32,
    rgb_profile_name: String,
    music: Option<music::MusicCapture>,
    soft_effect: Option<u32>,
    last_soft_apply: std::time::Instant,
    last_soft_rgb: [u8; 3],
    soft_note: String,
    cpu_util: Option<u32>,
    prev_stat: Option<(u64, u64)>,
    tab: Tab,
}

impl TufApp {
    fn new(backend: Box<dyn Backend>, writes_enabled: bool) -> Self {
        let mut app = TufApp {
            backend,
            telemetry: None,
            telemetry_error: None,
            last_poll: std::time::Instant::now() - std::time::Duration::from_secs(10),
            writes_enabled,
            status_msg: None,
            confirm_dgpu: false,
            pending_threshold: None,
            pending_pl1: None,
            boot_config: None,
            rgb_hue: 0.0,
            rgb_sat: 1.0,
            rgb_val: 1.0,
            rgb_color: [255, 0, 0],
            rgb_mode: 0,
            rgb_speed: 0xeb, // Armoury speed 67 % (kernel NORMAL)
            rgb_profile_name: String::new(),
            music: None,
            soft_effect: None,
            last_soft_apply: std::time::Instant::now(),
            last_soft_rgb: [0, 0, 0],
            soft_note: String::new(),
            cpu_util: None,
            prev_stat: None,
            tab: Tab::Home,
        };
        app.poll();
        app
    }

    fn poll(&mut self) {
        // CPU utilisation from /proc/stat deltas
        if let Ok(stat) = std::fs::read_to_string("/proc/stat") {
            if let Some(l) = stat.lines().next() {
                let f: Vec<u64> = l
                    .split_whitespace()
                    .skip(1)
                    .filter_map(|x| x.parse().ok())
                    .collect();
                if f.len() >= 4 {
                    let idle = f[3] + f.get(4).copied().unwrap_or(0);
                    let total: u64 = f.iter().sum();
                    if let Some((pt, pi)) = self.prev_stat {
                        let dt = total.saturating_sub(pt);
                        if dt > 0 {
                            self.cpu_util = Some(
                                (((dt - idle.saturating_sub(pi)) as f64 / dt as f64) * 100.0)
                                    .round() as u32,
                            );
                        }
                    }
                    self.prev_stat = Some((total, idle));
                }
            }
        }
        match self.backend.telemetry() {
            Ok(t) => {
                self.telemetry = Some(t);
                self.telemetry_error = None;
            }
            Err(e) => {
                self.telemetry_error = Some(e.to_string());
            }
        }
    }

    fn report(&mut self, msg: String, is_error: bool) {
        self.status_msg = Some((msg, is_error));
    }

    /// Push the selected effect + colour + speed to the keyboard through the
    /// verified WMNB channel.
    fn apply_rgb(&mut self, mode: u32) {
        if self.soft_effect.is_some() {
            self.stop_soft_effect();
        }
        let [r, g, b] = self.rgb_color;
        let speed = self.rgb_speed;
        let name = Self::aura_mode_name(mode);
        match self
            .backend
            .set_kbd_rgb(mode, r as u32, g as u32, b as u32, speed, true)
        {
            Ok(()) => self.report(
                format!(
                    "{name} applied — #{r:02X}{g:02X}{b:02X}, speed {} %",
                    Self::speed_percent(speed)
                ),
                false,
            ),
            Err(e) => self.report(format!("RGB apply failed: {}", friendly_error(&e)), true),
        }
        self.poll();
    }

    /// Start the Linux-native audio-reactive Music effect (pw-record monitor).
    fn start_music(&mut self) {
        self.stop_soft_effect();
        match music::MusicCapture::start() {
            Ok(cap) => {
                self.music = Some(cap);
                self.soft_effect = Some(music::MUSIC_EFFECT_ID);
                self.last_soft_apply =
                    std::time::Instant::now() - std::time::Duration::from_millis(500);
                self.last_soft_rgb = [0, 0, 0];
                self.report(
                    "Music effect started — streaming Static colour frames".to_string(),
                    false,
                );
                self.tick_soft_effect();
            }
            Err(e) => self.report(format!("Music effect unavailable: {e}"), true),
        }
    }

    /// Smart: colour follows the CPU package temperature.
    fn start_smart(&mut self) {
        self.stop_soft_effect();
        self.soft_effect = Some(music::SMART_EFFECT_ID);
        self.last_soft_apply =
            std::time::Instant::now() - std::time::Duration::from_secs(1);
        self.last_soft_rgb = [0, 0, 0];
        self.soft_note.clear();
        self.report("Smart effect started — colour follows CPU temperature".to_string(), false);
        self.tick_soft_effect();
    }

    /// Adaptive Color: colour follows the average of the screen (X11).
    fn start_adaptive(&mut self) {
        self.stop_soft_effect();
        self.soft_effect = Some(music::ADAPTIVE_EFFECT_ID);
        self.last_soft_apply =
            std::time::Instant::now() - std::time::Duration::from_secs(1);
        self.last_soft_rgb = [0, 0, 0];
        self.soft_note.clear();
        self.report(
            "Adaptive Color started — colour follows the screen average".to_string(),
            false,
        );
        self.tick_soft_effect();
    }

    /// Stop any software-rendered effect.
    fn stop_soft_effect(&mut self) {
        if self.soft_effect.take().is_some() {
            self.music = None;
            self.report("Software effect stopped".to_string(), false);
        }
    }

    /// Push one software-effect frame, rate-limited per effect and gated on a
    /// meaningful colour change so the audit log stays readable.
    fn tick_soft_effect(&mut self) {
        let Some(effect) = self.soft_effect else {
            return;
        };
        let interval = match effect {
            music::MUSIC_EFFECT_ID => 80,
            music::SMART_EFFECT_ID => 500,
            music::ADAPTIVE_EFFECT_ID => 1000,
            _ => return,
        };
        if self.last_soft_apply.elapsed() < std::time::Duration::from_millis(interval) {
            return;
        }
        let [r, g, b] = match effect {
            music::MUSIC_EFFECT_ID => {
                let (level, beat) = self
                    .music
                    .as_ref()
                    .map(|m| (m.level() as f32 / 1000.0, m.take_beat()))
                    .unwrap_or((0.0, false));
                self.soft_note = format!("level {}", if beat { 1000 } else { (level * 1000.0) as u32 });
                Self::music_color(
                    self.rgb_hue,
                    self.rgb_sat,
                    if beat { 1.0 } else { level },
                )
            }
            music::SMART_EFFECT_ID => {
                let temp_c = self
                    .telemetry
                    .as_ref()
                    .and_then(|t| t.cpu_package_temp_mC)
                    .map(|m| m as f32 / 1000.0)
                    .unwrap_or(50.0);
                self.soft_note = format!("{temp_c:.0} °C");
                Self::smart_color(temp_c)
            }
            music::ADAPTIVE_EFFECT_ID => match Self::sample_screen_colour() {
                Some(c) => {
                    self.soft_note = format!("#{:02X}{:02X}{:02X}", c[0], c[1], c[2]);
                    c
                }
                None => {
                    self.report(
                        "Adaptive Color needs an X11 display and ImageMagick (import)"
                            .to_string(),
                        true,
                    );
                    self.soft_effect = None;
                    self.soft_note.clear();
                    return;
                }
            },
            _ => return,
        };
        let delta = (r as i32 - self.last_soft_rgb[0] as i32)
            .abs()
            .max((g as i32 - self.last_soft_rgb[1] as i32).abs())
            .max((b as i32 - self.last_soft_rgb[2] as i32).abs());
        self.last_soft_apply = std::time::Instant::now();
        if delta < 4 {
            return;
        }
        match self
            .backend
            .set_kbd_rgb(0, r as u32, g as u32, b as u32, self.rgb_speed, true)
        {
            Ok(()) => self.last_soft_rgb = [r, g, b],
            Err(e) => {
                self.report(format!("Software frame failed: {}", friendly_error(&e)), true);
                self.soft_effect = None;
                self.music = None;
                self.soft_note.clear();
            }
        }
    }

    /// CPU temperature → colour: blue (cool) → green → amber → red (hot).
    fn smart_color(temp_c: f32) -> [u8; 3] {
        let t = temp_c.clamp(40.0, 90.0);
        let hue = if t <= 60.0 {
            210.0 - (t - 40.0) / 20.0 * 90.0
        } else {
            120.0 - (t - 60.0) / 30.0 * 120.0
        };
        Self::hsv_to_rgb(hue.rem_euclid(360.0), 1.0, 1.0)
    }

    /// Capture the screen average with ImageMagick and return it as sRGB.
    /// `import -resize 1x1!` yields a 16-bit-per-channel hex string.
    fn sample_screen_colour() -> Option<[u8; 3]> {
        let out = std::process::Command::new("import")
            .args([
                "-window",
                "root",
                "-resize",
                "1x1!",
                "-format",
                "%[hex:p{0,0}]",
                "info:",
            ])
            .output()
            .ok()?;
        if !out.status.success() {
            return None;
        }
        let text = String::from_utf8_lossy(&out.stdout);
        Self::parse_import_hex(&text).map(|c| {
            // keep a floor so a dark screen does not turn the keyboard off
            let (h, s, v) = Self::rgb_to_hsv(c);
            Self::hsv_to_rgb(h, s, v.max(0.30))
        })
    }

    /// Parse ImageMagick's 6- or 12-digit hex output (12 = 16-bit channels).
    fn parse_import_hex(text: &str) -> Option<[u8; 3]> {
        let hex: String = text.chars().filter(|c| c.is_ascii_hexdigit()).collect();
        let byte = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).ok();
        match hex.len() {
            6 => Some([byte(0)?, byte(2)?, byte(4)?]),
            12 => Some([byte(0)?, byte(4)?, byte(8)?]),
            _ => None,
        }
    }

    /// Map an audio level (0..1) to a colour: the picker hue with a level-driven
    /// brightness and a small hue shift, so quiet music glows dim and loud hits
    /// flash bright.
    fn music_color(hue: f32, sat: f32, level: f32) -> [u8; 3] {
        let level = level.clamp(0.0, 1.0);
        let h = (hue + (level - 0.5) * 24.0).rem_euclid(360.0);
        let v = 0.08 + 0.92 * level;
        Self::hsv_to_rgb(h, sat.max(0.70), v)
    }

    fn aura_mode_name(mode: u32) -> &'static str {
        match mode {
            0 => "Static",
            1 => "Breathing",
            2 => "Color Cycle",
            3 => "Rainbow",
            4 => "Star",
            5 => "Rain",
            6 => "Highlight",
            7 => "Laser",
            8 => "Ripple",
            10 => "Strobe",
            11 => "Comet",
            _ => "Effect",
        }
    }

    fn speed_percent(speed: u32) -> u32 {
        match speed {
            0xe1 => 33,
            0xf5 => 100,
            _ => 67,
        }
    }
}

/// Map HAL errors to user-accurate messages. The commonest case — a write
/// attempted while the app runs monitoring-only — must not blame the hardware.
fn friendly_error(e: &HalError) -> String {
    match e {
        HalError::Unsupported => {
            "writes are disabled — launch tuf-crate with --writes to enable hardware controls"
                .to_string()
        }
        HalError::Safety(se) => format!("safety check rejected this: {se}"),
        other => format!("{other}"),
    }
}

impl eframe::App for TufApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.last_poll.elapsed() >= std::time::Duration::from_secs(1) {
            self.poll();
            self.last_poll = std::time::Instant::now();
        }
        // request repaint so the polling continues; the Lighting tab animates
        // and software effects need a steady frame clock on every tab.
        if matches!(self.tab, Tab::Lighting) || self.soft_effect.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(33));
        } else {
            ctx.request_repaint_after(std::time::Duration::from_secs(1));
        }
        self.tick_soft_effect();

        egui::TopBottomPanel::top("header").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("🛡 tuf-crate");
                ui.label(format!("· model: {}", self.backend.capabilities().model));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let (txt, col) = if self.backend.name() == "daemon" {
                        ("controls: daemon", egui::Color32::from_rgb(120, 200, 140))
                    } else if self.writes_enabled {
                        ("writes ENABLED", egui::Color32::from_rgb(230, 170, 60))
                    } else {
                        ("monitoring-only", egui::Color32::from_rgb(120, 200, 140))
                    };
                    ui.colored_label(col, format!("● {txt}"));
                    ui.separator();
                    ui.label(format!("backend: {}", self.backend.name()));
                });
            });
        });

        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            if let Some((msg, err)) = &self.status_msg {
                ui.horizontal(|ui| {
                    if *err {
                        ui.colored_label(
                            egui::Color32::from_rgb(235, 110, 110),
                            format!("⚠ {msg}"),
                        );
                    } else {
                        ui.colored_label(
                            egui::Color32::from_rgb(130, 210, 140),
                            format!("✔ {msg}"),
                        );
                    }
                });
            }
            if let Some(err) = &self.telemetry_error {
                ui.colored_label(
                    egui::Color32::from_rgb(235, 110, 110),
                    format!("telemetry: {err}"),
                );
            }
        });

        egui::SidePanel::left("nav")
            .exact_width(190.0)
            .frame(egui::Frame::none().fill(egui::Color32::from_rgb(13, 14, 17)))
            .show(ctx, |ui| {
                ui.add_space(14.0);
                ui.colored_label(
                    egui::Color32::from_rgb(235, 140, 50),
                    egui::RichText::new("tuf-crate").size(19.0).strong(),
                );
                ui.colored_label(egui::Color32::GRAY, "TUF GAMING F17");
                ui.add_space(14.0);
                let items = [
                    (Tab::Home, "Home"),
                    (Tab::Dashboard, "Devices"),
                    (Tab::Performance, "Performance"),
                    (Tab::Lighting, "Lighting & Aura"),
                    (Tab::Fans, "Fans & Thermals"),
                    (Tab::Battery, "Battery"),
                    (Tab::DisplayGpu, "Display & GPU"),
                    (Tab::About, "Settings & About"),
                ];
                for (t, label) in items {
                    let selected =
                        self.tab == t || (t == Tab::Dashboard && self.tab == Tab::Dashboard);
                    let txt = egui::RichText::new(format!("  {label}")).size(15.0);
                    if ui
                        .add_sized(
                            [ui.available_width() - 12.0, 30.0],
                            egui::Button::new(txt)
                                .fill(if selected {
                                    egui::Color32::from_rgb(40, 30, 18)
                                } else {
                                    egui::Color32::TRANSPARENT
                                })
                                .stroke(if selected {
                                    egui::Stroke::new(1.0, egui::Color32::from_rgb(235, 140, 50))
                                } else {
                                    egui::Stroke::NONE
                                }),
                        )
                        .clicked()
                    {
                        self.tab = t;
                    }
                    ui.add_space(2.0);
                }
                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    ui.add_space(8.0);
                    ui.separator();
                    ui.colored_label(
                        egui::Color32::GRAY,
                        format!("v{}", env!("CARGO_PKG_VERSION")),
                    );
                    let (txt, col) = if self.backend.name() == "daemon" {
                        ("controls: daemon", egui::Color32::from_rgb(120, 200, 140))
                    } else if self.writes_enabled {
                        ("writes ENABLED", egui::Color32::from_rgb(230, 170, 60))
                    } else {
                        ("monitoring-only", egui::Color32::from_rgb(120, 200, 140))
                    };
                    ui.colored_label(col, format!("● {txt}"));
                    ui.label(format!("backend: {}", self.backend.name()));
                });
            });

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(egui::Color32::from_rgb(17, 18, 22)))
            .show(ctx, |ui| {
                ui.add_space(8.0);
                egui::ScrollArea::vertical().show(ui, |ui| match self.tab {
                    Tab::Home => self.ui_home(ui),
                    Tab::Dashboard => self.ui_dashboard(ui),
                    Tab::Performance => self.ui_performance(ui),
                    Tab::Fans => self.ui_fans(ui),
                    Tab::Battery => self.ui_battery(ui),
                    Tab::Lighting => self.ui_lighting(ui),
                    Tab::DisplayGpu => self.ui_display_gpu(ui),
                    Tab::About => self.ui_about(ui),
                });
            });
    }
}

#[derive(Default, Clone, Copy, PartialEq)]
enum Tab {
    #[default]
    Home,
    Dashboard,
    Performance,
    Fans,
    Battery,
    Lighting,
    DisplayGpu,
    About,
}

impl TufApp {
    fn temp_c(mc: Option<i64>) -> String {
        mc.map(|v| format!("{:.0} °C", v as f32 / 1000.0))
            .unwrap_or_else(|| "—".into())
    }

    /// Armoury-Crate-style Home: device header, System Usage card grid,
    /// Operating Mode row (Windows | Silent | Performance | Turbo).
    fn ui_home(&mut self, ui: &mut egui::Ui) {
        let Some(t) = self.telemetry.clone() else {
            return;
        };
        let caps = self.backend.capabilities().clone();

        // Device header card
        egui::Frame::none()
            .fill(egui::Color32::from_rgb(21, 23, 28))
            .rounding(6.0)
            .inner_margin(14.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.colored_label(
                        egui::Color32::from_rgb(235, 140, 50),
                        egui::RichText::new("ASUS TUF GAMING F17")
                            .size(22.0)
                            .strong(),
                    );
                });
                ui.colored_label(
                    egui::Color32::GRAY,
                    format!(
                        "{} · NVIDIA GeForce RTX 2050 GDDR6 @ 4GB",
                        caps.model.trim_start_matches("ASUS TUF Gaming F17 ")
                    ),
                );
            });
        ui.add_space(10.0);

        ui.horizontal(|ui| {
            ui.colored_label(
                egui::Color32::from_rgb(235, 140, 50),
                egui::RichText::new("⋙ System Usage").strong(),
            );
        });
        ui.add_space(4.0);

        // 2x2 usage cards: CPU | GPU / Fan | Memory
        egui::Grid::new("usage_cards")
            .num_columns(2)
            .spacing([12.0, 10.0])
            .show(ui, |ui| {
                // CPU card
                usage_card(ui, "CPU", |ui| {
                    ui.label("Frequency  —");
                    ui.label(format!(
                        "Usage  {}",
                        self.cpu_util
                            .map(|u| format!("{u} %"))
                            .unwrap_or_else(|| "—".into())
                    ));
                    ui.label(format!(
                        "Temperature  {}",
                        Self::temp_c(t.cpu_package_temp_mC)
                    ));
                    ui.colored_label(egui::Color32::GRAY, "MemClk  —");
                    ui.colored_label(egui::Color32::GRAY, "Vcore  —");
                });
                // GPU card
                usage_card(ui, "GPU", |ui| {
                    if let Some(g) = &t.nvidia {
                        ui.label(format!("Temp  {} °C", g.temperature_c));
                        ui.label(format!("Use  {} %", g.utilization_percent));
                        ui.label(format!("TGP  {:.1} W", g.power_watts));
                        ui.label(format!("Perf  {}", g.perf_state));
                    } else {
                        ui.colored_label(egui::Color32::GRAY, "GPU idle / powered down");
                    }
                    ui.colored_label(egui::Color32::GRAY, "MemClk  —");
                });
                // Fan card
                usage_card(ui, "Fan", |ui| {
                    ui.label(format!(
                        "CPU  {}",
                        t.cpu_fan_rpm
                            .map(|r| format!("{r} RPM"))
                            .unwrap_or_else(|| "—".into())
                    ));
                    ui.label(format!(
                        "GPU  {}",
                        t.gpu_fan_rpm
                            .map(|r| format!("{r} RPM"))
                            .unwrap_or_else(|| "—".into())
                    ));
                    if let Some(r) = t.cpu_fan_rpm {
                        ui.colored_label(
                            egui::Color32::GRAY,
                            format!("Fan Acoustics  ~{:.1} dBA (est.)", 44.8 * r as f32 / 5400.0),
                        );
                    }
                    ui.colored_label(
                        egui::Color32::GRAY,
                        "NVMe 47 °C class SSD telemetry on Fans tab",
                    );
                });
                // Memory card
                usage_card(ui, "Memory", |ui| {
                    if let (Some(u), Some(tot)) = (t.ram_used_gb, t.ram_total_gb) {
                        ui.label(format!("RAM  {u} / {tot} GB"));
                    }
                    if let (Some(u), Some(tot)) = (t.storage_used_gb, t.storage_total_gb) {
                        ui.label(format!("Storage  {u} / {tot} GB"));
                    }
                    if let Some(b) = &t.battery {
                        ui.label(format!("Battery  {} % · {}", b.capacity_percent, b.status));
                    }
                });
            });
        ui.add_space(12.0);

        // Operating Mode row (Armoury parity: Windows | Silent | Performance | Turbo)
        ui.horizontal(|ui| {
            ui.colored_label(
                egui::Color32::from_rgb(235, 140, 50),
                egui::RichText::new("⋙ Operating Mode").strong(),
            );
        });
        ui.add_space(4.0);
        let current = t.profile.clone().unwrap_or_default();
        let modes: [(&str, &str, Option<Profile>, &str); 4] = [
            (
                "Windows",
                "Handled by your desktop/TLP",
                None,
                "No Linux equivalent — tuf-crate never overrides system power management",
            ),
            (
                "Silent",
                "Low fan curve + reduced power",
                Some(Profile::Quiet),
                "platform_profile: quiet",
            ),
            (
                "Performance",
                "Balanced thermal policy",
                Some(Profile::Balanced),
                "platform_profile: balanced",
            ),
            (
                "Turbo",
                "Maximum fan + power policy",
                Some(Profile::Performance),
                "platform_profile: performance",
            ),
        ];
        ui.horizontal(|ui| {
            for (name, desc, profile, hint) in modes {
                let selected = profile.map(|p| current == p.as_str()).unwrap_or(false);
                let mut b = egui::Button::new(egui::RichText::new(name).size(14.0).strong())
                    .min_size(egui::vec2(118.0, 40.0));
                b = b.stroke(egui::Stroke::new(
                    if selected { 2.0 } else { 1.0 },
                    match profile {
                        Some(Profile::Quiet) => egui::Color32::from_rgb(90, 190, 190),
                        Some(Profile::Balanced) => egui::Color32::from_rgb(120, 200, 120),
                        Some(Profile::Performance) => egui::Color32::from_rgb(235, 140, 50),
                        None => egui::Color32::from_rgb(90, 90, 100),
                    },
                ));
                let resp = ui.add(b).on_hover_text(hint);
                if selected {
                    ui.painter().rect_stroke(
                        resp.rect,
                        4.0,
                        egui::Stroke::new(1.0, egui::Color32::from_rgb(235, 140, 50)),
                    );
                }
                let _ = desc;
                if profile.is_some() && resp.clicked() {
                    if let Some(p) = profile {
                        match self.backend.set_profile(p, true) {
                            Ok(()) => self.report(format!("profile -> {}", p.label()), false),
                            Err(e) => self.report(
                                format!("profile change failed: {}", friendly_error(&e)),
                                true,
                            ),
                        }
                        self.poll();
                    }
                }
            }
        });
        ui.add_space(6.0);
        ui.colored_label(
            egui::Color32::GRAY,
            "ARMOURY CRATE is a trademark of ASUSTeK Computer Inc. tuf-crate is an independent, unofficial Linux tool.",
        );
    }

    fn ui_dashboard(&mut self, ui: &mut egui::Ui) {
        let Some(t) = self.telemetry.clone() else {
            return;
        };
        ui.horizontal(|ui| {
            if let Some(c) = t.cpu_package_temp_mC {
                ring_gauge(
                    ui,
                    c as f32 / 100_000.0,
                    &Self::temp_c(Some(c)),
                    "CPU",
                    temp_color(c, 100_000),
                );
            }
            if let Some(g) = &t.nvidia {
                ring_gauge(
                    ui,
                    g.temperature_c as f32 / 97.0,
                    &format!("{} °C", g.temperature_c),
                    "GPU",
                    temp_color((g.temperature_c * 1000) as i64, 97_000),
                );
            }
            if let Some(r) = t.cpu_fan_rpm {
                ring_gauge(
                    ui,
                    r as f32 / 6000.0,
                    &format!("{r}"),
                    "CPU FAN",
                    egui::Color32::from_rgb(90, 160, 220),
                );
            }
            if let Some(r) = t.gpu_fan_rpm {
                ring_gauge(
                    ui,
                    r as f32 / 6000.0,
                    &format!("{r}"),
                    "GPU FAN",
                    egui::Color32::from_rgb(160, 110, 220),
                );
            }
            if let Some(b) = &t.battery {
                ring_gauge(
                    ui,
                    b.capacity_percent as f32 / 100.0,
                    &format!("{} %", b.capacity_percent),
                    "BATTERY",
                    if b.ac_online {
                        egui::Color32::from_rgb(120, 200, 120)
                    } else {
                        egui::Color32::from_rgb(235, 180, 60)
                    },
                );
            }
        });
        ui.add_space(6.0);
        egui::Grid::new("dash")
            .num_columns(2)
            .spacing([16.0, 6.0])
            .show(ui, |ui| {
                ui.label("Current profile");
                ui.label(t.profile.clone().unwrap_or_default());
                ui.end_row();
                ui.label("CPU (package)");
                ui.label(Self::temp_c(t.cpu_package_temp_mC));
                ui.end_row();
                ui.label("NVMe SSD");
                ui.label(Self::temp_c(t.nvme_temp_mC));
                ui.end_row();
                for (label, rpm) in &t.fan_labels {
                    ui.label(format!("Fan · {label}"));
                    ui.label(
                        rpm.map(|r| format!("{r} RPM"))
                            .unwrap_or_else(|| "—".into()),
                    );
                    ui.end_row();
                }
                if let Some(b) = &t.battery {
                    ui.label("Battery");
                    ui.label(format!(
                        "{} % · {} · AC {}",
                        b.capacity_percent,
                        b.status,
                        if b.ac_online { "on" } else { "off" }
                    ));
                    ui.end_row();
                }
                if let Some(g) = &t.nvidia {
                    ui.label("NVIDIA RTX 2050");
                    ui.label(format!(
                        "{} °C · {} % · {:.1} W · {}{}",
                        g.temperature_c,
                        g.utilization_percent,
                        g.power_watts,
                        g.perf_state,
                        if g.throttling.is_empty() {
                            String::new()
                        } else {
                            format!(" · ⚠ {:?}", g.throttling)
                        }
                    ));
                    ui.end_row();
                }
            });

        ui.add_space(12.0);
        ui.heading("Thermal picture");
        draw_temp_bar(ui, "CPU", t.cpu_package_temp_mC, 100_000);
        draw_temp_bar(ui, "NVMe", t.nvme_temp_mC, 70_000);
        if let Some(g) = &t.nvidia {
            draw_temp_bar(ui, "GPU", Some((g.temperature_c * 1000) as i64), 97_000);
        }
    }

    fn ui_performance(&mut self, ui: &mut egui::Ui) {
        let Some(t) = self.telemetry.clone() else {
            return;
        };
        let caps = self.backend.capabilities().clone();
        if !caps.platform_profile {
            ui.label("platform_profile not exposed by the running kernel — unavailable.");
            return;
        }
        ui.label("Armoury Crate operating modes map onto the kernel platform_profile:");
        ui.add_space(6.0);
        for p in Profile::all() {
            let selected = t.profile.as_deref() == Some(p.as_str());
            let (name, desc, color) = match p {
                Profile::Quiet => (
                    "SILENT",
                    "Reduced power — quiet fan curve",
                    egui::Color32::from_rgb(90, 190, 190),
                ),
                Profile::Balanced => (
                    "BALANCED",
                    "Default thermal policy",
                    egui::Color32::from_rgb(120, 200, 120),
                ),
                Profile::Performance => (
                    "PERFORMANCE (TURBO)",
                    "Maximum fan + power policy",
                    egui::Color32::from_rgb(235, 140, 50),
                ),
            };
            let card = egui::Button::new(
                egui::RichText::new(format!("{name}  —  {desc}"))
                    .size(15.0)
                    .strong(),
            )
            .min_size(egui::vec2(460.0, 46.0))
            .stroke(egui::Stroke::new(if selected { 2.5 } else { 1.0 }, color))
            .fill(if selected {
                egui::Color32::from_rgb(32, 34, 40)
            } else {
                egui::Color32::from_rgb(23, 25, 30)
            });
            if ui.add(card).clicked() {
                match self.backend.set_profile(p, true) {
                    Ok(()) => self.report(format!("profile -> {}", p.label()), false),
                    Err(e) => self.report(
                        format!("profile change failed: {}", friendly_error(&e)),
                        true,
                    ),
                }
                self.poll();
            }
            ui.add_space(4.0);
        }
        ui.add_space(10.0);
        ui.colored_label(egui::Color32::GRAY, "CPU governor/EPP remain managed by TLP — tuf-crate does not fight system power management (SAFETY.md).");
        ui.colored_label(egui::Color32::GRAY, "WMI PPT limits (SPL/SPPT/FPPT) are not implemented by this model's firmware — see docs/UNSUPPORTED.md.");
        ui.add_space(14.0);
        ui.heading("CPU power limit (sustained)");
        if self.backend.capabilities().cpu_power_limit {
            let current = self
                .telemetry
                .as_ref()
                .and_then(|t| t.pl1_watts)
                .unwrap_or(65);
            ui.label(format!(
                "Current long-term limit (PL1): {current} W   (short burst PL2: 83 W, untouched)"
            ));
            ui.add_space(4.0);
            let mut value = self.pending_pl1.unwrap_or(current);
            ui.add(egui::Slider::new(&mut value, 25..=65).text("sustained CPU power (W)"));
            self.pending_pl1 = Some(value);
            ui.colored_label(
                egui::Color32::from_rgb(235, 170, 90),
                "⚠ Lowering this caps sustained performance to reduce heat — games/compiles get slower. Short bursts (PL2) are unaffected.",
            );
            ui.colored_label(
                egui::Color32::GRAY,
                "Zero-risk by construction: you can only LOWER power (less heat), the range is clamped to 25–65 W, 65 W is the firmware default, every change is audited, and the limit resets on reboot.",
            );
            ui.horizontal(|ui| {
                for preset in [35u32, 45, 65] {
                    let name = match preset {
                        35 => "35 W (cool)",
                        45 => "45 W (nominal)",
                        _ => "65 W (default)",
                    };
                    if ui.button(name).clicked() {
                        self.pending_pl1 = Some(preset);
                    }
                }
                if ui.button("Apply").clicked() {
                    let w = self.pending_pl1.unwrap_or(current);
                    match self.backend.set_cpu_power_limit(w, true) {
                        Ok(()) => {
                            self.report(format!("CPU power limit -> {w} W"), false);
                            self.pending_pl1 = None;
                        }
                        Err(e) => {
                            self.report(format!("power limit failed: {}", friendly_error(&e)), true)
                        }
                    }
                    self.poll();
                }
            });
        } else {
            ui.colored_label(
                egui::Color32::GRAY,
                "intel-rapl interface not exposed — unavailable.",
            );
        }
        ui.add_space(14.0);
        ui.heading("24/7 persistence");
        ui.label(
            "Re-apply the current profile + power limit + fan mode at every boot (survives reboots):",
        );
        ui.horizontal(|ui| {
            if ui.button("Apply current settings at every boot").clicked() {
                let Some(t) = self.telemetry.clone() else {
                    return;
                };
                let mut cfg = serde_json::Map::new();
                if let Some(p) = &t.profile {
                    cfg.insert("profile".into(), serde_json::json!(p));
                }
                if let Some(w) = t.pl1_watts {
                    cfg.insert("pl1_watts".into(), serde_json::json!(w));
                }
                if let Some(f) = t.fan_full_speed {
                    if f {
                        cfg.insert("fan_full_speed".into(), serde_json::json!(true));
                    }
                }
                match self
                    .backend
                    .save_boot_config(serde_json::Value::Object(cfg))
                {
                    Ok(()) => self.report(
                        "settings will be re-applied at every boot".to_string(),
                        false,
                    ),
                    Err(e) => self.report(format!("persist failed: {}", friendly_error(&e)), true),
                }
                self.poll();
            }
            if ui.button("Remove").clicked() {
                match self.backend.clear_boot_config() {
                    Ok(()) => self.report("boot persistence removed".to_string(), false),
                    Err(e) => self.report(format!("clear failed: {}", friendly_error(&e)), true),
                }
                self.poll();
            }
        });
        match &self.boot_config {
            Some(v) => {
                ui.monospace(format!("persisted: {v}"));
            }
            None => {
                ui.monospace("persisted: (nothing — settings reset to firmware defaults at boot)");
            }
        }
    }

    fn ui_fans(&mut self, ui: &mut egui::Ui) {
        let Some(t) = self.telemetry.clone() else {
            return;
        };
        ui.heading("Fan monitoring");
        for (label, rpm) in &t.fan_labels {
            ui.horizontal(|ui| {
                ui.label(format!("{label}:"));
                ui.monospace(format!(
                    "{:>5} RPM",
                    rpm.map(|r| r.to_string()).unwrap_or_else(|| "—".into())
                ));
            });
        }
        ui.add_space(8.0);
        ui.heading("Thermal throttling indicators");
        if let Some(g) = &t.nvidia {
            if g.throttling.is_empty() {
                ui.label("GPU: no active throttle reasons");
            } else {
                for r in &g.throttling {
                    ui.colored_label(
                        egui::Color32::from_rgb(235, 170, 90),
                        format!("GPU throttle: {r}"),
                    );
                }
            }
        }
        if let Some(cpu) = t.cpu_package_temp_mC {
            if cpu >= 94_000 {
                ui.colored_label(egui::Color32::from_rgb(235, 110, 110),
                    format!("CPU at {:.0} °C — sustained high temperature. Hardware cooling service recommended (see audit report).", cpu as f32 / 1000.0));
            }
        }
        ui.add_space(10.0);
        if self.backend.capabilities().fan_full_speed {
            let on = t.fan_full_speed.unwrap_or(false);
            ui.heading("Max Cool");
            ui.horizontal(|ui| {
                let label = if on {
                    "MAX COOL ACTIVE — fans 100 %"
                } else {
                    "Force fans to 100 %"
                };
                let mut btn = egui::Button::new(egui::RichText::new(label).strong())
                    .min_size(egui::vec2(260.0, 38.0));
                if on {
                    btn = btn.stroke(egui::Stroke::new(
                        2.0,
                        egui::Color32::from_rgb(90, 160, 220),
                    ));
                }
                if ui.add(btn).clicked() {
                    match self.backend.set_fan_full_speed(!on, true) {
                        Ok(()) => self.report(
                            if on {
                                "fans -> auto"
                            } else {
                                "fans -> 100% (max cool)"
                            }
                            .to_string(),
                            false,
                        ),
                        Err(e) => {
                            self.report(format!("max cool failed: {}", friendly_error(&e)), true)
                        }
                    }
                    self.poll();
                }
                ui.colored_label(
                    egui::Color32::GRAY,
                    "(kernel FULLSPEED/AUTO mode — instantly reversible)",
                );
            });
        }
        ui.add_space(10.0);
        ui.colored_label(
            egui::Color32::GRAY,
            "Custom fan curves are NOT available on this model: the firmware acknowledges the WMI fan-curve device IDs but performs no curve I/O (docs/RESEARCH.md §5). Arbitrary PWM duty writes are rejected by the safety layer; FULLSPEED/AUTO is the kernel-documented control.");
    }

    fn ui_battery(&mut self, ui: &mut egui::Ui) {
        let Some(t) = self.telemetry.clone() else {
            return;
        };
        let Some(b) = t.battery.clone() else {
            ui.label("Battery interface not present.");
            return;
        };
        ui.heading("Battery health");
        ui.label(format!(
            "Charge: {} % · Status: {} · AC: {}",
            b.capacity_percent,
            b.status,
            if b.ac_online {
                "connected"
            } else {
                "on battery"
            }
        ));
        if let Some(h) = b.health_percent() {
            ui.label(format!("Health: {h} % of design capacity"));
        }
        if let Some(p) = b.power_now_uw {
            ui.label(format!("Power draw: {:.2} W", p as f32 / 1e6));
        }
        ui.add_space(10.0);
        ui.heading("Charge limit (battery longevity)");
        if !self.backend.capabilities().charge_threshold {
            ui.label("charge_control_end_threshold not exposed — unavailable.");
            return;
        }
        let current = b.charge_threshold.unwrap_or(100);
        let mut value = self.pending_threshold.unwrap_or(current);
        ui.add(egui::Slider::new(&mut value, 20..=100).text("stop charging at (%)"));
        self.pending_threshold = Some(value);
        ui.horizontal(|ui| {
            for preset in [60u32, 80, 100] {
                if ui.button(format!("{preset} %")).clicked() {
                    self.pending_threshold = Some(preset);
                    value = preset;
                }
            }
            if ui.button("Apply").clicked() {
                let v = self.pending_threshold.unwrap_or(current);
                match self.backend.set_charge_threshold(v, true) {
                    Ok(()) => {
                        self.report(format!("charge limit -> {v} %"), false);
                        self.pending_threshold = None;
                    }
                    Err(e) => {
                        self.report(format!("charge limit failed: {}", friendly_error(&e)), true)
                    }
                }
                self.poll();
            }
        });
        ui.colored_label(
            egui::Color32::GRAY,
            "Values are clamped to the kernel-verified range; every change is audited.",
        );
    }

    fn ui_lighting(&mut self, ui: &mut egui::Ui) {
        let Some(t) = self.telemetry.clone() else {
            return;
        };
        let now = ui.ctx().input(|i| i.time) as f32;
        let rate = match self.rgb_speed {
            0xe1 => 0.55,
            0xf5 => 1.7,
            _ => 1.0,
        };
        ui.horizontal(|ui| {
            ui.heading("Lighting & Aura");
            ui.colored_label(
                egui::Color32::from_rgb(120, 200, 120),
                "● LIVE · WMNB DEVS 0x00100056",
            );
        });
        ui.label(
            egui::RichText::new(
                "Full 0–255 RGB mixing · 11 Aura effects · speed 33/67/100 % · brightness",
            )
            .size(12.0)
            .color(egui::Color32::GRAY),
        );
        ui.add_space(6.0);

        // ---------- animated device preview (original art, drawn live) ----------
        self.lighting_preview(ui, now, rate);
        ui.add_space(8.0);

        // ---------- effect grid ----------
        ui.label("Effects");
        let modes: [(&str, u32, &str); 11] = [
            ("Static", 0, "Solid colour"),
            ("Breathing", 1, "Fade in / out"),
            ("Color Cycle", 2, "Sweep through the spectrum"),
            ("Rainbow", 3, "Animated rainbow across keys"),
            ("Star", 4, "Twinkling stars"),
            ("Rain", 5, "Falling drops"),
            ("Highlight", 6, "Travelling highlight band"),
            ("Laser", 7, "Sweeping beam"),
            ("Ripple", 8, "Expanding ripples"),
            ("Strobe", 10, "Fast flashing"),
            ("Comet", 11, "Comet with a fading tail"),
        ];
        egui::Grid::new("aura_modes")
            .num_columns(4)
            .spacing([6.0, 6.0])
            .show(ui, |ui| {
                for (i, (name, mode, desc)) in modes.iter().enumerate() {
                    let selected = self.rgb_mode == *mode;
                    let resp = ui.add_sized(
                        [126.0, 30.0],
                        egui::SelectableLabel::new(
                            selected,
                            egui::RichText::new(*name).size(13.0),
                        ),
                    );
                    let resp = resp.on_hover_text(*desc);
                    if resp.clicked() {
                        self.rgb_mode = *mode;
                        self.apply_rgb(*mode);
                    }
                    if i % 4 == 3 {
                        ui.end_row();
                    }
                }
                if !modes.len().is_multiple_of(4) {
                    ui.end_row();
                }
            });
        ui.add_space(4.0);

        // ---------- speed ----------
        ui.horizontal(|ui| {
            ui.label("Speed");
            for (val, label) in [(0xe1u32, "33 %"), (0xeb, "67 %"), (0xf5, "100 %")] {
                let selected = self.rgb_speed == val;
                if ui
                    .selectable_label(selected, label)
                    .on_hover_text("Animation speed (kernel TUF_KBD_RGB_SPEED_*)")
                    .clicked()
                {
                    self.rgb_speed = val;
                    self.apply_rgb(self.rgb_mode);
                }
            }
            ui.colored_label(
                egui::Color32::GRAY,
                "Static ignores speed; animated effects use it.",
            );
        });
        ui.add_space(8.0);

        // ---------- software effects (Linux-native) ----------
        ui.label("Software effects (Linux-native)");
        ui.horizontal(|ui| {
            let music_active = self.soft_effect == Some(music::MUSIC_EFFECT_ID);
            let smart_active = self.soft_effect == Some(music::SMART_EFFECT_ID);
            let adaptive_active = self.soft_effect == Some(music::ADAPTIVE_EFFECT_ID);
            let mut clicked: Option<u32> = None;
            for (id, active, label) in [
                (music::MUSIC_EFFECT_ID, music_active, "♪ Music"),
                (music::SMART_EFFECT_ID, smart_active, "🌡 Smart (temp)"),
                (music::ADAPTIVE_EFFECT_ID, adaptive_active, "🖥 Adaptive (screen)"),
            ] {
                let text = if active {
                    format!("⏹ {label}")
                } else {
                    label.to_string()
                };
                let fill = if active {
                    egui::Color32::from_rgb(45, 90, 60)
                } else {
                    egui::Color32::from_rgb(30, 32, 38)
                };
                if ui
                    .add_sized([170.0, 30.0], egui::Button::new(text).fill(fill))
                    .clicked()
                {
                    clicked = Some(if active { 0 } else { id });
                }
            }
            if music_active {
                let level = self.music.as_ref().map(|m| m.level()).unwrap_or(0);
                ui.add(
                    egui::ProgressBar::new(level as f32 / 1000.0)
                        .desired_width(110.0)
                        .text(format!("{level}")),
                );
            }
            if smart_active || adaptive_active {
                ui.colored_label(egui::Color32::from_rgb(120, 200, 120), &self.soft_note);
            }
            match clicked {
                Some(0) => self.stop_soft_effect(),
                Some(music::MUSIC_EFFECT_ID) => self.start_music(),
                Some(music::SMART_EFFECT_ID) => self.start_smart(),
                Some(music::ADAPTIVE_EFFECT_ID) => self.start_adaptive(),
                _ => {}
            }
        });
        ui.colored_label(
            egui::Color32::GRAY,
            if self.soft_effect.is_some() {
                "running — frames stream through the verified RGB path; click the active effect to stop"
            } else {
                "Music = desktop audio · Smart = CPU temperature · Adaptive = screen average"
            },
        );
        ui.add_space(8.0);

        // ---------- colour mixing ----------
        ui.heading("Colour mixing");
        ui.add_space(2.0);
        ui.horizontal(|ui| {
            // saturation/value square
            let sv_w = 200.0;
            let sv_h = 110.0;
            let (sv_rect, sv_resp) =
                ui.allocate_exact_size(egui::vec2(sv_w, sv_h), egui::Sense::click_and_drag());
            let p = ui.painter_at(sv_rect);
            let nx = 40;
            let ny = 24;
            for xi in 0..nx {
                for yi in 0..ny {
                    let sat = xi as f32 / (nx - 1) as f32;
                    let val = 1.0 - yi as f32 / (ny - 1) as f32;
                    let c = Self::hsv_to_rgb(self.rgb_hue, sat, val);
                    let r = egui::Rect::from_min_max(
                        egui::pos2(
                            sv_rect.left() + sv_rect.width() * xi as f32 / nx as f32,
                            sv_rect.top() + sv_rect.height() * yi as f32 / ny as f32,
                        ),
                        egui::pos2(
                            sv_rect.left() + sv_rect.width() * (xi + 1) as f32 / nx as f32 + 0.5,
                            sv_rect.top() + sv_rect.height() * (yi + 1) as f32 / ny as f32 + 0.5,
                        ),
                    );
                    p.rect_filled(r, 0.0, egui::Color32::from_rgb(c[0], c[1], c[2]));
                }
            }
            let marker = egui::pos2(
                sv_rect.left() + self.rgb_sat * sv_rect.width(),
                sv_rect.top() + (1.0 - self.rgb_val) * sv_rect.height(),
            );
            p.circle_stroke(marker, 5.0, egui::Stroke::new(2.0, egui::Color32::WHITE));
            if sv_resp.clicked() || sv_resp.dragged() {
                if let Some(pos) = sv_resp.interact_pointer_pos() {
                    self.rgb_sat =
                        ((pos.x - sv_rect.left()) / sv_rect.width()).clamp(0.0, 1.0);
                    self.rgb_val = 1.0
                        - ((pos.y - sv_rect.top()) / sv_rect.height()).clamp(0.0, 1.0);
                    self.rgb_color = Self::hsv_to_rgb(self.rgb_hue, self.rgb_sat, self.rgb_val);
                }
            }
            ui.vertical(|ui| {
                // hue strip
                let (strip_rect, strip_resp) = ui.allocate_exact_size(
                    egui::vec2(220.0, 20.0),
                    egui::Sense::click(),
                );
                let p = ui.painter_at(strip_rect);
                for i in 0..72 {
                    let hue = i as f32 * 5.0;
                    let c = Self::hsv_to_rgb(hue, 1.0, 1.0);
                    p.rect_filled(
                        egui::Rect::from_min_size(
                            egui::pos2(
                                strip_rect.left() + i as f32 * strip_rect.width() / 72.0,
                                strip_rect.top(),
                            ),
                            egui::vec2(strip_rect.width() / 72.0 + 0.5, 20.0),
                        ),
                        0.0,
                        egui::Color32::from_rgb(c[0], c[1], c[2]),
                    );
                }
                let marker_x = strip_rect.left()
                    + (self.rgb_hue / 360.0) * strip_rect.width();
                p.line_segment(
                    [
                        egui::pos2(marker_x, strip_rect.top() - 2.0),
                        egui::pos2(marker_x, strip_rect.bottom() + 2.0),
                    ],
                    egui::Stroke::new(2.0, egui::Color32::WHITE),
                );
                if strip_resp.clicked() {
                    if let Some(pos) = strip_resp.interact_pointer_pos() {
                        self.rgb_hue = ((pos.x - strip_rect.left())
                            / strip_rect.width())
                        .clamp(0.0, 0.9999)
                            * 360.0;
                        self.rgb_color =
                            Self::hsv_to_rgb(self.rgb_hue, self.rgb_sat, self.rgb_val);
                    }
                }
                ui.add_space(6.0);
                // presets
                ui.label("Presets");
                ui.horizontal_wrapped(|ui| {
                    for (name, rgb) in [
                        ("White", [255u8, 255, 255]),
                        ("Warm", [255, 170, 90]),
                        ("Red", [255, 0, 0]),
                        ("Amber", [255, 140, 0]),
                        ("Lime", [160, 255, 0]),
                        ("Green", [0, 255, 0]),
                        ("Cyan", [0, 255, 255]),
                        ("Blue", [0, 0, 255]),
                        ("Violet", [150, 0, 255]),
                        ("Pink", [255, 0, 120]),
                    ] {
                        let c = egui::Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
                        if ui
                            .add(
                                egui::Button::new(egui::RichText::new(" ").size(9.0))
                                    .fill(c)
                                    .min_size(egui::vec2(22.0, 20.0)),
                            )
                            .on_hover_text(name)
                            .clicked()
                        {
                            self.rgb_color = rgb;
                            let (h, sa, v) = Self::rgb_to_hsv(rgb);
                            self.rgb_hue = h;
                            self.rgb_sat = sa;
                            self.rgb_val = v;
                        }
                    }
                });
            });
        });
        ui.add_space(6.0);

        // ---------- numeric mixing ----------
        let mut col = self.rgb_color;
        let mut edited = false;
        egui::Grid::new("rgb_sliders")
            .num_columns(3)
            .spacing([10.0, 4.0])
            .show(ui, |ui| {
                ui.label("R");
                if ui.add(egui::Slider::new(&mut col[0], 0..=255)).changed() {
                    edited = true;
                }
                ui.end_row();
                ui.label("G");
                if ui.add(egui::Slider::new(&mut col[1], 0..=255)).changed() {
                    edited = true;
                }
                ui.end_row();
                ui.label("B");
                if ui.add(egui::Slider::new(&mut col[2], 0..=255)).changed() {
                    edited = true;
                }
                ui.end_row();
                ui.label("HEX");
                let mut hex = format!("#{:02X}{:02X}{:02X}", col[0], col[1], col[2]);
                let he = ui.add(egui::TextEdit::singleline(&mut hex).desired_width(90.0));
                if he.changed() {
                    let h = hex.trim().trim_start_matches('#').to_string();
                    if h.len() == 6 && h.chars().all(|c| c.is_ascii_hexdigit()) {
                        col = [
                            u8::from_str_radix(&h[0..2], 16).unwrap_or(0),
                            u8::from_str_radix(&h[2..4], 16).unwrap_or(0),
                            u8::from_str_radix(&h[4..6], 16).unwrap_or(0),
                        ];
                        edited = true;
                    }
                }
                ui.end_row();
            });
        if edited {
            self.rgb_color = col;
            let (h, sa, v) = Self::rgb_to_hsv(col);
            self.rgb_hue = h;
            self.rgb_sat = sa;
            self.rgb_val = v;
        }
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            let (pr, _) = ui.allocate_exact_size(egui::vec2(72.0, 34.0), egui::Sense::hover());
            ui.painter_at(pr).rect_filled(
                pr,
                4.0,
                egui::Color32::from_rgb(col[0], col[1], col[2]),
            );
            if ui
                .add_sized(
                    [170.0, 34.0],
                    egui::Button::new("🎨 Apply to keyboard").fill(egui::Color32::from_rgb(
                        45, 90, 60,
                    )),
                )
                .on_hover_text("Send the selected effect + colour + speed to the keyboard")
                .clicked()
            {
                self.apply_rgb(self.rgb_mode);
            }
            let swatch_hex = format!("{:02X}{:02X}{:02X}", col[0], col[1], col[2]);
            ui.label(format!(
                "{} · #{} · speed {} %",
                Self::aura_mode_name(self.rgb_mode),
                swatch_hex,
                Self::speed_percent(self.rgb_speed)
            ));
        });
        ui.add_space(8.0);

        // ---------- profiles ----------
        ui.horizontal(|ui| {
            ui.label("Profile name:");
            ui.add(
                egui::TextEdit::singleline(&mut self.rgb_profile_name)
                    .desired_width(120.0)
                    .hint_text("e.g. Night Red"),
            );
            if ui.button("💾 Save local profile").clicked() {
                let name = self.rgb_profile_name.trim().to_string();
                if name.is_empty() {
                    self.report("enter a profile name first".to_string(), true);
                } else if let Err(e) =
                    crate::profiles::save_profile(&name, self.rgb_color, self.rgb_hue)
                {
                    self.report(format!("save failed: {e}"), true);
                } else {
                    self.report(format!("local profile '{name}' saved"), false);
                    self.rgb_profile_name.clear();
                }
            }
        });
        let profiles = crate::profiles::load_all();
        if profiles.is_empty() {
            ui.colored_label(
                egui::Color32::GRAY,
                "No local profiles yet — pick a colour, name it, save it.",
            );
        } else {
            ui.horizontal_wrapped(|ui| {
                for pr in &profiles {
                    let c = egui::Color32::from_rgb(pr.rgb[0], pr.rgb[1], pr.rgb[2]);
                    let swatch =
                        egui::Button::new(egui::RichText::new(format!("■ {}", pr.name)).size(13.0));
                    if ui
                        .add(swatch.fill(c))
                        .on_hover_text(format!(
                            "#{:02X}{:02X}{:02X} — click to load into the picker",
                            pr.rgb[0], pr.rgb[1], pr.rgb[2]
                        ))
                        .clicked()
                    {
                        self.rgb_color = pr.rgb;
                        let (h, sa, v) = Self::rgb_to_hsv(pr.rgb);
                        self.rgb_hue = h;
                        self.rgb_sat = sa;
                        self.rgb_val = v;
                    }
                    if ui
                        .small_button("✕")
                        .on_hover_text("delete profile")
                        .clicked()
                    {
                        let _ = crate::profiles::delete_profile(&pr.name);
                    }
                }
            });
        }
        ui.add_space(10.0);

        // ---------- brightness ----------
        ui.heading("Brightness");
        let max = t.kbd_max_brightness.unwrap_or(3);
        let cur = t.kbd_brightness.unwrap_or(0);
        ui.horizontal(|ui| {
            for (lvl, pct) in [(0u32, "0 %"), (1, "33 %"), (2, "66 %"), (3, "100 %")] {
                if lvl > max {
                    break;
                }
                if ui
                    .selectable_label(cur == lvl, pct.to_string())
                    .on_hover_text(format!("kernel kbd level {lvl}"))
                    .clicked()
                {
                    if let Err(e) = self.backend.set_kbd_brightness(lvl, true) {
                        self.report(format!("brightness failed: {}", friendly_error(&e)), true);
                    }
                    self.poll();
                }
            }
        });
        ui.colored_label(
            egui::Color32::GRAY,
            "Brightness is the kernel asus::kbd_backlight LED (same 33/66/100 % levels as Armoury Crate).",
        );
        ui.add_space(10.0);
        ui.colored_label(
            egui::Color32::GRAY,
            "Effects supported by this firmware: Static, Breathing, Color Cycle, Rainbow, Star, Rain, Highlight, Laser, Ripple, Strobe, Comet. Music / Smart / Adaptive / AI effects are Armoury-Crate-side features with no firmware hook on this model.",
        );
    }

    /// Animated, original-art keyboard preview. Colours are computed from the
    /// selected Aura mode and the RGB picker state.
    fn lighting_preview(&self, ui: &mut egui::Ui, t: f32, rate: f32) {
        let w = ui.available_width().min(560.0);
        let (rect, _) = ui.allocate_exact_size(egui::vec2(w, 150.0), egui::Sense::hover());
        let p = ui.painter_at(rect);
        p.rect_filled(rect, 12.0, egui::Color32::from_rgb(13, 13, 17));
        let (bh, bs, bv) = Self::rgb_to_hsv(self.rgb_color);
        // glow
        for i in (0..7).rev() {
            let f = i as f32 / 7.0;
            let glow_rect = egui::Rect::from_center_size(
                rect.center() + egui::vec2(0.0, 34.0),
                egui::vec2(w * (0.55 + f * 0.3), 90.0 + f * 60.0),
            );
            let c = Self::hsv_to_rgb(bh, bs, bv.min(0.55));
            p.rect_filled(
                glow_rect,
                20.0,
                egui::Color32::from_rgba_unmultiplied(c[0], c[1], c[2], (10.0 * (1.0 - f)) as u8),
            );
        }
        // laptop body
        let body = egui::Rect::from_center_size(
            rect.center() + egui::vec2(0.0, 8.0),
            egui::vec2(w * 0.78, 128.0),
        );
        p.rect_filled(body, 10.0, egui::Color32::from_rgb(28, 28, 34));
        let screen = egui::Rect::from_min_size(
            body.min + egui::vec2(14.0, 10.0),
            egui::vec2(body.width() - 28.0, 38.0),
        );
        p.rect_filled(screen, 4.0, egui::Color32::from_rgb(9, 12, 18));
        p.text(
            screen.center(),
            egui::Align2::CENTER_CENTER,
            "TUF GAMING F17 · tuf-crate",
            egui::FontId::proportional(13.0),
            egui::Color32::from_rgb(120, 200, 140),
        );
        // keyboard deck
        let deck = egui::Rect::from_min_size(
            egui::pos2(screen.left(), screen.bottom() + 8.0),
            egui::vec2(screen.width(), 64.0),
        );
        p.rect_filled(deck, 5.0, egui::Color32::from_rgb(20, 20, 24));
        let rows = 5usize;
        let cols = 15usize;
        let pad = 8.0;
        let gap = 3.0;
        let kw = (deck.width() - 2.0 * pad - (cols as f32 - 1.0) * gap) / cols as f32;
        let kh = (deck.height() - 2.0 * pad - (rows as f32 - 1.0) * gap) / rows as f32;
        for row in 0..rows {
            for col in 0..cols {
                let idx = row * cols + col;
                let (h, sa, v) = Self::effect_hsv(
                    EffectParams {
                        mode: self.rgb_mode,
                        bh,
                        bs,
                        bv,
                        cols,
                        t,
                        rate,
                    },
                    idx,
                );
                let [r, g, b] = Self::hsv_to_rgb(h, sa, v);
                let kr = egui::Rect::from_min_size(
                    egui::pos2(
                        deck.left() + pad + col as f32 * (kw + gap),
                        deck.top() + pad + row as f32 * (kh + gap),
                    ),
                    egui::vec2(kw, kh),
                );
                p.rect_filled(kr, 2.0, egui::Color32::from_rgb(r, g, b));
            }
        }
    }

    /// Per-key colour for the animated preview. Returns HSV (h 0..360).
    fn effect_hsv(p: EffectParams, key: usize) -> (f32, f32, f32) {
        let EffectParams {
            mode,
            bh,
            bs,
            bv,
            cols,
            t,
            rate,
        } = p;
        let phase = key as f32 / cols.max(1) as f32;
        let h = |x: f32| x.rem_euclid(360.0);
        match mode {
            0 => (bh, bs, bv),
            1 => {
                let k = 0.22 + 0.78 * (0.5 + 0.5 * (t * 1.8 * rate).sin());
                (bh, bs, bv * k)
            }
            2 => (h(bh + t * 45.0 * rate), 1.0, 1.0),
            3 => (h(phase * 360.0 + t * 90.0 * rate), 1.0, 1.0),
            4 => {
                let seed = ((key as u32).wrapping_mul(2654435761) % 1000) as f32 / 1000.0;
                let tw = (0.5 + 0.5 * (t * 2.5 * rate + seed * std::f32::consts::TAU).sin()).powi(5);
                (bh, bs, (0.12 + 0.88 * tw) * bv)
            }
            5 => {
                let col = (key % cols) as f32;
                let fall = ((col * 0.55 - t * 4.0 * rate) % 6.0 + 6.0) % 6.0;
                let k = (1.0 - fall / 2.0).max(0.0);
                (bh, bs, (0.10 + 0.90 * k) * bv)
            }
            6 => {
                let pos = (t * 0.6 * rate) % 1.0;
                let k = (1.0 - (phase - pos).abs() * 6.0).max(0.0);
                (bh, bs, (0.12 + 0.88 * k) * bv)
            }
            7 => {
                let pos = (t * 0.9 * rate) % 1.0;
                let k = (1.0 - (phase - pos).abs() * 4.0).max(0.0);
                (bh, bs, (0.10 + 0.90 * k) * bv)
            }
            8 => {
                let wave = ((t * 1.4 * rate) % 1.2) - 0.1;
                let k = (1.0 - ((phase - 0.5).abs() - wave).abs() * 5.0).max(0.0);
                (bh, bs, (0.10 + 0.90 * k) * bv)
            }
            10 => {
                let on = (t * 7.0 * rate).sin() > 0.0;
                (bh, bs, if on { bv } else { bv * 0.05 })
            }
            11 => {
                let head = (t * 1.2 * rate) % 1.0;
                let d = (head - phase).rem_euclid(1.0);
                let k = (1.0 - d * 2.5).max(0.0);
                (bh, bs, (0.08 + 0.92 * k) * bv)
            }
            _ => (bh, bs, bv),
        }
    }

    /// RGB bytes → HSV (h 0..360, s/v 0..1).
    fn rgb_to_hsv(c: [u8; 3]) -> (f32, f32, f32) {
        let r = c[0] as f32 / 255.0;
        let g = c[1] as f32 / 255.0;
        let b = c[2] as f32 / 255.0;
        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let d = max - min;
        let h = if d == 0.0 {
            0.0
        } else if max == r {
            60.0 * (((g - b) / d) % 6.0)
        } else if max == g {
            60.0 * ((b - r) / d + 2.0)
        } else {
            60.0 * ((r - g) / d + 4.0)
        };
        let s = if max == 0.0 { 0.0 } else { d / max };
        (h.rem_euclid(360.0), s, max)
    }

    /// HSV(360,1,1) → RGB bytes.
    fn hsv_to_rgb(hue: f32, sat: f32, val: f32) -> [u8; 3] {
        let c = val * sat;
        let x = c * (1.0 - ((hue / 60.0) % 2.0 - 1.0).abs());
        let m = val - c;
        let (r, g, b) = match hue as u32 {
            0..=59 => (c, x, 0.0),
            60..=119 => (x, c, 0.0),
            120..=179 => (0.0, c, x),
            180..=239 => (0.0, x, c),
            240..=299 => (x, 0.0, c),
            _ => (c, 0.0, x),
        };
        [
            ((r + m) * 255.0).round() as u8,
            ((g + m) * 255.0).round() as u8,
            ((b + m) * 255.0).round() as u8,
        ]
    }
    fn ui_display_gpu(&mut self, ui: &mut egui::Ui) {
        let Some(t) = self.telemetry.clone() else {
            return;
        };
        ui.heading("Display");
        if let Some(od) = t.panel_od {
            ui.horizontal(|ui| {
                ui.label("Panel overdrive (144 Hz response optimisation):");
                let mut on = od;
                if ui.checkbox(&mut on, "").changed() {
                    if let Err(e) = self.backend.set_panel_overdrive(on, true) {
                        self.report(format!("panel_od failed: {}", friendly_error(&e)), true);
                    }
                    self.poll();
                }
            });
        }
        ui.colored_label(egui::Color32::GRAY, "Refresh rate switching (60/144 Hz): available via your desktop display settings; xrandr integration is read-only here.");
        ui.add_space(10.0);
        ui.heading("GPU");
        if let Some(g) = &t.nvidia {
            ui.label(format!(
                "RTX 2050 · {} °C · util {} % · {:.1} W · perf {}",
                g.temperature_c, g.utilization_percent, g.power_watts, g.perf_state
            ));
        } else {
            ui.label("NVIDIA GPU not visible (powered down in offload mode, or driver absent).");
        }
        ui.add_space(6.0);
        ui.label("dGPU offload launcher:");
        ui.colored_label(
            egui::Color32::GRAY,
            "prime-run <application>   (wrapper installed during the system audit)",
        );
        ui.add_space(10.0);
        ui.heading("dGPU disable — hardware-level");
        let caps = self.backend.capabilities();
        if !caps.dgpu_disable {
            ui.label("dgpu_disable not exposed — unavailable.");
            return;
        }
        let disabled = t.dgpu_disable.unwrap_or(false);
        ui.label(format!(
            "Current state: {}",
            if disabled {
                "dGPU DISABLED"
            } else {
                "dGPU enabled (hybrid)"
            }
        ));
        if !self.confirm_dgpu {
            if ui
                .button(if disabled {
                    "Re-enable dGPU"
                } else {
                    "Disable dGPU…"
                })
                .clicked()
            {
                self.confirm_dgpu = true;
            }
        } else {
            ui.colored_label(egui::Color32::from_rgb(235, 170, 90),
                "This switches the hardware GPU mode via the ASUS WMI interface. The dGPU disappears until re-enabled; open applications using it will fail. Requires reboot to fully apply.");
            ui.horizontal(|ui| {
                if ui.button("Confirm").clicked() {
                    let res = self.backend.set_dgpu_disabled(!disabled, true);
                    match res {
                        Ok(()) => self.report("dGPU mode change applied".to_string(), false),
                        Err(e) => self
                            .report(format!("dgpu_disable failed: {}", friendly_error(&e)), true),
                    }
                    self.confirm_dgpu = false;
                    self.poll();
                }
                if ui.button("Cancel").clicked() {
                    self.confirm_dgpu = false;
                }
            });
        }
    }

    fn ui_about(&mut self, ui: &mut egui::Ui) {
        let caps = self.backend.capabilities();
        ui.heading("Detected capabilities");
        egui::Grid::new("caps")
            .num_columns(2)
            .spacing([16.0, 4.0])
            .show(ui, |ui| {
                ui.label("Model");
                ui.label(&caps.model);
                ui.end_row();
                ui.label("Model verified (FX706HF)");
                ui.label(if caps.is_fx706hf {
                    "yes"
                } else {
                    "NO — write ops still gated per-attr"
                });
                ui.end_row();
                ui.label("platform_profile");
                ui.label(if caps.platform_profile {
                    caps.profile_choices.join(", ")
                } else {
                    "—".into()
                });
                ui.end_row();
                ui.label("charge threshold");
                ui.label(yes_no(caps.charge_threshold));
                ui.end_row();
                ui.label("keyboard backlight");
                ui.label(yes_no(caps.kbd_backlight));
                ui.end_row();
                ui.label("panel overdrive");
                ui.label(yes_no(caps.panel_overdrive));
                ui.end_row();
                ui.label("dgpu_disable");
                ui.label(yes_no(caps.dgpu_disable));
                ui.end_row();
                ui.label("firmware-attributes (asus-armoury)");
                ui.label(if caps.firmware_attrs.is_empty() {
                    "—".into()
                } else {
                    caps.firmware_attrs.join(", ")
                });
                ui.end_row();
                ui.label("fans");
                ui.label(yes_no(!caps.fans.is_empty()));
                ui.end_row();
                ui.label("nvidia-smi");
                ui.label(yes_no(caps.nvidia));
                ui.end_row();
            });
        ui.add_space(10.0);
        ui.label("tuf-crate builds exclusively on kernel-sanctioned interfaces (asus-wmi / asus-armoury / leds / hwmon / power_supply). It does not write EC registers, invoke ACPI methods, patch kernels, or touch fan PWM controls. Every accepted write is clamped, capability-gated and audited. See docs/SAFETY.md.");
    }
}

fn yes_no(b: bool) -> &'static str {
    if b {
        "yes"
    } else {
        "no"
    }
}

fn draw_temp_bar(ui: &mut egui::Ui, label: &str, temp_mdeg: Option<i64>, warn_mdeg: i64) {
    ui.horizontal(|ui| {
        ui.monospace(format!("{label:>6}"));
        let Some(v) = temp_mdeg else {
            ui.label("—");
            return;
        };
        let c = v as f32 / 1000.0;
        let frac = (v as f32 / warn_mdeg as f32).clamp(0.0, 1.0);
        let bar = egui::widgets::ProgressBar::new(frac)
            .show_percentage()
            .text(format!("{c:.0} °C"));
        let resp = ui.add_sized([380.0, 16.0], bar);
        if v >= warn_mdeg {
            resp.on_hover_text("At/above the warning threshold");
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hsv_roundtrip_within_one_lsb() {
        for rgb in [
            [255u8, 0, 0],
            [0, 255, 0],
            [0, 0, 255],
            [255, 255, 0],
            [0, 255, 255],
            [255, 0, 255],
            [255, 255, 255],
            [0, 0, 0],
            [128, 64, 32],
            [12, 200, 90],
        ] {
            let (h, s, v) = TufApp::rgb_to_hsv(rgb);
            let back = TufApp::hsv_to_rgb(h, s, v);
            for (a, b) in rgb.iter().zip(back.iter()) {
                assert!(
                    (*a as i32 - *b as i32).abs() <= 1,
                    "roundtrip {rgb:?} -> {back:?}"
                );
            }
        }
    }

    #[test]
    fn speed_and_mode_labels() {
        assert_eq!(TufApp::speed_percent(0xe1), 33);
        assert_eq!(TufApp::speed_percent(0xeb), 67);
        assert_eq!(TufApp::speed_percent(0xf5), 100);
        assert_eq!(TufApp::aura_mode_name(0), "Static");
        assert_eq!(TufApp::aura_mode_name(3), "Rainbow");
        assert_eq!(TufApp::aura_mode_name(99), "Effect");
    }

    #[test]
    fn smart_colour_goes_blue_to_red() {
        let cool = TufApp::smart_color(40.0);
        let hot = TufApp::smart_color(90.0);
        assert!(cool[2] > cool[0], "cool should be blue-ish: {cool:?}");
        assert!(hot[0] > hot[2], "hot should be red-ish: {hot:?}");
    }

    #[test]
    fn parses_imagemagick_hex() {
        assert_eq!(TufApp::parse_import_hex("ff8000"), Some([255, 128, 0]));
        assert_eq!(
            TufApp::parse_import_hex("ff000000ffff"),
            Some([255, 0, 255])
        );
        assert_eq!(TufApp::parse_import_hex("nonsense"), None);
    }

    #[test]
    fn music_colour_brightens_with_level() {
        let dim = TufApp::music_color(200.0, 1.0, 0.0);
        let loud = TufApp::music_color(200.0, 1.0, 1.0);
        assert!(loud.iter().map(|v| *v as u32).sum::<u32>() > dim.iter().map(|v| *v as u32).sum::<u32>());
        for level in [0.0f32, 0.25, 0.5, 0.75, 1.0] {
            let c = TufApp::music_color(350.0, 0.8, level);
            // hue shift must stay in gamut and produce a valid colour
            assert!(c.iter().any(|v| *v > 0));
        }
    }

    #[test]
    fn effect_hsv_stays_in_range_for_every_mode() {
        for mode in [0u32, 1, 2, 3, 4, 5, 6, 7, 8, 10, 11] {
            for key in 0..75 {
                let (h, s, v) = TufApp::effect_hsv(
                    EffectParams {
                        mode,
                        bh: 200.0,
                        bs: 0.8,
                        bv: 0.9,
                        cols: 15,
                        t: 1.234,
                        rate: 1.0,
                    },
                    key,
                );
                assert!((0.0..360.0).contains(&h), "h={h} mode={mode}");
                assert!((0.0..=1.0).contains(&s), "s={s} mode={mode}");
                assert!((0.0..=1.0).contains(&v), "v={v} mode={mode}");
            }
        }
    }
}
