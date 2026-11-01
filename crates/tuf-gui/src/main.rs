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

pub mod assets;
pub mod games;
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

    let opts = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1800.0, 1000.0])
            .with_min_inner_size([1100.0, 680.0]),
        ..Default::default()
    };
    eframe::run_native(
        "tuf-crate — TUF FX706HF Control Center",
        opts,
        Box::new(move |cc| {
            egui_extras_style(cc);
            assets::load_fonts(&cc.egui_ctx);
            let logo = assets::load_texture(&cc.egui_ctx, "branding/AC_Logo.png");
            Ok(Box::new(TufApp::new(backend, writes, &cc.egui_ctx, logo)))
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
    v.selection.stroke = egui::Stroke::new(1.0, accent());
    v.widgets.inactive.bg_fill = egui::Color32::from_rgb(28, 30, 36);
    v.widgets.inactive.weak_bg_fill = egui::Color32::from_rgb(28, 30, 36);
    v.widgets.hovered.bg_fill = egui::Color32::from_rgb(40, 42, 48);
    v.widgets.hovered.weak_bg_fill = egui::Color32::from_rgb(40, 42, 48);
    v.widgets.active.bg_fill = egui::Color32::from_rgb(64, 44, 20);
    cc.egui_ctx.set_visuals(v);
}

fn thin_stroke(col: egui::Color32) -> egui::Stroke {
    egui::Stroke::new(1.1, col)
}

/// Armoury-style line icon for the sidebar (drawn, no external assets).
fn nav_icon(p: &egui::Painter, c: egui::Pos2, kind: u8, col: egui::Color32) {
    let s = egui::Stroke::new(1.6, col);
    match kind {
        0 => {
            // home
            p.add(egui::Shape::line(
                vec![
                    egui::pos2(c.x - 8.0, c.y - 1.0),
                    egui::pos2(c.x, c.y - 9.0),
                    egui::pos2(c.x + 8.0, c.y - 1.0),
                ],
                s,
            ));
            p.add(egui::Shape::line(
                vec![
                    egui::pos2(c.x - 6.0, c.y - 1.0),
                    egui::pos2(c.x - 6.0, c.y + 8.0),
                    egui::pos2(c.x + 6.0, c.y + 8.0),
                    egui::pos2(c.x + 6.0, c.y - 1.0),
                ],
                s,
            ));
        }
        1 => {
            // laptop / devices
            p.rect_stroke(
                egui::Rect::from_min_max(
                    egui::pos2(c.x - 9.0, c.y - 7.0),
                    egui::pos2(c.x + 9.0, c.y + 4.0),
                ),
                2.0,
                s,
            );
            p.add(egui::Shape::line(
                vec![
                    egui::pos2(c.x - 11.0, c.y + 8.0),
                    egui::pos2(c.x + 11.0, c.y + 8.0),
                ],
                s,
            ));
        }
        2 => {
            // Aura bulb
            p.circle_stroke(egui::pos2(c.x, c.y - 2.0), 5.5, s);
            p.rect_stroke(
                egui::Rect::from_center_size(egui::pos2(c.x, c.y + 5.5), egui::vec2(6.0, 4.0)),
                1.0,
                s,
            );
            for dx in [-2.0f32, 2.0] {
                p.add(egui::Shape::line(
                    vec![
                        egui::pos2(c.x + dx, c.y + 9.0),
                        egui::pos2(c.x + dx, c.y + 11.0),
                    ],
                    egui::Stroke::new(1.0, col),
                ));
            }
        }
        3 => {
            // speedometer
            let pts: Vec<egui::Pos2> = (0..=16)
                .map(|i| {
                    let a = std::f32::consts::PI * (0.15 + 0.7 * i as f32 / 16.0);
                    egui::pos2(c.x + 8.0 * a.cos(), c.y + 4.0 - 8.0 * a.sin())
                })
                .collect();
            p.add(egui::Shape::line(pts, s));
            p.add(egui::Shape::line(
                vec![egui::pos2(c.x, c.y + 4.0), egui::pos2(c.x + 4.0, c.y - 4.0)],
                s,
            ));
        }
        4 => {
            // gamepad
            p.rect_stroke(
                egui::Rect::from_min_max(
                    egui::pos2(c.x - 9.0, c.y - 5.0),
                    egui::pos2(c.x + 9.0, c.y + 6.0),
                ),
                6.0,
                s,
            );
            p.add(egui::Shape::line(
                vec![egui::pos2(c.x - 4.0, c.y), egui::pos2(c.x - 4.0, c.y + 4.0)],
                s,
            ));
            p.add(egui::Shape::line(
                vec![
                    egui::pos2(c.x - 6.0, c.y + 2.0),
                    egui::pos2(c.x - 2.0, c.y + 2.0),
                ],
                s,
            ));
            p.circle_stroke(egui::pos2(c.x + 4.0, c.y + 2.0), 1.2, s);
            p.circle_stroke(egui::pos2(c.x + 6.5, c.y - 0.5), 1.2, s);
        }
        5 => {
            // fan
            p.circle_stroke(c, 2.5, s);
            for a in [0.0f32, 2.094, 4.189] {
                p.add(egui::Shape::line(
                    vec![
                        c,
                        egui::pos2(c.x + 7.0 * a.cos(), c.y + 7.0 * a.sin()),
                    ],
                    s,
                ));
            }
            p.circle_stroke(c, 9.0, egui::Stroke::new(1.0, col));
        }
        6 => {
            // battery
            p.rect_stroke(
                egui::Rect::from_min_max(
                    egui::pos2(c.x - 8.0, c.y - 5.0),
                    egui::pos2(c.x + 6.0, c.y + 5.0),
                ),
                2.0,
                s,
            );
            p.rect_filled(
                egui::Rect::from_min_max(
                    egui::pos2(c.x + 6.0, c.y - 2.0),
                    egui::pos2(c.x + 9.0, c.y + 2.0),
                ),
                1.0,
                col,
            );
            p.rect_filled(
                egui::Rect::from_min_max(
                    egui::pos2(c.x - 6.0, c.y - 3.0),
                    egui::pos2(c.x + 2.0, c.y + 3.0),
                ),
                1.0,
                col,
            );
        }
        7 => {
            // display
            p.rect_stroke(
                egui::Rect::from_min_max(
                    egui::pos2(c.x - 9.0, c.y - 7.0),
                    egui::pos2(c.x + 9.0, c.y + 5.0),
                ),
                2.0,
                s,
            );
            p.add(egui::Shape::line(
                vec![
                    egui::pos2(c.x - 5.0, c.y + 9.0),
                    egui::pos2(c.x + 5.0, c.y + 9.0),
                ],
                s,
            ));
            p.add(egui::Shape::line(
                vec![egui::pos2(c.x, c.y + 5.0), egui::pos2(c.x, c.y + 9.0)],
                s,
            ));
        }
        9 => {
            // grid (content)
            for (dx, dy) in [(-6.0f32, -6.0f32), (1.0, -6.0), (-6.0, 1.0), (1.0, 1.0)] {
                p.rect_stroke(
                    egui::Rect::from_min_size(
                        egui::pos2(c.x + dx, c.y + dy),
                        egui::vec2(5.0, 5.0),
                    ),
                    1.0,
                    s,
                );
            }
        }
        10 => {
            // person (user centre)
            p.circle_stroke(egui::pos2(c.x, c.y - 4.0), 3.5, s);
            let pts: Vec<egui::Pos2> = (0..=12)
                .map(|i| {
                    let a = std::f32::consts::PI * (1.05 + 0.9 * i as f32 / 12.0);
                    egui::pos2(c.x + 8.0 * a.cos(), c.y + 6.0 - 7.0 * a.sin())
                })
                .collect();
            p.add(egui::Shape::line(pts, s));
        }
        11 => {
            // book (feature library)
            p.rect_stroke(
                egui::Rect::from_min_max(
                    egui::pos2(c.x - 8.0, c.y - 8.0),
                    egui::pos2(c.x + 8.0, c.y + 8.0),
                ),
                2.0,
                s,
            );
            p.add(egui::Shape::line(
                vec![egui::pos2(c.x, c.y - 8.0), egui::pos2(c.x, c.y + 8.0)],
                thin_stroke(col),
            ));
        }
        _ => {
            // gear
            p.circle_stroke(c, 3.5, s);
            for i in 0..8 {
                let a = i as f32 * std::f32::consts::TAU / 8.0;
                p.add(egui::Shape::line(
                    vec![
                        egui::pos2(c.x + 5.0 * a.cos(), c.y + 5.0 * a.sin()),
                        egui::pos2(c.x + 8.0 * a.cos(), c.y + 8.0 * a.sin()),
                    ],
                    s,
                ));
            }
        }
    }
}

/// Fill a rect with a vertical gradient (top → bottom).
fn gradient_rect(
    p: &egui::Painter,
    rect: egui::Rect,
    top: egui::Color32,
    bottom: egui::Color32,
) {
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(rect.left_top(), top);
    mesh.colored_vertex(rect.right_top(), top);
    mesh.colored_vertex(rect.right_bottom(), bottom);
    mesh.colored_vertex(rect.left_bottom(), bottom);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    p.add(egui::Shape::mesh(mesh));
}

/// Original laptop product render for the Home card (no ASUS assets):
/// metallic deck, gradient screen with TUF-style lettering and a live RGB keyboard.
fn draw_product(
    p: &egui::Painter,
    rect: egui::Rect,
    t: f32,
    rate: f32,
    rgb: [u8; 3],
    mode: u32,
) {
    let (bh, bs, bv) = TufApp::rgb_to_hsv(rgb);
    // soft glow behind the machine
    for i in (0..6).rev() {
        let f = i as f32 / 6.0;
        let glow = egui::Rect::from_center_size(
            egui::pos2(rect.center().x, rect.center().y + rect.height() * 0.10),
            egui::vec2(
                rect.width() * (0.55 + f * 0.35),
                rect.height() * (0.35 + f * 0.35),
            ),
        );
        let c = TufApp::hsv_to_rgb(bh, bs, bv.min(0.45));
        p.rect_filled(
            glow,
            24.0,
            egui::Color32::from_rgba_unmultiplied(c[0], c[1], c[2], (9.0 * (1.0 - f)) as u8),
        );
    }
    // screen bezel + panel
    let scr = egui::Rect::from_center_size(
        egui::pos2(rect.center().x, rect.top() + rect.height() * 0.30),
        egui::vec2(rect.width() * 0.80, rect.height() * 0.42),
    );
    p.rect_filled(scr.expand(7.0), 7.0, egui::Color32::from_rgb(22, 22, 26));
    gradient_rect(
        p,
        scr,
        egui::Color32::from_rgb(24, 31, 44),
        egui::Color32::from_rgb(9, 11, 16),
    );
    // geometric TUF-style backdrop
    for i in 0..5 {
        let x = scr.left() + scr.width() * (0.16 + i as f32 * 0.17);
        p.add(egui::Shape::line(
            vec![
                egui::pos2(x, scr.top() + scr.height() * 0.16),
                egui::pos2(x - scr.width() * 0.07, scr.bottom() - scr.height() * 0.14),
            ],
            egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(120, 170, 220, 40)),
        ));
    }
    p.text(
        egui::pos2(scr.center().x, scr.center().y - 6.0),
        egui::Align2::CENTER_CENTER,
        "TUF",
        rog_font(52.0),
        accent(),
    );
    p.text(
        egui::pos2(scr.center().x, scr.center().y + 34.0),
        egui::Align2::CENTER_CENTER,
        "G A M I N G",
        rog_font(12.0),
        egui::Color32::from_rgb(150, 160, 175),
    );
    // metallic deck (trapezoid)
    let dy = scr.bottom() + 10.0;
    let deck = [
        egui::pos2(scr.left() - 12.0, dy),
        egui::pos2(scr.right() + 12.0, dy),
        egui::pos2(rect.right() - rect.width() * 0.05, dy + rect.height() * 0.30),
        egui::pos2(rect.left() + rect.width() * 0.05, dy + rect.height() * 0.30),
    ];
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(deck[0], egui::Color32::from_rgb(96, 101, 110));
    mesh.colored_vertex(deck[1], egui::Color32::from_rgb(84, 89, 98));
    mesh.colored_vertex(deck[2], egui::Color32::from_rgb(40, 43, 49));
    mesh.colored_vertex(deck[3], egui::Color32::from_rgb(52, 56, 63));
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    p.add(egui::Shape::mesh(mesh));
    // keyboard well
    let kb = egui::Rect::from_center_size(
        egui::pos2(rect.center().x, dy + rect.height() * 0.13),
        egui::vec2(rect.width() * 0.60, rect.height() * 0.16),
    );
    p.rect_filled(kb, 4.0, egui::Color32::from_rgb(16, 16, 20));
    // live per-key RGB
    let rows = 5usize;
    let cols = 15usize;
    let gap = 2.0;
    let kw = (kb.width() - (cols as f32 + 1.0) * gap) / cols as f32;
    let kh = (kb.height() - (rows as f32 + 1.0) * gap) / rows as f32;
    for row in 0..rows {
        for col in 0..cols {
            let idx = row * cols + col;
            let (h, sa, v) = TufApp::effect_hsv(
                EffectParams {
                    mode,
                    bh,
                    bs,
                    bv,
                    cols,
                    t,
                    rate,
                },
                idx,
            );
            let [r, g, b] = TufApp::hsv_to_rgb(h, sa, v);
            let kr = egui::Rect::from_min_size(
                egui::pos2(
                    kb.left() + gap + col as f32 * (kw + gap),
                    kb.top() + gap + row as f32 * (kh + gap),
                ),
                egui::vec2(kw, kh),
            );
            p.rect_filled(kr, 1.5, egui::Color32::from_rgb(r, g, b));
        }
    }
    // trackpad
    p.rect_filled(
        egui::Rect::from_center_size(
            egui::pos2(rect.center().x, kb.bottom() + rect.height() * 0.055),
            egui::vec2(rect.width() * 0.16, rect.height() * 0.045),
        ),
        2.0,
        egui::Color32::from_rgb(72, 76, 84),
    );
    // reflection
    p.rect_filled(
        egui::Rect::from_center_size(
            egui::pos2(rect.center().x, deck[3].y + 18.0),
            egui::vec2(rect.width() * 0.56, 16.0),
        ),
        8.0,
        { let a = accent(); egui::Color32::from_rgba_unmultiplied(a.r(), a.g(), a.b(), 14) },
    );
}

/// Large line icon for an Aura effect tile.
fn effect_icon(p: &egui::Painter, c: egui::Pos2, mode: u32, col: egui::Color32) {
    let s = egui::Stroke::new(1.8, col);
    let thin = egui::Stroke::new(1.2, col);
    match mode {
        0 => {
            // Static — concentric rings
            p.circle_stroke(c, 13.0, s);
            p.circle_stroke(c, 7.0, thin);
            p.circle_filled(c, 2.5, col);
        }
        1 => {
            // Breathing — sine wave
            let pts: Vec<egui::Pos2> = (0..=32)
                .map(|i| {
                    let x = -15.0 + 30.0 * i as f32 / 32.0;
                    let y = 7.0 * (x / 15.0 * std::f32::consts::PI).sin();
                    egui::pos2(c.x + x, c.y + y)
                })
                .collect();
            p.add(egui::Shape::line(pts, s));
        }
        2 => {
            // Color Cycle — swirl
            let pts: Vec<egui::Pos2> = (0..=40)
                .map(|i| {
                    let a = i as f32 / 40.0 * std::f32::consts::TAU * 1.4;
                    let r = 2.0 + 11.0 * i as f32 / 40.0;
                    egui::pos2(c.x + r * a.cos(), c.y + r * a.sin())
                })
                .collect();
            p.add(egui::Shape::line(pts, s));
        }
        3 => {
            // Rainbow — arcs
            for (i, r) in [6.0f32, 11.0, 16.0].iter().enumerate() {
                let pts: Vec<egui::Pos2> = (0..=24)
                    .map(|k| {
                        let a = std::f32::consts::PI * (0.1 + 0.8 * k as f32 / 24.0);
                        egui::pos2(c.x + r * a.cos(), c.y + 6.0 - r * a.sin())
                    })
                    .collect();
                p.add(egui::Shape::line(
                    pts,
                    egui::Stroke::new(if i == 1 { 1.8 } else { 1.2 }, col),
                ));
            }
        }
        4 => {
            // Star — sparkles
            for (dx, dy, r) in [(0.0f32, 0.0f32, 8.0f32), (-9.0, 6.0, 4.0), (9.0, -5.0, 3.5)] {
                let cc = egui::pos2(c.x + dx, c.y + dy);
                p.add(egui::Shape::line(
                    vec![
                        egui::pos2(cc.x - r, cc.y),
                        egui::pos2(cc.x + r, cc.y),
                    ],
                    thin,
                ));
                p.add(egui::Shape::line(
                    vec![
                        egui::pos2(cc.x, cc.y - r),
                        egui::pos2(cc.x, cc.y + r),
                    ],
                    thin,
                ));
            }
        }
        5 => {
            // Rain — falling dashes
            for (dx, off) in [(-8.0f32, 0.0f32), (0.0, 4.0), (8.0, -3.0)] {
                for i in 0..3 {
                    let y = c.y - 10.0 + off + i as f32 * 8.0;
                    p.add(egui::Shape::line(
                        vec![
                            egui::pos2(c.x + dx, y),
                            egui::pos2(c.x + dx, y + 4.0),
                        ],
                        thin,
                    ));
                }
            }
        }
        6 => {
            // Highlight — band with glow
            p.rect_stroke(
                egui::Rect::from_center_size(c, egui::vec2(28.0, 14.0)),
                4.0,
                thin,
            );
            p.rect_filled(
                egui::Rect::from_center_size(egui::pos2(c.x, c.y), egui::vec2(9.0, 14.0)),
                2.0,
                col,
            );
        }
        7 => {
            // Laser — diagonal beam
            p.add(egui::Shape::line(
                vec![
                    egui::pos2(c.x - 14.0, c.y + 10.0),
                    egui::pos2(c.x + 14.0, c.y - 10.0),
                ],
                s,
            ));
            p.circle_filled(egui::pos2(c.x + 12.0, c.y - 8.0), 2.0, col);
        }
        8 => {
            // Ripple — concentric rings
            for r in [5.0f32, 9.0, 13.0] {
                p.circle_stroke(c, r, thin);
            }
        }
        10 => {
            // Strobe — diamonds
            for (dx, dy) in [(0.0f32, 0.0f32), (-9.0, 6.0), (9.0, -5.0)] {
                let cc = egui::pos2(c.x + dx, c.y + dy);
                p.add(egui::Shape::line(
                    vec![
                        egui::pos2(cc.x, cc.y - 5.0),
                        egui::pos2(cc.x + 5.0, cc.y),
                        egui::pos2(cc.x, cc.y + 5.0),
                        egui::pos2(cc.x - 5.0, cc.y),
                        egui::pos2(cc.x, cc.y - 5.0),
                    ],
                    thin,
                ));
            }
        }
        _ => {
            // Comet — head + tail
            p.circle_filled(egui::pos2(c.x + 7.0, c.y - 4.0), 3.5, col);
            let pts: Vec<egui::Pos2> = (0..=14)
                .map(|i| {
                    let f = i as f32 / 14.0;
                    egui::pos2(c.x - 12.0 + 16.0 * f, c.y + 8.0 - 10.0 * f)
                })
                .collect();
            p.add(egui::Shape::line(pts, thin));
        }
    }
}

/// Operating-mode icon variants used on the Home page.
fn mode_icon(p: &egui::Painter, c: egui::Pos2, kind: u8, col: egui::Color32) {
    let s = egui::Stroke::new(1.6, col);
    match kind {
        0 => {
            // Windows — four squares
            for (dx, dy) in [(-5.0f32, -5.0f32), (1.0, -5.0), (-5.0, 1.0), (1.0, 1.0)] {
                p.rect_stroke(
                    egui::Rect::from_min_size(
                        egui::pos2(c.x + dx, c.y + dy),
                        egui::vec2(4.0, 4.0),
                    ),
                    0.8,
                    s,
                );
            }
        }
        1 => {
            // Silent — leaf
            let pts: Vec<egui::Pos2> = (0..=16)
                .map(|i| {
                    let a = std::f32::consts::PI * (0.15 + 0.7 * i as f32 / 16.0);
                    egui::pos2(c.x + 8.0 * a.cos(), c.y + 8.0 * a.sin())
                })
                .collect();
            p.add(egui::Shape::line(pts, s));
            p.add(egui::Shape::line(
                vec![egui::pos2(c.x - 6.0, c.y + 6.0), egui::pos2(c.x + 6.0, c.y - 6.0)],
                s,
            ));
        }
        2 => nav_icon(p, c, 3, col),
        _ => nav_icon(p, c, 5, col),
    }
}

/// One Armoury-style sidebar row: icon + label, orange accent when selected.
fn nav_item_c(
    ui: &mut egui::Ui,
    icon: u8,
    label: &str,
    selected: bool,
    accent: egui::Color32,
) -> bool {
    let h = 34.0;
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), h), egui::Sense::click());
    let p = ui.painter_at(rect);
    let orange = accent;
    if selected {
        p.rect_filled(rect, 4.0, egui::Color32::from_rgb(36, 28, 18));
        p.rect_filled(
            egui::Rect::from_min_size(
                egui::pos2(rect.left(), rect.top() + 5.0),
                egui::vec2(3.0, h - 10.0),
            ),
            1.5,
            orange,
        );
    } else if resp.hovered() {
        p.rect_filled(rect, 4.0, egui::Color32::from_rgb(24, 25, 30));
    }
    let col = if selected {
        orange
    } else {
        egui::Color32::from_rgb(195, 195, 205)
    };
    nav_icon(&p, egui::pos2(rect.left() + 22.0, rect.center().y), icon, col);
    p.text(
        egui::pos2(rect.left() + 40.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(14.0),
        col,
    );
    resp.clicked()
}

