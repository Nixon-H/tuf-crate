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
            tab: Tab::Dashboard,
        };
        app.poll();
        app
    }

    fn poll(&mut self) {
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
        // request repaint so the polling continues
        ctx.request_repaint_after(std::time::Duration::from_secs(1));

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

        egui::SidePanel::left("nav").show(ctx, |ui| {
            ui.add_space(4.0);
        });
        // note: placeholder panel kept minimal; tabs below

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                let mut tab = self.tab;
                for (t, label) in [
                    (Tab::Dashboard, "Dashboard"),
                    (Tab::Performance, "Performance"),
                    (Tab::Fans, "Fans & Thermals"),
                    (Tab::Battery, "Battery"),
                    (Tab::Lighting, "Lighting"),
                    (Tab::DisplayGpu, "Display & GPU"),
                    (Tab::About, "About"),
                ] {
                    ui.selectable_value(&mut tab, t, label);
                    ui.separator();
                }
                self.tab = tab;
            });
            ui.add_space(8.0);
            egui::ScrollArea::vertical().show(ui, |ui| match self.tab {
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
        ui.heading("Keyboard backlight");
        if !self.backend.capabilities().kbd_backlight {
            ui.label("asus::kbd_backlight not exposed — unavailable.");
            return;
        }
        let max = t.kbd_max_brightness.unwrap_or(3);
        let cur = t.kbd_brightness.unwrap_or(0);
        ui.horizontal(|ui| {
            for lvl in 0..=max {
                if ui.selectable_label(cur == lvl, format!("{lvl}")).clicked() {
                    if let Err(e) = self.backend.set_kbd_brightness(lvl, true) {
                        self.report(format!("brightness failed: {}", friendly_error(&e)), true);
                    }
                    self.poll();
                }
            }
        });
        ui.add_space(8.0);
        ui.colored_label(egui::Color32::GRAY, "Static colour: the embedded controller keeps the last colour set by Armoury Crate in Windows — set it once there; brightness is controllable here.");
        ui.colored_label(egui::Color32::GRAY, "Dynamic RGB effects: this firmware rejects the WMI RGB interface (0xFFFFFFFE) — unsupported, with evidence (docs/RESEARCH.md §4).");
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