/// Armoury-style ring gauge (CPU/GPU temps, fan RPM, battery).
#[allow(dead_code)]
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

const TUF_ACCENT: egui::Color32 = egui::Color32::from_rgb(235, 140, 50);
static THEME_ACCENT: std::sync::atomic::AtomicU32 =
    std::sync::atomic::AtomicU32::new(0x00EB_8C32);

/// Current theme accent, shared by all custom-painted widgets.
fn accent() -> egui::Color32 {
    let v = THEME_ACCENT.load(std::sync::atomic::Ordering::Relaxed);
    egui::Color32::from_rgb((v >> 16) as u8, (v >> 8) as u8, v as u8)
}

fn set_accent(c: egui::Color32) {
    let v = ((c.r() as u32) << 16) | ((c.g() as u32) << 8) | (c.b() as u32);
    THEME_ACCENT.store(v, std::sync::atomic::Ordering::Relaxed);
}

fn rog_family() -> egui::FontFamily {
    egui::FontFamily::Name("rog".into())
}

fn rog_font(size: f32) -> egui::FontId {
    egui::FontId::new(size, rog_family())
}

/// Small rounded status chip used in the Home hero.
fn status_chip(ui: &mut egui::Ui, text: &str, color: egui::Color32) {
    let galley = ui.painter().layout_no_wrap(
        text.to_string(),
        egui::FontId::proportional(12.0),
        color,
    );
    let size = egui::vec2(galley.size().x + 16.0, 22.0);
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    let p = ui.painter_at(rect);
    p.rect_filled(
        rect,
        11.0,
        egui::Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 26),
    );
    p.rect_stroke(rect, 11.0, egui::Stroke::new(1.0, color));
    p.galley(
        rect.min + egui::vec2(8.0, (rect.height() - galley.size().y) / 2.0),
        galley,
        color,
    );
}

/// Armoury-style thin orange meter bar with label + right-aligned value.
fn meter_bar(ui: &mut egui::Ui, label: &str, value: &str, frac: f32) {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.colored_label(egui::Color32::GRAY, value);
        });
    });
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 4.0), egui::Sense::hover());
    let p = ui.painter_at(rect);
    p.rect_filled(rect, 2.0, egui::Color32::from_rgb(44, 46, 54));
    if frac > 0.0 {
        p.rect_filled(
            egui::Rect::from_min_size(
                rect.min,
                egui::vec2(rect.width() * frac.clamp(0.0, 1.0), 4.0),
            ),
            2.0,
            accent(),
        );
    }
}

fn meta_row(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.colored_label(egui::Color32::GRAY, value);
        });
    });
}

fn usage_cpu(
    ui: &mut egui::Ui,
    t: &Telemetry,
    cpu_util: Option<u32>,
    mhz: Option<u64>,
) {
    let white = egui::Color32::from_rgb(235, 235, 240);
    ui.colored_label(white, egui::RichText::new("CPU").size(27.0).strong().family(rog_family()));
    ui.add_space(4.0);
    meter_bar(
        ui,
        "Frequency",
        &mhz.map(|m| format!("{m} MHz")).unwrap_or_else(|| "—".into()),
        mhz.map(|m| m as f32 / 4500.0).unwrap_or(0.0),
    );
    let u = cpu_util.unwrap_or(0);
    meter_bar(ui, "Usage", &format!("{u} %"), u as f32 / 100.0);
    meta_row(ui, "Memory Frequency", "—");
    if let Some(m) = t.cpu_package_temp_mC {
        meter_bar(
            ui,
            "Temperature",
            &format!("{:.0} °C", m as f32 / 1000.0),
            m as f32 / 95_000.0,
        );
    }
    meta_row(ui, "Voltage", "—");
    meta_row(ui, "————————", "");
}

fn usage_gpu(ui: &mut egui::Ui, t: &Telemetry) {
    let white = egui::Color32::from_rgb(235, 235, 240);
    ui.colored_label(white, egui::RichText::new("GPU").size(27.0).strong().family(rog_family()));
    ui.add_space(4.0);
    let (util, temp, tgp) = t
        .nvidia
        .as_ref()
        .map(|g| (g.utilization_percent, g.temperature_c, g.power_watts))
        .unwrap_or((0, 0, 0.0));
    meta_row(
        ui,
        "Frequency",
        if util == 0 { "Power Saving" } else { "Active" },
    );
    meter_bar(ui, "Usage", &format!("{util} %"), util as f32 / 100.0);
    meta_row(ui, "Memory Frequency", "—");
    meter_bar(ui, "Temperature", &format!("{temp} °C"), temp as f32 / 95.0);
    meta_row(ui, "Voltage", "—");
    meta_row(ui, "TGP", &format!("{tgp:.0} W"));
}

fn usage_fan(ui: &mut egui::Ui, t: &Telemetry) {
    let white = egui::Color32::from_rgb(235, 235, 240);
    ui.colored_label(white, egui::RichText::new("Fan").size(27.0).strong().family(rog_family()));
    ui.add_space(4.0);
    if let Some(r) = t.cpu_fan_rpm {
        meter_bar(ui, "CPU Fan", &format!("{r} RPM"), r as f32 / 6000.0);
    }
    if let Some(r) = t.gpu_fan_rpm {
        meter_bar(ui, "GPU Fan", &format!("{r} RPM"), r as f32 / 6000.0);
    }
    if let Some(r) = t.cpu_fan_rpm {
        meta_row(
            ui,
            "Fan Acoustics",
            &format!("~{:.1} dBA", 44.8 * r as f32 / 5400.0),
        );
    }
    meta_row(ui, "NVMe", "—");
}

fn usage_mem(ui: &mut egui::Ui, t: &Telemetry) {
    let white = egui::Color32::from_rgb(235, 235, 240);
    ui.colored_label(white, egui::RichText::new("Memory").size(27.0).strong().family(rog_family()));
    ui.add_space(4.0);
    if let (Some(u), Some(tot)) = (t.storage_used_gb, t.storage_total_gb) {
        meter_bar(
            ui,
            "Storage",
            &format!("{u:.1} / {tot:.1} GB"),
            u / tot.max(0.1),
        );
    }
    if let (Some(u), Some(tot)) = (t.ram_used_gb, t.ram_total_gb) {
        meter_bar(
            ui,
            "RAM",
            &format!("{u:.1} / {tot:.1} GB"),
            u / tot.max(0.1),
        );
    }
    if let Some(b) = &t.battery {
        meta_row(ui, "Battery", &format!("{} %", b.capacity_percent));
    }
}

#[allow(dead_code)]
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

#[derive(Clone)]
struct ProcRow {
    pid: i32,
    comm: String,
    rss_mb: u64,
    uid: u32,
}

fn current_uid() -> u32 {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("Uid:"))
                .and_then(|l| l.split_whitespace().nth(1).and_then(|x| x.parse().ok()))
        })
        .unwrap_or(u32::MAX)
}

fn safe_to_stop(comm: &str) -> bool {
    const DENY: &[&str] = &[
        "systemd",
        "init",
        "Xorg",
        "X",
        "xfwm4",
        "xfdesktop",
        "xfce4-panel",
        "xfce4-session",
        "xfsettingsd",
        "xfconfd",
        "pipewire",
        "wireplumber",
        "pulseaudio",
        "dbus-daemon",
        "dbus-broker",
        "sshd",
        "NetworkManager",
        "wpa_supplicant",
        "thermald",
        "tlp",
        "dockerd",
        "containerd",
        "tuf-crate",
        "tuf-crated",
        "bash",
        "zsh",
        "sh",
    ];
    !DENY.contains(&comm) && !comm.starts_with("xfce4-")
}

fn read_proc_rows(limit: usize) -> Vec<ProcRow> {
    let uid = current_uid();
    let mut rows = Vec::new();
    if let Ok(rd) = std::fs::read_dir("/proc") {
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if !name.chars().all(|c| c.is_ascii_digit()) {
                continue;
            }
            let Ok(pid) = name.parse::<i32>() else {
                continue;
            };
            let comm = std::fs::read_to_string(e.path().join("comm"))
                .unwrap_or_default()
                .trim()
                .to_string();
            let rss_mb = std::fs::read_to_string(e.path().join("statm"))
                .ok()
                .and_then(|s| {
                    s.split_whitespace()
                        .nth(1)
                        .and_then(|x| x.parse::<u64>().ok())
                })
                .map(|pages| pages * 4096 / 1024 / 1024)
                .unwrap_or(0);
            let puid = std::fs::read_to_string(e.path().join("status"))
                .ok()
                .and_then(|s| {
                    s.lines()
                        .find(|l| l.starts_with("Uid:"))
                        .and_then(|l| {
                            l.split_whitespace()
                                .nth(1)
                                .and_then(|x| x.parse::<u32>().ok())
                        })
                })
                .unwrap_or(u32::MAX);
            if rss_mb > 0 {
                rows.push(ProcRow {
                    pid,
                    comm,
                    rss_mb,
                    uid: puid,
                });
            }
        }
    }
    rows.retain(|r| r.uid == uid);
    rows.sort_by_key(|r| std::cmp::Reverse(r.rss_mb));
    rows.truncate(limit);
    rows
}

fn tab_strip<T: PartialEq + Copy>(
    ui: &mut egui::Ui,
    current: &mut T,
    items: &[(T, &str)],
    accent: egui::Color32,
) {
    ui.horizontal_wrapped(|ui| {
        for (value, label) in items {
            let selected = *current == *value;
            let resp = ui.add(
                egui::Button::new(
                    egui::RichText::new(*label)
                        .size(13.0)
                        .color(if selected {
                            accent
                        } else {
                            egui::Color32::GRAY
                        }),
                )
                .fill(if selected {
                    egui::Color32::from_rgba_unmultiplied(
                        accent.r(),
                        accent.g(),
                        accent.b(),
                        28,
                    )
                } else {
                    egui::Color32::TRANSPARENT
                })
                .stroke(if selected {
                    egui::Stroke::new(1.0, accent)
                } else {
                    egui::Stroke::NONE
                }),
            );
            if resp.clicked() {
                *current = *value;
            }
        }
    });
    ui.add_space(4.0);
    ui.separator();
    ui.add_space(8.0);
}

fn feature_tile(
    ui: &mut egui::Ui,
    icon: u8,
    title: &str,
    desc: &str,
    supported: bool,
    accent: egui::Color32,
) {
    egui::Frame::none()
        .fill(egui::Color32::from_rgb(24, 24, 27))
        .rounding(8.0)
        .inner_margin(12.0)
        .show(ui, |ui| {
            ui.set_width(200.0);
            ui.vertical(|ui| {
                ui.set_min_height(150.0);
                let (r, _) =
                    ui.allocate_exact_size(egui::vec2(200.0, 42.0), egui::Sense::hover());
                nav_icon(
                    &ui.painter_at(r),
                    r.center(),
                    icon,
                    if supported {
                        accent
                    } else {
                        egui::Color32::from_rgb(130, 130, 140)
                    },
                );
                ui.label(egui::RichText::new(title).size(14.0).strong());
                ui.colored_label(egui::Color32::GRAY, desc);
                if !supported {
                    ui.colored_label(
                        egui::Color32::from_rgb(160, 140, 110),
                        "Windows-only hardware feature",
                    );
                }
            });
        });
}

fn set_xfce_wallpaper(path: &str) -> Result<usize, String> {
    let out = std::process::Command::new("xfconf-query")
        .args(["-c", "xfce4-desktop", "-l"])
        .output()
        .map_err(|e| format!("xfconf-query not available: {e}"))?;
    if !out.status.success() {
        return Err("cannot list XFCE desktop properties".into());
    }
    let props: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| l.ends_with("/last-image"))
        .map(|l| l.to_string())
        .collect();
    if props.is_empty() {
        return Err("no XFCE last-image property found".into());
    }
    let mut count = 0;
    for prop in props {
        let ok = std::process::Command::new("xfconf-query")
            .args(["-c", "xfce4-desktop", "-p", &prop, "-s", path])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if ok {
            count += 1;
        }
    }
    if count == 0 {
        Err("xfconf-query could not set the wallpaper".into())
    } else {
        Ok(count)
    }
}


fn ui_settings_path() -> std::path::PathBuf {
    let base = std::env::var("XDG_CONFIG_HOME")
        .unwrap_or_else(|_| format!("{}/.config", std::env::var("HOME").unwrap_or_default()));
    std::path::PathBuf::from(base).join("tuf-crate/ui.json")
}

fn load_ui_settings() -> serde_json::Value {
    std::fs::read_to_string(ui_settings_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or(serde_json::Value::Null)
}

fn theme_from_str(s: &str) -> Theme {
    match s {
        "Rog" => Theme::Rog,
        "Prime" => Theme::Prime,
        "ProArt" => Theme::ProArt,
        "Light" => Theme::Light,
        "FollowSystem" => Theme::FollowSystem,
        _ => Theme::Tuf,
    }
}

fn apply_theme_visuals(ctx: &egui::Context, theme: Theme) {
    let accent = theme.accent();
    set_accent(accent);
    let mut v = if theme == Theme::Light {
        egui::Visuals::light()
    } else {
        egui::Visuals::dark()
    };
    if theme != Theme::Light {
        v.panel_fill = egui::Color32::from_rgb(17, 18, 22);
        v.window_fill = egui::Color32::from_rgb(22, 24, 28);
        v.extreme_bg_color = egui::Color32::from_rgb(11, 12, 15);
        v.widgets.inactive.bg_fill = egui::Color32::from_rgb(28, 30, 36);
        v.widgets.hovered.bg_fill = egui::Color32::from_rgb(40, 42, 48);
    }
    v.selection.bg_fill =
        egui::Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), 70);
    v.selection.stroke = egui::Stroke::new(1.0, accent);
    v.widgets.active.bg_fill =
        egui::Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), 90);
    ctx.set_visuals(v);
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
    splash_start: std::time::Instant,
    splash_logo: Option<egui::TextureHandle>,
    anim_cpu: f32,
    anim_gpu: f32,
    anim_fan: f32,
    anim_batt: f32,
    last_anim: std::time::Instant,
    devices_sub: DeviceTab,
    aura_sub: AuraTab,
    settings_sub: SettingsTab,
    theme: Theme,
    theme_previews: Vec<(Theme, Option<egui::TextureHandle>)>,
    launch_page: String,
    free_confirm: bool,
    sync_select_all: bool,
    aura_perf_mode: bool,
    free_selected: std::collections::BTreeSet<i32>,
    wallpapers: Vec<(String, Option<egui::TextureHandle>)>,
    wallpaper_idx: usize,
    device_line_art: bool,
    update_text: String,
    game_rules: Vec<games::GameRule>,
    game_input: String,
    game_profile: String,
    active_game_rule: Option<String>,
    restore_profile: Option<String>,
    last_game_check: std::time::Instant,
    cpu_util: Option<u32>,
    prev_stat: Option<(u64, u64)>,
    tab: Tab,
}

impl TufApp {
    fn new(
        backend: Box<dyn Backend>,
        writes_enabled: bool,
        ctx: &egui::Context,
        splash_logo: Option<egui::TextureHandle>,
    ) -> Self {
        let saved = load_ui_settings();
        let theme = saved
            .get("theme")
            .and_then(|v| v.as_str())
            .map(theme_from_str)
            .unwrap_or_default();
        let launch_page = saved
            .get("launch_page")
            .and_then(|v| v.as_str())
            .unwrap_or("Home")
            .to_string();
        let device_line_art = saved
            .get("device_line_art")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let initial_tab = match launch_page.as_str() {
            "Devices" => Tab::Devices,
            "Lighting & Aura Sync" => Tab::AuraSync,
            "Scenario Profiles" => Tab::ScenarioProfiles,
            "Settings" => Tab::Settings,
            _ => Tab::Home,
        };
        apply_theme_visuals(ctx, theme);
        let theme_previews = [
            Theme::Tuf,
            Theme::Rog,
            Theme::Prime,
            Theme::ProArt,
            Theme::Light,
            Theme::FollowSystem,
        ]
        .into_iter()
        .map(|t| (t, assets::load_texture(ctx, t.preview())))
        .collect::<Vec<_>>();
        let wallpapers: Vec<(String, Option<egui::TextureHandle>)> =
            assets::wallpaper_paths()
                .into_iter()
                .map(|p| {
                    let name = p
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_default();
                    let key = format!("wallpaper-{name}");
                    let tex = assets::load_texture_abs(ctx, &key, &p);
                    (p.to_string_lossy().into_owned(), tex)
                })
                .collect();
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
            splash_start: std::time::Instant::now(),
            splash_logo,
            theme_previews,
            anim_cpu: 0.0,
            anim_gpu: 0.0,
            anim_fan: 0.0,
            anim_batt: 0.0,
            last_anim: std::time::Instant::now(),
            devices_sub: DeviceTab::SystemConfig,
            aura_sub: AuraTab::Effects,
            settings_sub: SettingsTab::General,
            theme,
            launch_page,
            free_confirm: false,
            sync_select_all: true,
            aura_perf_mode: false,
            free_selected: std::collections::BTreeSet::new(),
            wallpapers,
            wallpaper_idx: 0,
            device_line_art,
            update_text: String::new(),
            game_rules: games::load_all(),
            game_input: String::new(),
            game_profile: "performance".to_string(),
            active_game_rule: None,
            restore_profile: None,
            last_game_check: std::time::Instant::now(),
            cpu_util: None,
            prev_stat: None,
            tab: initial_tab,
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
        const SPLASH_SECS: f32 = 2.2;
        let splash_t = self.splash_start.elapsed().as_secs_f32();
        if std::env::var("TUF_CRATE_SKIP_SPLASH").is_err() && splash_t < SPLASH_SECS {
            self.draw_splash(ctx, splash_t, SPLASH_SECS);
            ctx.request_repaint_after(std::time::Duration::from_millis(16));
            return;
        }
        if self.last_poll.elapsed() >= std::time::Duration::from_secs(1) {
            self.poll();
            self.last_poll = std::time::Instant::now();
        }
        // request repaint so the polling continues; the Lighting tab animates
        // and software effects need a steady frame clock on every tab.
        if matches!(self.tab, Tab::AuraSync | Tab::Home) || self.soft_effect.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
        } else {
            ctx.request_repaint_after(std::time::Duration::from_secs(1));
        }
        self.tick_soft_effect();
        self.check_game_rules();
        self.tick_animations();

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
            .exact_width(230.0)
            .frame(egui::Frame::none().fill(egui::Color32::from_rgb(11, 12, 15)))
            .show(ctx, |ui| {
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    let (r, _) =
                        ui.allocate_exact_size(egui::vec2(30.0, 26.0), egui::Sense::hover());
                    let p = ui.painter_at(r);
                    for i in 0..3 {
                        p.add(egui::Shape::line(
                            vec![
                                egui::pos2(r.left() + 6.0, r.top() + 8.0 + i as f32 * 5.0),
                                egui::pos2(r.right() - 6.0, r.top() + 8.0 + i as f32 * 5.0),
                            ],
                            egui::Stroke::new(1.5, egui::Color32::from_rgb(190, 190, 200)),
                        ));
                    }
                    ui.colored_label(
                        accent(),
                        egui::RichText::new("tuf-crate").size(18.0).strong().family(rog_family()),
                    );
                });
                ui.colored_label(
                    egui::Color32::from_rgb(120, 120, 130),
                    "   TUF GAMING F17",
                );
                ui.add_space(10.0);

                let group = |ui: &mut egui::Ui, t: &str| {
                    ui.add_space(8.0);
                    ui.colored_label(
                        egui::Color32::from_rgb(110, 110, 120),
                        egui::RichText::new(t).size(11.0).strong().family(rog_family()),
                    );
                    ui.add_space(2.0);
                };

                let accent = self.theme.accent();
                if nav_item_c(ui, 0, "Home", self.tab == Tab::Home, accent) {
                    self.tab = Tab::Home;
                }
                if nav_item_c(ui, 1, "Devices", self.tab == Tab::Devices, accent) {
                    self.tab = Tab::Devices;
                }
                group(ui, "PLAYGROUND");
                if nav_item_c(ui, 2, "Lighting & Aura Sync", self.tab == Tab::AuraSync, accent) {
                    self.tab = Tab::AuraSync;
                }
                if nav_item_c(ui, 7, "Aura Wallpaper", self.tab == Tab::AuraWallpaper, accent) {
                    self.tab = Tab::AuraWallpaper;
                }
                group(ui, "ASSISTANT");
                if nav_item_c(ui, 3, "Scenario Profiles", self.tab == Tab::ScenarioProfiles, accent) {
                    self.tab = Tab::ScenarioProfiles;
                }
                if nav_item_c(ui, 4, "Macros", self.tab == Tab::Macros, accent) {
                    self.tab = Tab::Macros;
                }
                if nav_item_c(ui, 7, "GameVisual", self.tab == Tab::GameVisual, accent) {
                    self.tab = Tab::GameVisual;
                }
                group(ui, "CONTENTS");
                if nav_item_c(ui, 9, "Content Platform", self.tab == Tab::ContentPlatform, accent) {
                    self.tab = Tab::ContentPlatform;
                }

                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    ui.add_space(6.0);
                    if nav_item_c(ui, 8, "Settings", self.tab == Tab::Settings, self.theme.accent()) {
                        self.tab = Tab::Settings;
                    }
                    if nav_item_c(ui, 10, "User Center", self.tab == Tab::UserCenter, self.theme.accent()) {
                        self.tab = Tab::UserCenter;
                    }
                    if nav_item_c(ui, 11, "Feature Library", self.tab == Tab::FeatureLibrary, self.theme.accent()) {
                        self.tab = Tab::FeatureLibrary;
                    }
                    ui.separator();
                    ui.colored_label(
                        egui::Color32::GRAY,
                        format!("v{}", env!("CARGO_PKG_VERSION")),
                    );
                    let (txt, col) = if self.backend.name() == "daemon" {
                        ("daemon controls", egui::Color32::from_rgb(120, 200, 140))
                    } else if self.writes_enabled {
                        ("writes ENABLED", egui::Color32::from_rgb(230, 170, 60))
                    } else {
                        ("monitoring-only", egui::Color32::from_rgb(120, 200, 140))
                    };
                    ui.colored_label(col, format!("• {txt}"));
                });
            });

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(egui::Color32::from_rgb(17, 18, 22)))
            .show(ctx, |ui| {
                let (title, subtitle) = match self.tab {
                    Tab::Home => ("Home", "System overview and operating mode"),
                    Tab::Devices => ("System Settings", "Device configuration and tools"),
                    Tab::AuraSync => ("Lighting & Aura Sync", "Keyboard lighting and effects"),
                    Tab::AuraWallpaper => ("Aura Wallpaper", "Wallpaper gallery and display modes"),
                    Tab::ScenarioProfiles => ("Scenario Profiles", "Automatic profiles per application"),
                    Tab::Macros => ("Macros", "Input automation"),
                    Tab::GameVisual => ("GameVisual", "Display colour and visual modes"),
                    Tab::ContentPlatform => ("Content Platform", "ASUS content and promotions"),
                    Tab::FeatureLibrary => ("Feature Library", "Installed and available capabilities"),
                    Tab::UserCenter => ("User Center", "Account and membership"),
                    Tab::Settings => ("Settings", "General, updates and about"),
                };
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.colored_label(
                            egui::Color32::from_rgb(240, 240, 245),
                            egui::RichText::new(title).size(24.0).strong().family(rog_family()),
                        );
                        ui.colored_label(egui::Color32::GRAY, subtitle);
                    });
                    ui.with_layout(
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| {
                            if ui
                                .add(
                                    egui::Button::new(
                                        egui::RichText::new("Aura")
                                            .color(accent()),
                                    )
                                    .fill(egui::Color32::TRANSPARENT)
                                    .stroke(egui::Stroke::new(
                                        1.0,
                                        egui::Color32::from_rgb(90, 70, 40),
                                    )),
                                )
                                .clicked()
                            {
                                self.tab = Tab::AuraSync;
                            }
                            if let Some(p) =
                                self.telemetry.as_ref().and_then(|t| t.profile.clone())
                            {
                                status_chip(
                                    ui,
                                    &p,
                                    egui::Color32::from_rgb(120, 200, 140),
                                );
                            }
                            if let Some(b) =
                                self.telemetry.as_ref().and_then(|t| t.battery.as_ref())
                            {
                                ui.label(format!("{} %", b.capacity_percent));
                                let (r, _) = ui.allocate_exact_size(
                                    egui::vec2(24.0, 18.0),
                                    egui::Sense::hover(),
                                );
                                nav_icon(
                                    &ui.painter_at(r),
                                    r.center(),
                                    6,
                                    egui::Color32::from_rgb(120, 200, 140),
                                );
                            }
                        },
                    );
                });
                ui.add_space(6.0);
                ui.separator();
                ui.add_space(8.0);
                egui::ScrollArea::vertical().show(ui, |ui| match self.tab {
                    Tab::Home => self.ui_home(ui),
                    Tab::Devices => self.ui_devices(ui),
                    Tab::AuraSync => self.ui_aura_sync(ui),
                    Tab::AuraWallpaper => self.ui_aura_wallpaper(ui),
                    Tab::ScenarioProfiles => self.ui_scenario_profiles(ui),
                    Tab::Macros => self.ui_macros(ui),
                    Tab::GameVisual => self.ui_gamevisual(ui),
                    Tab::ContentPlatform => self.ui_content_platform(ui),
                    Tab::FeatureLibrary => self.ui_feature_library(ui),
                    Tab::UserCenter => self.ui_user_center(ui),
                    Tab::Settings => self.ui_settings(ui),
                });
            });
    }
}

#[derive(Default, Clone, Copy, PartialEq)]
enum Tab {
    #[default]
    Home,
    Devices,
    AuraSync,
    AuraWallpaper,
    ScenarioProfiles,
    Macros,
    GameVisual,
    ContentPlatform,
    FeatureLibrary,
    UserCenter,
    Settings,
}

#[derive(Default, Clone, Copy, PartialEq)]
enum DeviceTab {
    #[default]
    SystemConfig,
    FreeMemory,
    GpuPerformance,
    Lighting,
    Audio,
    ResourceMonitor,
}

#[derive(Default, Clone, Copy, PartialEq)]
enum AuraTab {
    #[default]
    Effects,
    SyncDevices,
    AiAura,
}

#[derive(Default, Clone, Copy, PartialEq)]
enum SettingsTab {
    #[default]
    General,
    UpdateCenter,
    About,
}

#[derive(Default, Clone, Copy, PartialEq, Eq, Debug)]
enum Theme {
    #[default]
    Tuf,
    Rog,
    Prime,
    ProArt,
    Light,
    FollowSystem,
}

impl Theme {
    fn label(self) -> &'static str {
        match self {
            Theme::Tuf => "TUF Gaming",
            Theme::Rog => "ROG",
            Theme::Prime => "ASUS PRIME",
            Theme::ProArt => "ProArt",
            Theme::Light => "Light",
            Theme::FollowSystem => "Follow System",
        }
    }
    fn accent(self) -> egui::Color32 {
        match self {
            Theme::Tuf => TUF_ACCENT,
            Theme::Rog => egui::Color32::from_rgb(224, 42, 54),
            Theme::Prime => egui::Color32::from_rgb(64, 150, 210),
            Theme::ProArt => egui::Color32::from_rgb(200, 170, 90),
            Theme::Light => egui::Color32::from_rgb(200, 100, 20),
            Theme::FollowSystem => TUF_ACCENT,
        }
    }
    fn preview(self) -> &'static str {
        match self {
            Theme::Tuf => "themes/ac_settings_img_tuftheme.png",
            Theme::Rog => "themes/ac_settings_img_rogtheme.png",
            Theme::Prime => "themes/ac_settings_img_asustheme.png",
            Theme::ProArt => "themes/ac_settings_img_proarttheme.png",
            Theme::Light => "themes/ac_settings_img_lighttheme.png",
            Theme::FollowSystem => "themes/ac_settings_img_follow system.png",
        }
    }
}

impl TufApp {
    fn temp_c(mc: Option<i64>) -> String {
        mc.map(|v| format!("{:.0} °C", v as f32 / 1000.0))
            .unwrap_or_else(|| "—".into())
    }

    /// Armoury-Crate-style Home: device header, System Usage card grid,
    /// Operating Mode row (Windows | Silent | Performance | Turbo).
    /// Ease the Home gauges toward live telemetry (called every frame).
    fn tick_animations(&mut self) {
        let now = std::time::Instant::now();
        let dt = (now - self.last_anim).as_secs_f32().clamp(0.0, 0.2);
        self.last_anim = now;
        let k = (dt * 6.0).min(1.0);
        let t = self.telemetry.as_ref();
        let cpu = t
            .and_then(|t| t.cpu_package_temp_mC)
            .map(|m| (m as f32 / 100_000.0).clamp(0.0, 1.0))
            .unwrap_or(0.0);
        let gpu = t
            .and_then(|t| t.nvidia.as_ref())
            .map(|g| (g.temperature_c as f32 / 100.0).clamp(0.0, 1.0))
            .unwrap_or(0.0);
        let fan = t
            .and_then(|t| t.cpu_fan_rpm)
            .map(|r| (r as f32 / 6000.0).clamp(0.0, 1.0))
            .unwrap_or(0.0);
        let batt = t
            .and_then(|t| t.battery.as_ref())
            .map(|b| (b.capacity_percent as f32 / 100.0).clamp(0.0, 1.0))
            .unwrap_or(0.0);
        self.anim_cpu += (cpu - self.anim_cpu) * k;
        self.anim_gpu += (gpu - self.anim_gpu) * k;
        self.anim_fan += (fan - self.anim_fan) * k;
        self.anim_batt += (batt - self.anim_batt) * k;
    }

    /// Armoury-style startup splash: black field, logo entrance, orange glow,
    /// rotating accent ring and a thin progress line.
    fn draw_splash(&self, ctx: &egui::Context, t: f32, total: f32) {
        let rect = ctx.screen_rect();
        let p = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Foreground,
            egui::Id::new("splash"),
        ));
        p.rect_filled(rect, 0.0, egui::Color32::BLACK);
        let appear = (t / 0.55).clamp(0.0, 1.0);
        let fade = if t > total - 0.45 {
            ((total - t) / 0.45).clamp(0.0, 1.0)
        } else {
            1.0
        };
        let alpha = (appear * fade * 255.0) as u8;
        let center = rect.center() - egui::vec2(0.0, 10.0);
        let scale = 0.88 + 0.12 * appear;
        let logo_w = 240.0 * scale;
        // glow
        for i in (0..6).rev() {
            let f = i as f32 / 6.0;
            let r = logo_w * (0.55 + f * 0.55);
            p.circle_filled(
                center,
                r,
                egui::Color32::from_rgba_unmultiplied(
                    235,
                    140,
                    50,
                    ((10.0 * (1.0 - f)) * fade) as u8,
                ),
            );
        }
        // rotating accent ring segment
        let radius = logo_w * 0.72;
        let rot = t * 1.6;
        let pts: Vec<egui::Pos2> = (0..=28)
            .map(|i| {
                let a = rot + i as f32 / 28.0 * std::f32::consts::TAU * 0.72;
                egui::pos2(center.x + radius * a.cos(), center.y + radius * a.sin())
            })
            .collect();
        p.add(egui::Shape::line(
            pts,
            egui::Stroke::new(2.0, { let a = accent(); egui::Color32::from_rgba_unmultiplied(a.r(), a.g(), a.b(), alpha) }),
        ));
        if let Some(tex) = &self.splash_logo {
            let sz = tex.size_vec2();
            let h = logo_w * sz.y / sz.x.max(1.0);
            let img = egui::Rect::from_center_size(center, egui::vec2(logo_w, h));
            p.image(
                tex.id(),
                img,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                egui::Color32::from_white_alpha(alpha),
            );
        } else {
            p.text(
                center,
                egui::Align2::CENTER_CENTER,
                "ARMOURY CRATE",
                egui::FontId::proportional(30.0),
                { let a = accent(); egui::Color32::from_rgba_unmultiplied(a.r(), a.g(), a.b(), alpha) },
            );
        }
        p.text(
            egui::pos2(center.x, center.y + logo_w * 0.55),
            egui::Align2::CENTER_CENTER,
            "TUF GAMING F17 · tuf-crate",
            egui::FontId::proportional(15.0),
            egui::Color32::from_rgba_unmultiplied(170, 170, 180, alpha),
        );
        // progress line
        let bar = egui::Rect::from_center_size(
            egui::pos2(rect.center().x, rect.bottom() - 42.0),
            egui::vec2(240.0, 3.0),
        );
        p.rect_filled(bar, 1.5, egui::Color32::from_rgb(40, 40, 45));
        let prog = (t / total).clamp(0.0, 1.0);
        p.rect_filled(
            egui::Rect::from_min_size(
                bar.min,
                egui::vec2(bar.width() * prog, bar.height()),
            ),
            1.5,
            accent(),
        );
    }



    fn ui_devices(&mut self, ui: &mut egui::Ui) {
        let accent = self.theme.accent();
        let tabs = [
            (DeviceTab::SystemConfig, "System Configuration"),
            (DeviceTab::FreeMemory, "Free up memory"),
            (DeviceTab::GpuPerformance, "GPU Performance"),
            (DeviceTab::Lighting, "Lighting"),
            (DeviceTab::Audio, "Audio"),
            (DeviceTab::ResourceMonitor, "Resource Monitor"),
        ];
        tab_strip(ui, &mut self.devices_sub, &tabs, accent);
        match self.devices_sub {
            DeviceTab::SystemConfig => self.ui_system_config(ui),
            DeviceTab::FreeMemory => self.ui_free_memory(ui),
            DeviceTab::GpuPerformance => self.ui_gpu_performance(ui),
            DeviceTab::Lighting => self.ui_lighting(ui),
            DeviceTab::Audio => self.ui_audio(ui),
            DeviceTab::ResourceMonitor => self.ui_resource_monitor(ui),
        }
    }

    fn ui_system_config(&mut self, ui: &mut egui::Ui) {
        let accent = self.theme.accent();
        ui.heading("System Configuration");
        ui.colored_label(
            egui::Color32::GRAY,
            "Armoury Crate device features with their Linux status. Unsupported Windows features are shown honestly.",
        );
        ui.add_space(8.0);
        ui.horizontal_top(|ui| {
            feature_tile(ui, 9, "Win Key", "Lock or disable the Windows key.", false, accent);
            feature_tile(ui, 7, "Touch Pad", "Enable or disable the touchpad.", false, accent);
            feature_tile(ui, 8, "Modern Standby Assistant", "Hibernate assistance for standby power.", false, accent);
            feature_tile(ui, 6, "Panel Power Saver", "Lower refresh rate on battery.", false, accent);
            feature_tile(ui, 3, "Power mode synchronization", "Link Windows power mode to Armoury modes.", false, accent);
        });
        ui.add_space(12.0);
        ui.colored_label(
            accent,
            egui::RichText::new(">>> Linux hardware controls").strong(),
        );
        ui.colored_label(
            egui::Color32::GRAY,
            "These sections control real kernel interfaces on this laptop.",
        );
        ui.add_space(6.0);
        egui::CollapsingHeader::new("Performance & power")
            .default_open(true)
            .show(ui, |ui| self.ui_performance(ui));
        egui::CollapsingHeader::new("Fans & thermals").show(ui, |ui| self.ui_fans(ui));
        egui::CollapsingHeader::new("Battery").show(ui, |ui| self.ui_battery(ui));
        egui::CollapsingHeader::new("Display & GPU").show(ui, |ui| self.ui_display_gpu(ui));
        egui::CollapsingHeader::new("Hardware inventory").show(ui, |ui| self.ui_dashboard(ui));
    }

    fn ui_free_memory(&mut self, ui: &mut egui::Ui) {
        let accent = self.theme.accent();
        ui.heading("Free up memory");
        ui.colored_label(
            egui::Color32::GRAY,
            "Select non-critical processes to stop. Only your own processes are listed; system and session-critical processes are excluded and protected.",
        );
        ui.add_space(8.0);
        let rows = read_proc_rows(30);
        ui.horizontal(|ui| {
            if ui.button("Refresh").clicked() {
                self.free_selected.clear();
            }
            ui.checkbox(
                &mut self.free_confirm,
                "I understand the selected processes will be terminated",
            );
            let enabled = self.free_confirm && !self.free_selected.is_empty();
            if ui
                .add_enabled(enabled, egui::Button::new("Free Up selected"))
                .clicked()
            {
                let pids: Vec<i32> = self.free_selected.iter().copied().collect();
                let mut ok = 0;
                for pid in &pids {
                    if *pid == std::process::id() as i32 {
                        continue;
                    }
                    if let Ok(st) = std::process::Command::new("kill")
                        .args(["-TERM", &pid.to_string()])
                        .status()
                    {
                        if st.success() {
                            ok += 1;
                        }
                    }
                }
                self.report(
                    format!("Sent SIGTERM to {ok} process(es)"),
                    ok == 0,
                );
                self.free_selected.clear();
                self.free_confirm = false;
            }
        });
        ui.add_space(6.0);
        egui::Grid::new("free_mem_grid")
            .num_columns(3)
            .spacing([16.0, 4.0])
            .show(ui, |ui| {
                ui.colored_label(egui::Color32::GRAY, "Stop");
                ui.colored_label(egui::Color32::GRAY, "Process");
                ui.colored_label(egui::Color32::GRAY, "RAM");
                ui.end_row();
                for row in rows {
                    let safe = safe_to_stop(&row.comm);
                    let mut checked = self.free_selected.contains(&row.pid);
                    if ui
                        .add_enabled(
                            safe,
                            egui::Checkbox::without_text(&mut checked),
                        )
                        .changed()
                    {
                        if checked {
                            self.free_selected.insert(row.pid);
                        } else {
                            self.free_selected.remove(&row.pid);
                        }
                    }
                    let label = format!("{}  ({})", row.comm, row.pid);
                    if safe {
                        ui.label(label);
                    } else {
                        ui.colored_label(egui::Color32::from_rgb(150, 140, 120), label)
                            .on_hover_text("protected system/session process");
                    }
                    ui.label(format!("{} MB", row.rss_mb));
                    ui.end_row();
                }
            });
        let _ = accent;
    }

    fn ui_gpu_performance(&mut self, ui: &mut egui::Ui) {
        let accent = self.theme.accent();
        ui.heading("GPU Performance");
        ui.colored_label(
            egui::Color32::GRAY,
            "This laptop has no MUX switch. Only enabling/disabling the discrete GPU is supported.",
        );
        ui.add_space(8.0);
        let dgpu_off = self
            .telemetry
            .as_ref()
            .and_then(|t| t.dgpu_disable)
            .unwrap_or(false);
        let modes = [
            (
                "Standard",
                "Hybrid graphics — dGPU available for rendering.",
                false,
            ),
            (
                "Eco Mode",
                "Disable the discrete GPU to save power (reversible; reboot may be required by the kernel).",
                true,
            ),
            (
                "Optimized",
                "Requires a MUX switch — not present on the FX706HF.",
                false,
            ),
        ];
        ui.horizontal_wrapped(|ui| {
            for (i, (name, desc, is_eco)) in modes.iter().enumerate() {
                let selected = if *is_eco { dgpu_off } else if i == 0 { !dgpu_off } else { false };
                egui::Frame::none()
                    .fill(if selected {
                        egui::Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), 24)
                    } else {
                        egui::Color32::from_rgb(24, 24, 27)
                    })
                    .rounding(8.0)
                    .stroke(egui::Stroke::new(
                        if selected { 1.5 } else { 1.0 },
                        if selected { accent } else { egui::Color32::from_rgb(44, 44, 48) },
                    ))
                    .inner_margin(14.0)
                    .show(ui, |ui| {
                        ui.set_width(220.0);
                        ui.vertical(|ui| {
                        ui.label(egui::RichText::new(*name).size(16.0).strong());
                        ui.colored_label(egui::Color32::GRAY, *desc);
                        if i == 2 {
                            ui.colored_label(
                                egui::Color32::from_rgb(160, 140, 110),
                                "Unsupported on this hardware",
                            );
                        } else if ui
                            .add_enabled(
                                i != 1 || self.free_confirm,
                                egui::Button::new(if selected {
                                    "Active"
                                } else if i == 0 {
                                    "Use Standard"
                                } else {
                                    "Enable Eco Mode"
                                }),
                            )
                            .clicked()
                        {
                            let disable = i == 1;
                            match self.backend.set_dgpu_disabled(disable, true) {
                                Ok(()) => self.report(
                                    if disable {
                                        "dGPU disabled (Eco Mode)".to_string()
                                    } else {
                                        "dGPU enabled (Standard)".to_string()
                                    },
                                    false,
                                ),
                                Err(e) => self.report(
                                    format!("GPU mode failed: {}", friendly_error(&e)),
                                    true,
                                ),
                            }
                            self.poll();
                        }
                        });
                    });
            }
        });
        ui.add_space(8.0);
        ui.checkbox(
            &mut self.free_confirm,
            "I understand Eco Mode disables the discrete GPU until re-enabled",
        );
    }

    fn ui_audio(&mut self, ui: &mut egui::Ui) {
        ui.heading("Audio");
        ui.colored_label(
            egui::Color32::GRAY,
            "PipeWire device inventory (read-only). Audio routing and effects are owned by your desktop/audio server.",
        );
        ui.add_space(8.0);
        let out = std::process::Command::new("wpctl")
            .arg("status")
            .output();
        match out {
            Ok(o) if o.status.success() => {
                let text = String::from_utf8_lossy(&o.stdout);
                let mut lines: Vec<&str> = text.lines().collect();
                let keep = lines
                    .iter()
                    .position(|l| l.contains("Sinks:"))
                    .unwrap_or(0)
                    .saturating_sub(1);
                lines.truncate(keep + 26);
                ui.monospace(lines.join("\n"));
            }
            _ => {
                ui.colored_label(
                    egui::Color32::from_rgb(220, 160, 90),
                    "wpctl not available — audio inventory requires PipeWire/WirePlumber.",
                );
            }
        }
        if let Ok(o) = std::process::Command::new("wpctl")
            .args(["get-volume", "@DEFAULT_AUDIO_SINK@"])
            .output()
        {
            if o.status.success() {
                ui.add_space(8.0);
                ui.label(format!(
                    "Default output volume: {}",
                    String::from_utf8_lossy(&o.stdout).trim()
                ));
            }
        }
    }

    fn ui_resource_monitor(&mut self, ui: &mut egui::Ui) {
        let Some(t) = self.telemetry.clone() else {
            return;
        };
        ui.heading("Resource Monitor");
        ui.add_space(8.0);
        ui.horizontal_wrapped(|ui| {
            status_chip(
                ui,
                &format!(
                    "CPU {}",
                    Self::temp_c(t.cpu_package_temp_mC)
                ),
                egui::Color32::from_rgb(235, 180, 60),
            );
            if let Some(g) = &t.nvidia {
                status_chip(
                    ui,
                    &format!("GPU {} °C · {} %", g.temperature_c, g.utilization_percent),
                    egui::Color32::from_rgb(120, 200, 140),
                );
            }
            if let Some(r) = t.cpu_fan_rpm {
                status_chip(ui, &format!("Fan {r} RPM"), egui::Color32::from_rgb(100, 170, 235));
            }
            if let (Some(u), Some(tot)) = (t.ram_used_gb, t.ram_total_gb) {
                status_chip(ui, &format!("RAM {u:.1}/{tot:.1} GB"), accent());
            }
            if let Some(b) = &t.battery {
                status_chip(ui, &format!("Battery {} %", b.capacity_percent), egui::Color32::from_rgb(120, 200, 140));
            }
        });
        ui.add_space(10.0);
        ui.label("Top processes by memory:");
        let rows = read_proc_rows(15);
        egui::Grid::new("rm_procs")
            .num_columns(3)
            .spacing([20.0, 4.0])
            .show(ui, |ui| {
                ui.colored_label(egui::Color32::GRAY, "Process");
                ui.colored_label(egui::Color32::GRAY, "PID");
                ui.colored_label(egui::Color32::GRAY, "RAM");
                ui.end_row();
                for r in rows {
                    ui.label(&r.comm);
                    ui.label(r.pid.to_string());
                    ui.label(format!("{} MB", r.rss_mb));
                    ui.end_row();
                }
            });
    }

    fn ui_aura_sync(&mut self, ui: &mut egui::Ui) {
        let accent = self.theme.accent();
        let tabs = [
            (AuraTab::SyncDevices, "Sync devices"),
            (AuraTab::Effects, "Aura Effects"),
            (AuraTab::AiAura, "AI Aura Lighting"),
        ];
        tab_strip(ui, &mut self.aura_sub, &tabs, accent);
        match self.aura_sub {
            AuraTab::SyncDevices => self.ui_aura_sync_devices(ui),
            AuraTab::Effects => self.ui_lighting(ui),
            AuraTab::AiAura => self.ui_ai_aura(ui),
        }
    }

    fn ui_aura_sync_devices(&mut self, ui: &mut egui::Ui) {
        let accent = self.theme.accent();
        ui.label(
            "Apply Aura Sync to the selected device(s), or click “Set device lighting” to choose an effect for that device. \
             Whether lighting effects are supported depends on the model.",
        );
        ui.colored_label(
            egui::Color32::GRAY,
            "FX706HF is a single-zone keyboard; Aura Wallpaper is a display feature.",
        );
        ui.add_space(8.0);
        ui.checkbox(&mut self.sync_select_all, "Select all");
        ui.add_space(8.0);
        ui.horizontal_top(|ui| {
            // laptop card
            egui::Frame::none()
                .fill(egui::Color32::from_rgb(24, 24, 27))
                .rounding(8.0)
                .inner_margin(14.0)
                .show(ui, |ui| {
                    ui.set_width(300.0);
                    ui.vertical(|ui| {
                        let (r, _) = ui.allocate_exact_size(
                            egui::vec2(300.0, 150.0),
                            egui::Sense::hover(),
                        );
                        let now = ui.ctx().input(|i| i.time) as f32;
                        let rate = match self.rgb_speed {
                            0xe1 => 0.55,
                            0xf5 => 1.7,
                            _ => 1.0,
                        };
                        Self::draw_device(
                            &ui.painter_at(r),
                            r,
                            self.rgb_mode,
                            self.rgb_color,
                            now,
                            rate,
                        );
                        ui.colored_label(
                            egui::Color32::WHITE,
                            egui::RichText::new("FX706HF")
                                .size(15.0)
                                .family(rog_family()),
                        );
                        ui.colored_label(
                            egui::Color32::GRAY,
                            format!(
                                "Single-zone · {} · #{:02X}{:02X}{:02X}",
                                Self::aura_mode_name(self.rgb_mode),
                                self.rgb_color[0],
                                self.rgb_color[1],
                                self.rgb_color[2]
                            ),
                        );
                        if ui.link("Set device lighting").clicked() {
                            self.aura_sub = AuraTab::Effects;
                        }
                    });
                });
            ui.add_space(12.0);
            // wallpaper card
            egui::Frame::none()
                .fill(egui::Color32::from_rgb(24, 24, 27))
                .rounding(8.0)
                .inner_margin(14.0)
                .show(ui, |ui| {
                    ui.set_width(300.0);
                    ui.vertical(|ui| {
                        let (r, _) = ui.allocate_exact_size(
                            egui::vec2(300.0, 150.0),
                            egui::Sense::hover(),
                        );
                        let p = ui.painter_at(r);
                        p.rect_filled(r, 6.0, egui::Color32::from_rgb(16, 16, 19));
                        if let Some((_, Some(tex))) =
                            self.wallpapers.get(self.wallpaper_idx)
                        {
                            p.image(
                                tex.id(),
                                r.shrink(4.0),
                                egui::Rect::from_min_max(
                                    egui::pos2(0.0, 0.0),
                                    egui::pos2(1.0, 1.0),
                                ),
                                egui::Color32::WHITE,
                            );
                        }
                        ui.colored_label(
                            egui::Color32::WHITE,
                            egui::RichText::new("Aura Wallpaper")
                                .size(15.0)
                                .family(rog_family()),
                        );
                        ui.colored_label(egui::Color32::GRAY, "Desktop wallpaper");
                        if ui.link("Set device lighting").clicked() {
                            self.tab = Tab::AuraWallpaper;
                        }
                    });
                });
        });
        ui.add_space(16.0);
        egui::Frame::none()
            .fill(egui::Color32::from_rgb(22, 22, 25))
            .rounding(8.0)
            .inner_margin(14.0)
            .show(ui, |ui| {
                ui.set_width(ui.available_width().min(1000.0));
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.label(
                                egui::RichText::new("Aura Performance Mode").strong(),
                            );
                            ui.colored_label(
                                egui::Color32::GRAY,
                                "Windows-side rendering option for smoother effect responsiveness.",
                            );
                        });
                        ui.with_layout(
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                let _ = ui.add_enabled(
                                    false,
                                    egui::Checkbox::without_text(&mut self.aura_perf_mode),
                                );
                                ui.colored_label(
                                    egui::Color32::from_rgb(160, 140, 110),
                                    "Windows-only",
                                );
                            },
                        );
                    });
                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new("Hue Connection").strong());
                            ui.colored_label(
                                egui::Color32::GRAY,
                                "Hue Bridge discovery is a Windows network integration.",
                            );
                        });
                        ui.with_layout(
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                let _ = ui.add_enabled(false, egui::Button::new("Search for devices"));
                            },
                        );
                    });
                });
            });
        ui.add_space(8.0);
        ui.colored_label(
            egui::Color32::GRAY,
            format!(
                "Active accent: #{:02X}{:02X}{:02X}",
                accent.r(),
                accent.g(),
                accent.b()
            ),
        );
    }

    fn ui_ai_aura(&mut self, ui: &mut egui::Ui) {
        let accent = self.theme.accent();
        let w = ui.available_width();
        let (rect, _) = ui.allocate_exact_size(egui::vec2(w, 360.0), egui::Sense::hover());
        let p = ui.painter_at(rect);
        let mut mesh = egui::Mesh::default();
        mesh.colored_vertex(rect.left_top(), egui::Color32::from_rgb(48, 30, 10));
        mesh.colored_vertex(rect.right_top(), egui::Color32::from_rgb(30, 18, 40));
        mesh.colored_vertex(rect.right_bottom(), egui::Color32::from_rgb(10, 26, 18));
        mesh.colored_vertex(rect.left_bottom(), egui::Color32::from_rgb(40, 28, 10));
        mesh.add_triangle(0, 1, 2);
        mesh.add_triangle(0, 2, 3);
        p.add(egui::Shape::mesh(mesh));
        p.text(
            egui::pos2(rect.center().x, rect.top() + 120.0),
            egui::Align2::CENTER_CENTER,
            "Your Style, Shining Brightly!",
            egui::FontId::proportional(34.0),
            egui::Color32::WHITE,
        );
        p.text(
            egui::pos2(rect.center().x, rect.top() + 168.0),
            egui::Align2::CENTER_CENTER,
            "Turn your inspiration into vibrant lighting effects that enhance every moment.",
            egui::FontId::proportional(14.0),
            egui::Color32::from_rgb(220, 220, 225),
        );
        p.text(
            egui::pos2(rect.center().x, rect.bottom() - 90.0),
            egui::Align2::CENTER_CENTER,
            "ASUS AI Aura cloud generation is not available on Linux.",
            egui::FontId::proportional(13.0),
            egui::Color32::from_rgb(200, 180, 140),
        );
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            let _ = ui
                .add_enabled(false, egui::Button::new("Create new (ASUS cloud)"))
                .on_disabled_hover_text("Requires the ASUS AI service and sign-in");
            let _ = ui.add_enabled(false, egui::Button::new("Lighting Gallery (ASUS cloud)"));
            ui.add_space(12.0);
            if ui
                .button("Local alternative: Smart (CPU temp)")
                .clicked()
            {
                self.start_smart();
            }
            if ui
                .button("Local alternative: Adaptive (screen)")
                .clicked()
            {
                self.start_adaptive();
            }
        });
        let _ = accent;
    }

    fn ui_aura_wallpaper(&mut self, ui: &mut egui::Ui) {
        let accent = self.theme.accent();
        ui.heading("Aura Wallpaper");
        ui.colored_label(
            egui::Color32::GRAY,
            "Original local wallpapers (redistributable) plus any locally extracted Armoury references.",
        );
        ui.add_space(8.0);
        if self.wallpapers.is_empty() {
            ui.colored_label(
                egui::Color32::from_rgb(220, 160, 90),
                "No wallpapers found. Run the asset extraction workflow into assets/vendor-reference/wallpapers/.",
            );
            return;
        }
        ui.horizontal_wrapped(|ui| {
            for i in 0..self.wallpapers.len() {
                let (_, tex) = &self.wallpapers[i];
                let selected = self.wallpaper_idx == i;
                let (rect, resp) =
                    ui.allocate_exact_size(egui::vec2(200.0, 120.0), egui::Sense::click());
                let p = ui.painter_at(rect);
                p.rect_filled(rect, 6.0, egui::Color32::from_rgb(24, 24, 27));
                if let Some(tex) = tex {
                    p.image(
                        tex.id(),
                        rect.shrink(3.0),
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        egui::Color32::WHITE,
                    );
                }
                p.rect_stroke(
                    rect,
                    6.0,
                    egui::Stroke::new(
                        if selected { 2.0 } else { 1.0 },
                        if selected {
                            accent
                        } else {
                            egui::Color32::from_rgb(50, 50, 55)
                        },
                    ),
                );
                if resp.clicked() {
                    self.wallpaper_idx = i;
                }
            }
        });
        ui.add_space(8.0);
        let (path, _) = &self.wallpapers[self.wallpaper_idx];
        let path = path.clone();
        ui.label(format!("Selected: {path}"));
        if ui
            .button("Set as XFCE wallpaper")
            .on_hover_text("Uses xfconf-query to set the last-image property for all XFCE workspaces")
            .clicked()
        {
            match set_xfce_wallpaper(&path) {
                Ok(n) => self.report(format!("Wallpaper set on {n} XFCE workspace(s)"), false),
                Err(e) => self.report(format!("Wallpaper: {e}"), true),
            }
        }
        ui.add_space(10.0);
        ui.colored_label(
            egui::Color32::GRAY,
            "Activation switch, auto-suspension, superior mode and multi-display settings are Windows-only features; the gallery and native XFCE wallpaper application are implemented.",
        );
    }

    fn ui_scenario_profiles(&mut self, ui: &mut egui::Ui) {
        ui.heading("Scenario Profiles");
        ui.colored_label(
            egui::Color32::GRAY,
            "Automatically apply a performance profile while a game or application runs, then restore the previous profile.",
        );
        ui.add_space(10.0);
        self.ui_gaming(ui);
    }

    fn ui_macros(&mut self, ui: &mut egui::Ui) {
        ui.heading("Macros");
        ui.colored_label(
            egui::Color32::GRAY,
            "Macro recording and playback on Linux requires unrestricted input injection (uinput/evdev).",
        );
        ui.add_space(10.0);
        egui::Frame::none()
            .fill(egui::Color32::from_rgb(24, 24, 27))
            .rounding(8.0)
            .inner_margin(16.0)
            .show(ui, |ui| {
                ui.set_width(560.0);
                ui.label(egui::RichText::new("Not implemented by design").size(16.0).strong());
                ui.colored_label(
                    egui::Color32::GRAY,
                    "tuf-crate does not install keyboard hooks or inject input events. A safe implementation would need explicit user permission, a virtual device with a narrow scope, and a visible recording indicator. That feature is out of the current safety boundary.",
                );
                ui.add_space(6.0);
                let _ = ui
                    .add_enabled(false, egui::Button::new("Record macro"))
                    .on_disabled_hover_text("Requires an input-injection design review");
            });
    }

    fn ui_gamevisual(&mut self, ui: &mut egui::Ui) {
        ui.heading("GameVisual");
        ui.colored_label(
            egui::Color32::GRAY,
            "Display colour profiles are owned by the GPU driver and desktop. tuf-crate exposes the panel overdrive and GPU controls; it does not override global colour management.",
        );
        ui.add_space(10.0);
        self.ui_display_gpu(ui);
    }

    fn ui_content_platform(&mut self, ui: &mut egui::Ui) {
        ui.heading("Content Platform");
        ui.colored_label(
            egui::Color32::GRAY,
            "ASUS Content Platform is a Windows cloud service. No fake catalogue or login is provided.",
        );
        ui.add_space(10.0);
        ui.label("Local content available in this build:");
        ui.colored_label(
            egui::Color32::from_rgb(120, 200, 140),
            "• Aura wallpapers extracted from the local installation (Aura Wallpaper tab)",
        );
        ui.colored_label(
            egui::Color32::from_rgb(120, 200, 140),
            "• Lighting effect references and theme previews (local asset library)",
        );
    }

    fn ui_feature_library(&mut self, ui: &mut egui::Ui) {
        ui.heading("Feature Library");
        ui.colored_label(
            egui::Color32::GRAY,
            "What this build implements on the FX706HF, and what is intentionally unavailable.",
        );
        ui.add_space(10.0);
        let features = [
            ("Performance profiles (quiet/balanced/performance)", true),
            ("Fan FULLSPEED/AUTO + thermal monitoring", true),
            ("CPU power cap (PL1)", true),
            ("Battery charge limit and charge mode", true),
            ("Keyboard brightness 0–3", true),
            ("11 firmware Aura effects + speed + 0–255 mixing", true),
            ("Software effects: Music, Smart, Adaptive Color", true),
            ("Scenario Profiles (per-process profiles)", true),
            ("Aura Wallpaper gallery + XFCE apply", true),
            ("Aura Creator per-key effects", false),
            ("AI Aura cloud generation", false),
            ("Macro recording/injection", false),
            ("In-app updater", false),
        ];
        for (name, ok) in features {
            if ok {
                ui.colored_label(egui::Color32::from_rgb(120, 200, 140), format!("• {name}"));
            } else {
                ui.colored_label(
                    egui::Color32::from_rgb(160, 140, 110),
                    format!("• {name} — unavailable (documented)"),
                );
            }
        }
    }

    fn ui_user_center(&mut self, ui: &mut egui::Ui) {
        ui.heading("User Center");
        ui.colored_label(
            egui::Color32::GRAY,
            "ASUS account sign-in is a Windows cloud service; no login is attempted here.",
        );
        ui.add_space(10.0);
        let user = std::env::var("USER").unwrap_or_else(|_| "user".into());
        ui.label(format!("Local user: {user}"));
        ui.label(format!(
            "Device: {}",
            self.backend.capabilities().model
        ));
        ui.label(format!(
            "Backend: {} · writes: {}",
            self.backend.name(),
            if self.writes_enabled { "enabled" } else { "monitoring-only" }
        ));
        ui.add_space(8.0);
        ui.colored_label(
            egui::Color32::GRAY,
            "Membership, cloud sync and promotions are not available on Linux.",
        );
    }

    fn ui_settings(&mut self, ui: &mut egui::Ui) {
        let accent = self.theme.accent();
        let tabs = [
            (SettingsTab::General, "General"),
            (SettingsTab::UpdateCenter, "Update Center"),
            (SettingsTab::About, "About"),
        ];
        tab_strip(ui, &mut self.settings_sub, &tabs, accent);
        match self.settings_sub {
            SettingsTab::General => self.ui_settings_general(ui),
            SettingsTab::UpdateCenter => self.ui_settings_update(ui),
            SettingsTab::About => self.ui_about(ui),
        }
    }

    fn ui_settings_general(&mut self, ui: &mut egui::Ui) {
        let accent = self.theme.accent();
        ui.heading("General");
        ui.add_space(8.0);
        ui.label("Launch with");
        let launch_before = self.launch_page.clone();
        egui::ComboBox::from_id_salt("launch_page")
            .selected_text(&self.launch_page)
            .show_ui(ui, |ui| {
                for p in [
                    "Home",
                    "Devices",
                    "Lighting & Aura Sync",
                    "Scenario Profiles",
                    "Settings",
                ] {
                    ui.selectable_value(&mut self.launch_page, p.to_string(), p);
                }
            });
        if self.launch_page != launch_before {
            self.save_ui_settings();
        }
        ui.add_space(12.0);
        ui.label("Customize theme");
        ui.colored_label(
            egui::Color32::GRAY,
            "Theme previews use the original Armoury Crate assets from the local reference library.",
        );
        ui.add_space(6.0);
        let previews = self.theme_previews.clone();
        ui.horizontal_wrapped(|ui| {
            for (theme, tex) in previews {
                let selected = self.theme == theme;
                let (rect, resp) =
                    ui.allocate_exact_size(egui::vec2(200.0, 120.0), egui::Sense::click());
                let p = ui.painter_at(rect);
                p.rect_filled(rect, 6.0, egui::Color32::from_rgb(24, 24, 27));
                if let Some(tex) = &tex {
                    p.image(
                        tex.id(),
                        rect.shrink(4.0),
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        egui::Color32::WHITE,
                    );
                } else {
                    p.rect_filled(
                        rect.shrink(4.0),
                        4.0,
                        egui::Color32::from_rgb(
                            theme.accent().r() / 3,
                            theme.accent().g() / 3,
                            theme.accent().b() / 3,
                        ),
                    );
                }
                p.rect_stroke(
                    rect,
                    6.0,
                    egui::Stroke::new(
                        if selected { 2.0 } else { 1.0 },
                        if selected {
                            theme.accent()
                        } else {
                            egui::Color32::from_rgb(48, 48, 53)
                        },
                    ),
                );
                p.text(
                    egui::pos2(rect.center().x, rect.bottom() - 12.0),
                    egui::Align2::CENTER_CENTER,
                    theme.label(),
                    egui::FontId::proportional(11.5),
                    egui::Color32::WHITE,
                );
                if resp.clicked() {
                    self.theme = theme;
                    apply_theme_visuals(ui.ctx(), theme);
                    self.save_ui_settings();
                }
            }
        });
        ui.add_space(12.0);
        ui.label("Device image style");
        ui.horizontal(|ui| {
            ui.radio_value(&mut self.device_line_art, false, "Photographic");
            ui.radio_value(&mut self.device_line_art, true, "Sketch / line art");
            if ui.button("Reload device image").clicked() {
                self.report(
                    "Device image is delivered by ASUS cloud and is not cached on this machine; the local vector render is used."
                        .to_string(),
                    false,
                );
            }
        });
        ui.add_space(10.0);
        ui.colored_label(
            egui::Color32::GRAY,
            format!("#{:02X}{:02X}{:02X}", accent.r(), accent.g(), accent.b()),
        );
    }

    fn ui_settings_update(&mut self, ui: &mut egui::Ui) {
        ui.heading("Update Center");
        ui.colored_label(
            egui::Color32::GRAY,
            "Native Debian package state. tuf-crate never installs packages, adds repositories, or runs unverified scripts.",
        );
        ui.add_space(8.0);
        ui.label(format!("tuf-crate {}", env!("CARGO_PKG_VERSION")));
        if let Ok(v) = std::fs::read_to_string("/proc/sys/kernel/osrelease") {
            ui.label(format!("Kernel: {}", v.trim()));
        }
        if let Ok(v) = std::fs::read_to_string("/etc/debian_version") {
            ui.label(format!("Debian: {}", v.trim()));
        }
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui.button("Check for updates").clicked() {
                match std::process::Command::new("apt")
                    .args(["list", "--upgradable"])
                    .output()
                {
                    Ok(o) => {
                        let text = String::from_utf8_lossy(&o.stdout).to_string();
                        let count = text
                            .lines()
                            .filter(|l| l.contains('/') && !l.starts_with("Listing"))
                            .count();
                        self.update_text = format!(
                            "{count} upgradable package(s)\n\n{text}"
                        );
                    }
                    Err(e) => self.update_text = format!("apt not available: {e}"),
                }
            }
            if ui
                .add_enabled(false, egui::Button::new("Update All"))
                .on_disabled_hover_text("Use your package manager (apt) to apply updates")
                .clicked()
            {}
        });
        if !self.update_text.is_empty() {
            ui.add_space(6.0);
            egui::ScrollArea::vertical()
                .max_height(320.0)
                .show(ui, |ui| {
                    ui.monospace(&self.update_text);
                });
        }
        ui.add_space(8.0);
        ui.colored_label(
            egui::Color32::GRAY,
            "Firmware updates: use fwupd (if installed) and the vendor tooling; they are not automated here.",
        );
    }

    fn save_ui_settings(&self) {
        let path = ui_settings_path();
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let doc = serde_json::json!({
            "theme": format!("{:?}", self.theme),
            "launch_page": self.launch_page,
            "device_line_art": self.device_line_art,
        });
        let _ = std::fs::write(path, serde_json::to_string_pretty(&doc).unwrap_or_default());
    }

    fn ui_home(&mut self, ui: &mut egui::Ui) {
        let Some(t) = self.telemetry.clone() else {
            return;
        };
        let orange = accent();
        let white = egui::Color32::from_rgb(235, 235, 240);
        let gray = egui::Color32::GRAY;
        let now = ui.ctx().input(|i| i.time) as f32;
        let rate = match self.rgb_speed {
            0xe1 => 0.55,
            0xf5 => 1.7,
            _ => 1.0,
        };
        let avail = ui.available_width();
        let left_w = (avail * 0.42).clamp(420.0, 640.0);

        ui.horizontal_top(|ui| {
            // ---------------- left column: product + devices ----------------
            ui.vertical(|ui| {
                ui.set_width(left_w);
                let (card, _) =
                    ui.allocate_exact_size(egui::vec2(left_w, 540.0), egui::Sense::hover());
                let p = ui.painter_at(card);
                gradient_rect(
                    &p,
                    card,
                    egui::Color32::from_rgb(30, 30, 33),
                    egui::Color32::from_rgb(15, 15, 17),
                );
                p.rect_stroke(
                    card,
                    10.0,
                    egui::Stroke::new(1.0, egui::Color32::from_rgb(42, 42, 46)),
                );
                let art = egui::Rect::from_center_size(
                    egui::pos2(card.center().x, card.top() + 245.0),
                    egui::vec2(card.width() * 0.96, 350.0),
                );
                draw_product(&p, art, now, rate, self.rgb_color, self.rgb_mode);
                p.text(
                    egui::pos2(card.center().x, card.bottom() - 108.0),
                    egui::Align2::CENTER_CENTER,
                    "ASUS TUF GAMING F17",
                    rog_font(23.0),
                    white,
                );
                p.text(
                    egui::pos2(card.center().x, card.bottom() - 80.0),
                    egui::Align2::CENTER_CENTER,
                    "11th Gen Intel(R) Core(TM) i5-11260H @ 2.60GHz",
                    egui::FontId::proportional(11.5),
                    gray,
                );
                p.text(
                    egui::pos2(card.center().x, card.bottom() - 62.0),
                    egui::Align2::CENTER_CENTER,
                    "NVIDIA GeForce RTX 2050 GDDR6 4 GB 64-bit",
                    egui::FontId::proportional(11.5),
                    gray,
                );
                p.text(
                    egui::pos2(card.center().x, card.bottom() - 34.0),
                    egui::Align2::CENTER_CENTER,
                    format!(
                        "{}  ·  CPU {:.0} °C  ·  Fan {} RPM",
                        t.profile.clone().unwrap_or_else(|| "—".into()),
                        t.cpu_package_temp_mC
                            .map(|m| m as f32 / 1000.0)
                            .unwrap_or(0.0),
                        t.cpu_fan_rpm.unwrap_or(0)
                    ),
                    egui::FontId::proportional(11.0),
                    orange,
                );
                ui.add_space(12.0);

                // Devices (0) card
                let (dev, devresp) =
                    ui.allocate_exact_size(egui::vec2(left_w, 108.0), egui::Sense::click());
                let dp = ui.painter_at(dev);
                dp.rect_filled(dev, 8.0, egui::Color32::from_rgb(24, 24, 27));
                dp.rect_stroke(
                    dev,
                    8.0,
                    egui::Stroke::new(1.0, egui::Color32::from_rgb(44, 44, 48)),
                );
                dp.text(
                    egui::pos2(dev.left() + 16.0, dev.top() + 18.0),
                    egui::Align2::LEFT_CENTER,
                    ">>> Devices (0)",
                    rog_font(14.0),
                    orange,
                );
                dp.text(
                    egui::pos2(dev.right() - 16.0, dev.top() + 18.0),
                    egui::Align2::RIGHT_CENTER,
                    "View All",
                    egui::FontId::proportional(12.0),
                    gray,
                );
                let plus = egui::Rect::from_center_size(
                    egui::pos2(dev.left() + 58.0, dev.center().y + 12.0),
                    egui::vec2(44.0, 44.0),
                );
                dp.rect_stroke(
                    plus,
                    3.0,
                    egui::Stroke::new(1.0, egui::Color32::from_rgb(72, 72, 80)),
                );
                dp.add(egui::Shape::line(
                    vec![plus.left_center(), plus.right_center()],
                    egui::Stroke::new(1.5, egui::Color32::from_rgb(120, 120, 130)),
                ));
                dp.add(egui::Shape::line(
                    vec![plus.center_top(), plus.center_bottom()],
                    egui::Stroke::new(1.5, egui::Color32::from_rgb(120, 120, 130)),
                ));
                dp.text(
                    egui::pos2(plus.right() + 18.0, plus.center().y - 9.0),
                    egui::Align2::LEFT_CENTER,
                    "No Aura devices connected",
                    egui::FontId::proportional(12.5),
                    gray,
                );
                dp.text(
                    egui::pos2(plus.right() + 18.0, plus.center().y + 10.0),
                    egui::Align2::LEFT_CENTER,
                    "Peripherals appear here when paired.",
                    egui::FontId::proportional(11.0),
                    egui::Color32::from_rgb(120, 120, 130),
                );
                if devresp.clicked() {
                    self.tab = Tab::Devices;
                }
            });

            ui.add_space(18.0);

            // ---------------- right column: system usage + mode ----------------
            let right_w = (avail - left_w - 34.0).max(420.0);
            ui.vertical(|ui| {
                ui.set_max_width(right_w);
                ui.colored_label(
                    orange,
                    egui::RichText::new(">>> System Usage").size(15.0).strong(),
                );
                ui.add_space(6.0);
                let col_w = (right_w - 26.0) / 2.0;
                let cpu_mhz = std::fs::read_to_string(
                    "/sys/devices/system/cpu/cpu0/cpufreq/scaling_cur_freq",
                )
                .ok()
                .and_then(|s| s.trim().parse::<u64>().ok())
                .map(|k| k / 1000);
                ui.horizontal_top(|ui| {
                    ui.vertical(|ui| {
                        ui.set_width(col_w);
                        usage_cpu(ui, &t, self.cpu_util, cpu_mhz);
                    });
                    ui.add_space(14.0);
                    ui.vertical(|ui| {
                        ui.set_width(col_w);
                        usage_gpu(ui, &t);
                    });
                });
                ui.add_space(10.0);
                ui.horizontal_top(|ui| {
                    ui.vertical(|ui| {
                        ui.set_width(col_w);
                        usage_fan(ui, &t);
                    });
                    ui.add_space(14.0);
                    ui.vertical(|ui| {
                        ui.set_width(col_w);
                        usage_mem(ui, &t);
                    });
                });
                ui.add_space(14.0);
                ui.colored_label(
                    orange,
                    egui::RichText::new(">>> Operating Mode").size(15.0).strong(),
                );
                ui.add_space(6.0);
                let current = t.profile.clone().unwrap_or_default();
                let modes: [(&str, u8, Option<Profile>, &str); 4] = [
                    (
                        "Windows",
                        0,
                        None,
                        "No Linux equivalent — desktop/TLP owns power management",
                    ),
                    ("Silent", 1, Some(Profile::Quiet), "platform_profile: quiet"),
                    (
                        "Performance",
                        2,
                        Some(Profile::Balanced),
                        "platform_profile: balanced",
                    ),
                    (
                        "Turbo",
                        3,
                        Some(Profile::Performance),
                        "platform_profile: performance",
                    ),
                ];
                let card_w = (right_w - 3.0 * 10.0) / 4.0;
                ui.horizontal(|ui| {
                    for (name, icon, profile, hint) in modes {
                        let selected =
                            profile.map(|p| current == p.as_str()).unwrap_or(false);
                        let (rect, resp) = ui
                            .allocate_exact_size(egui::vec2(card_w, 86.0), egui::Sense::click());
                        let p = ui.painter_at(rect);
                        p.rect_filled(
                            rect,
                            6.0,
                            if selected {
                                egui::Color32::from_rgb(34, 27, 19)
                            } else {
                                egui::Color32::from_rgb(22, 22, 25)
                            },
                        );
                        p.rect_stroke(
                            rect,
                            6.0,
                            egui::Stroke::new(
                                if selected { 1.5 } else { 1.0 },
                                if selected {
                                    orange
                                } else {
                                    egui::Color32::from_rgb(44, 44, 48)
                                },
                            ),
                        );
                        mode_icon(
                            &p,
                            egui::pos2(rect.center().x, rect.top() + 30.0),
                            icon,
                            if selected {
                                orange
                            } else {
                                egui::Color32::from_rgb(190, 190, 200)
                            },
                        );
                        p.text(
                            egui::pos2(rect.center().x, rect.bottom() - 16.0),
                            egui::Align2::CENTER_CENTER,
                            name,
                            egui::FontId::proportional(12.5),
                            if selected { orange } else { gray },
                        );
                        let resp = resp.on_hover_text(hint);
                        if profile.is_some() && resp.clicked() {
                            if let Some(pr) = profile {
                                match self.backend.set_profile(pr, true) {
                                    Ok(()) => self.report(
                                        format!("profile -> {}", pr.label()),
                                        false,
                                    ),
                                    Err(e) => self.report(
                                        format!("profile failed: {}", friendly_error(&e)),
                                        true,
                                    ),
                                }
                                self.poll();
                            }
                        }
                    }
                });
                ui.add_space(8.0);
                ui.colored_label(
                    gray,
                    "ARMOURY CRATE is a trademark of ASUSTeK Computer Inc. tuf-crate is an independent, unofficial Linux tool.",
                );
            });
        });
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
                    accent(),
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

    /// Poll the process list and apply/restore game-profile rules.
    fn check_game_rules(&mut self) {
        if self.last_game_check.elapsed() < std::time::Duration::from_secs(4) {
            return;
        }
        self.last_game_check = std::time::Instant::now();
        if self.game_rules.is_empty() {
            if self.active_game_rule.take().is_some() {
                if let Some(prev) = self.restore_profile.take() {
                    if let Some(p) = Profile::from_kernel_str(&prev) {
                        let _ = self.backend.set_profile(p, true);
                    }
                }
            }
            return;
        }
        let running = games::running_processes();
        let matched = games::match_rule(&self.game_rules, &running).cloned();
        let new_name = matched.as_ref().map(|r| r.process.clone());
        if new_name == self.active_game_rule {
            return;
        }
        match matched {
            Some(rule) => {
                if self.active_game_rule.is_none() {
                    self.restore_profile =
                        self.telemetry.as_ref().and_then(|t| t.profile.clone());
                }
                if let Some(p) = Profile::from_kernel_str(&rule.profile) {
                    match self.backend.set_profile(p, true) {
                        Ok(()) => self.report(
                            format!("Gaming: '{}' → {} profile", rule.process, rule.profile),
                            false,
                        ),
                        Err(e) => self.report(
                            format!("Gaming profile failed: {}", friendly_error(&e)),
                            true,
                        ),
                    }
                }
            }
            None => {
                if let Some(prev) = self.restore_profile.take() {
                    if let Some(p) = Profile::from_kernel_str(&prev) {
                        let _ = self.backend.set_profile(p, true);
                        self.report("Gaming: game exited — profile restored".to_string(), false);
                    }
                }
            }
        }
        self.active_game_rule = new_name;
        self.poll();
    }

    fn ui_gaming(&mut self, ui: &mut egui::Ui) {
        ui.heading("Gaming profiles");
        ui.label(
            "Automatically apply a performance profile while a game or application is running \
             (matched by the process name in /proc/<pid>/comm, case-insensitive).",
        );
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label("Process:");
            ui.add(
                egui::TextEdit::singleline(&mut self.game_input)
                    .desired_width(160.0)
                    .hint_text("e.g. cs2"),
            );
            egui::ComboBox::from_id_salt("game_profile")
                .selected_text(&self.game_profile)
                .show_ui(ui, |ui| {
                    for p in games::RULE_PROFILES {
                        ui.selectable_value(&mut self.game_profile, p.to_string(), p);
                    }
                });
            if ui.button("➕ Add rule").clicked() {
                match games::add(&self.game_input, &self.game_profile) {
                    Ok(()) => {
                        self.game_rules = games::load_all();
                        self.game_input.clear();
                        self.report("Gaming rule added".to_string(), false);
                    }
                    Err(e) => self.report(format!("Gaming rule: {e}"), true),
                }
            }
        });
        ui.add_space(6.0);
        if self.game_rules.is_empty() {
            ui.colored_label(
                egui::Color32::GRAY,
                "No rules yet — add a process name and pick the profile it should trigger.",
            );
        } else {
            egui::Grid::new("game_rules")
                .num_columns(3)
                .spacing([12.0, 6.0])
                .show(ui, |ui| {
                    for rule in self.game_rules.clone() {
                        let mut enabled = rule.enabled;
                        if ui
                            .checkbox(&mut enabled, "")
                            .on_hover_text("enable / disable this rule")
                            .changed()
                        {
                            let _ = games::set_enabled(&rule.process, enabled);
                            self.game_rules = games::load_all();
                        }
                        ui.label(format!("{}  →  {}", rule.process, rule.profile));
                        if ui
                            .small_button("✕")
                            .on_hover_text("delete rule")
                            .clicked()
                        {
                            let _ = games::remove(&rule.process);
                            self.game_rules = games::load_all();
                        }
                        ui.end_row();
                    }
                });
        }
        ui.add_space(8.0);
        match &self.active_game_rule {
            Some(p) => {
                ui.colored_label(
                    egui::Color32::from_rgb(120, 200, 120),
                    format!("• Active: '{p}' is running — its profile is applied"),
                );
            }
            None => {
                ui.colored_label(
                    egui::Color32::GRAY,
                    "○ Watching the process list (checks every 4 s while tuf-crate is open)",
                );
            }
        }
        ui.colored_label(
            egui::Color32::GRAY,
            "The profile from before the game started is restored when the game exits. \
             Games launched through Steam/Wine often show their wrapper name — use the name \
             that appears in `ps -eo comm` (max 15 characters).",
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
                "• LIVE · WMNB DEVS 0x00100056",
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
        let orange = accent();
        let white = egui::Color32::from_rgb(235, 235, 240);
        let gray = egui::Color32::GRAY;
        let modes: [(&str, u32, &str); 11] = [
            ("Static", 0, "Solid colour"),
            ("Breathing", 1, "Fade in / out"),
            ("Color Cycle", 2, "Sweep through the spectrum"),
            ("Rainbow", 3, "Animated rainbow"),
            ("Star", 4, "Twinkling stars"),
            ("Rain", 5, "Falling drops"),
            ("Highlight", 6, "Travelling band"),
            ("Laser", 7, "Sweeping beam"),
            ("Ripple", 8, "Expanding ripples"),
            ("Strobe", 10, "Fast flashing"),
            ("Comet", 11, "Comet with a tail"),
        ];
        ui.horizontal_top(|ui| {
            egui::Grid::new("aura_icons")
                .num_columns(8)
                .spacing([4.0, 6.0])
                .show(ui, |ui| {
                    for (i, (name, mode, desc)) in modes.iter().enumerate() {
                        let selected = self.rgb_mode == *mode;
                        let (rect, resp) = ui.allocate_exact_size(
                            egui::vec2(108.0, 92.0),
                            egui::Sense::click(),
                        );
                        let p = ui.painter_at(rect);
                        if selected {
                            p.rect_filled(rect, 8.0, egui::Color32::from_rgb(34, 27, 19));
                            p.rect_stroke(
                                rect,
                                8.0,
                                egui::Stroke::new(1.0, orange),
                            );
                        } else if resp.hovered() {
                            p.rect_filled(rect, 8.0, egui::Color32::from_rgb(24, 25, 30));
                        }
                        effect_icon(
                            &p,
                            egui::pos2(rect.center().x, rect.top() + 34.0),
                            *mode,
                            if selected {
                                orange
                            } else {
                                egui::Color32::from_rgb(205, 205, 215)
                            },
                        );
                        p.text(
                            egui::pos2(rect.center().x, rect.bottom() - 13.0),
                            egui::Align2::CENTER_CENTER,
                            *name,
                            egui::FontId::proportional(11.5),
                            if selected { orange } else { gray },
                        );
                        let resp = resp.on_hover_text(*desc);
                        if resp.clicked() {
                            self.rgb_mode = *mode;
                            self.apply_rgb(*mode);
                        }
                        if i % 8 == 7 {
                            ui.end_row();
                        }
                    }
                    if !modes.len().is_multiple_of(8) {
                        ui.end_row();
                    }
                });
            let spacer = (ui.available_width() - 268.0).max(8.0);
            ui.add_space(spacer);
            // Armoury-style detail panel for the selected effect
            egui::Frame::none()
                .fill(egui::Color32::from_rgb(18, 19, 24))
                .rounding(8.0)
                .inner_margin(14.0)
                .show(ui, |ui| {
                    ui.set_width(250.0);
                    ui.vertical(|ui| {
                        ui.colored_label(
                            white,
                            egui::RichText::new(Self::aura_mode_name(self.rgb_mode))
                                .size(20.0)
                                .strong(),
                        );
                        ui.add_space(8.0);
                        ui.colored_label(gray, "Pattern");
                        let (strip, _) = ui.allocate_exact_size(
                            egui::vec2(ui.available_width(), 18.0),
                            egui::Sense::hover(),
                        );
                        let p = ui.painter_at(strip);
                        let spectrum = matches!(self.rgb_mode, 2 | 3);
                        for i in 0..36 {
                            let c = if spectrum {
                                Self::hsv_to_rgb(i as f32 * 10.0, 1.0, 1.0)
                            } else {
                                Self::hsv_to_rgb(
                                    self.rgb_hue,
                                    1.0,
                                    0.35 + 0.65 * (i as f32 / 35.0),
                                )
                            };
                            p.rect_filled(
                                egui::Rect::from_min_size(
                                    egui::pos2(
                                        strip.left() + strip.width() * i as f32 / 36.0,
                                        strip.top(),
                                    ),
                                    egui::vec2(strip.width() / 36.0 + 0.5, 18.0),
                                ),
                                0.0,
                                egui::Color32::from_rgb(c[0], c[1], c[2]),
                            );
                        }
                        ui.add_space(8.0);
                        ui.colored_label(gray, "Speed");
                        ui.horizontal(|ui| {
                            for (val, label) in
                                [(0xe1u32, "33 %"), (0xeb, "67 %"), (0xf5, "100 %")]
                            {
                                if ui
                                    .selectable_label(self.rgb_speed == val, label)
                                    .clicked()
                                {
                                    self.rgb_speed = val;
                                    self.apply_rgb(self.rgb_mode);
                                }
                            }
                        });
                        ui.add_space(10.0);
                        if ui
                            .add_sized(
                                [ui.available_width(), 32.0],
                                egui::Button::new("Apply to keyboard")
                                    .fill(egui::Color32::from_rgb(45, 60, 45)),
                            )
                            .clicked()
                        {
                            self.apply_rgb(self.rgb_mode);
                        }
                        if ui
                            .add_sized(
                                [ui.available_width(), 26.0],
                                egui::Button::new("Reset to default"),
                            )
                            .clicked()
                        {
                            self.rgb_mode = 0;
                            self.rgb_color = [255, 255, 255];
                            self.rgb_hue = 0.0;
                            self.rgb_sat = 0.0;
                            self.rgb_val = 1.0;
                            self.apply_rgb(0);
                        }
                        ui.add_space(6.0);
                        ui.colored_label(
                            gray,
                            format!(
                                "#{:02X}{:02X}{:02X} · speed {} %",
                                self.rgb_color[0],
                                self.rgb_color[1],
                                self.rgb_color[2],
                                Self::speed_percent(self.rgb_speed)
                            ),
                        );
                    });
                });
        });
        ui.add_space(6.0);
        ui.colored_label(
            egui::Color32::from_rgb(190, 170, 120),
            "Note: Rain, Highlight, Laser, Ripple, Star and Comet render multicolour in this \
             firmware regardless of the picker; Static, Breathing and Strobe follow the chosen \
             colour, Color Cycle and Rainbow are spectrum effects by design.",
        );
        ui.add_space(8.0);

        // ---------- software effects (Linux-native) ----------
        ui.label("Software effects (Linux-native)");
        ui.horizontal(|ui| {
            let music_active = self.soft_effect == Some(music::MUSIC_EFFECT_ID);
            let smart_active = self.soft_effect == Some(music::SMART_EFFECT_ID);
            let adaptive_active = self.soft_effect == Some(music::ADAPTIVE_EFFECT_ID);
            let mut clicked: Option<u32> = None;
            for (id, active, label) in [
                (music::MUSIC_EFFECT_ID, music_active, "Music"),
                (music::SMART_EFFECT_ID, smart_active, "Smart (temp)"),
                (music::ADAPTIVE_EFFECT_ID, adaptive_active, "Adaptive (screen)"),
            ] {
                let text = if active {
                    format!("Stop {label}")
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
        Self::draw_device(
            &p,
            rect.shrink2(egui::vec2(8.0, 6.0)),
            self.rgb_mode,
            self.rgb_color,
            t,
            rate,
        );
    }

    /// Draw the laptop with an animated keyboard into `rect`. Used by the
    /// Lighting tab and the Home hero card.
    fn draw_device(
        p: &egui::Painter,
        rect: egui::Rect,
        mode: u32,
        rgb: [u8; 3],
        t: f32,
        rate: f32,
    ) {
        let (bh, bs, bv) = Self::rgb_to_hsv(rgb);
        // glow
        for i in (0..7).rev() {
            let f = i as f32 / 7.0;
            let glow_rect = egui::Rect::from_center_size(
                rect.center() + egui::vec2(0.0, rect.height() * 0.18),
                egui::vec2(
                    rect.width() * (0.55 + f * 0.3),
                    rect.height() * (0.45 + f * 0.3),
                ),
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
            rect.center(),
            egui::vec2(rect.width() * 0.80, rect.height() * 0.88),
        );
        p.rect_filled(body, 10.0, egui::Color32::from_rgb(28, 28, 34));
        let screen = egui::Rect::from_min_size(
            body.min + egui::vec2(14.0, 10.0),
            egui::vec2(body.width() - 28.0, body.height() * 0.30),
        );
        p.rect_filled(screen, 4.0, egui::Color32::from_rgb(9, 12, 18));
        p.text(
            screen.center(),
            egui::Align2::CENTER_CENTER,
            "TUF GAMING F17 · tuf-crate",
            egui::FontId::proportional(12.0),
            egui::Color32::from_rgb(120, 200, 140),
        );
        // keyboard deck
        let deck = egui::Rect::from_min_size(
            egui::pos2(screen.left(), screen.bottom() + 8.0),
            egui::vec2(screen.width(), body.bottom() - screen.bottom() - 20.0),
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
                        mode,
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
        ui.add_space(14.0);
        ui.heading("Armoury feature parity");
        ui.label("Implemented on this machine:");
        for item in [
            "Silent / Balanced / Performance profiles",
            "Fan FULLSPEED / AUTO (safety modes)",
            "CPU power cap (PL1, 25–65 W)",
            "Battery charge limit + charge mode",
            "Keyboard brightness 0/33/66/100 %",
            "RGB: 11 Aura effects, speed 33/67/100 %, 0–255 colour mixing",
            "Software effects: Music (audio), Smart (CPU temp), Adaptive (screen)",
            "Gaming profiles (per-process auto profile + restore)",
            "Panel overdrive, dGPU disable",
            "Full telemetry, boot persistence, animated UI",
        ] {
            ui.colored_label(
                egui::Color32::from_rgb(120, 200, 120),
                format!("• {item}"),
            );
        }
        ui.add_space(6.0);
        ui.label("Explicitly out of scope (documented, not missing):");
        for (item, why) in [
            (
                "AI Aura",
                "needs ASUS's Windows AI service/models; Smart + Adaptive cover adaptive lighting",
            ),
            (
                "Aura Creator per-key effects",
                "this keyboard is single-zone — per-key lighting is physically impossible",
            ),
            (
                "In-app updater",
                "updates belong to your package manager / git checkout; no network or privilege surface added",
            ),
            (
                "Aura Sync",
                "no compatible ASUS peripherals connected",
            ),
            (
                "Windows-only services",
                "Armoury Crate account, game library and ASUS services are Windows components",
            ),
        ] {
            ui.colored_label(
                egui::Color32::from_rgb(180, 180, 190),
                format!("• {item} — {why}"),
            );
        }
        ui.add_space(12.0);
        ui.heading("Updates");
        ui.label(format!(
            "tuf-crate {} — update with your package manager or a git pull + rebuild. There is no in-app updater by design: it would add a network and privilege surface for no hardware benefit.",
            env!("CARGO_PKG_VERSION")
        ));
        ui.add_space(10.0);
        ui.label("tuf-crate builds exclusively on kernel-sanctioned interfaces (asus-wmi / asus-armoury / leds / hwmon / power_supply). It does not write EC registers, invoke ACPI methods, patch kernels, or touch fan PWM controls. Every accepted write is clamped, capability-gated and audited. See docs/SAFETY.md, docs/RGB.md and docs/SCOPE.md.");
    }
}

fn yes_no(b: bool) -> &'static str {
    if b {
        "yes"
    } else {
        "no"
    }
}

#[allow(dead_code)]
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
