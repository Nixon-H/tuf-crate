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

    let window_size = std::env::var("TUF_CRATE_WINDOW")
        .ok()
        .and_then(|v| {
            let (w, h) = v.split_once('x')?;
            Some([w.parse::<f32>().ok()?, h.parse::<f32>().ok()?])
        })
        .unwrap_or([1920.0, 1000.0]);
    let opts = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size(window_size)
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
    v.panel_fill = page_bg();
    v.window_fill = egui::Color32::from_rgb(22, 24, 28);
    v.extreme_bg_color = sidebar_bg();
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
                    vec![c, egui::pos2(c.x + 7.0 * a.cos(), c.y + 7.0 * a.sin())],
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
                    egui::Rect::from_min_size(egui::pos2(c.x + dx, c.y + dy), egui::vec2(5.0, 5.0)),
                    1.0,
                    s,
                );
            }
        }
        12 => {
            // speaker / audio
            p.add(egui::Shape::convex_polygon(
                vec![
                    egui::pos2(c.x - 9.0, c.y - 4.0),
                    egui::pos2(c.x - 3.0, c.y - 4.0),
                    egui::pos2(c.x + 3.0, c.y - 9.0),
                    egui::pos2(c.x + 3.0, c.y + 9.0),
                    egui::pos2(c.x - 3.0, c.y + 4.0),
                    egui::pos2(c.x - 9.0, c.y + 4.0),
                ],
                col,
                egui::Stroke::NONE,
            ));
            for (r, a) in [(6.0f32, 0.55f32), (9.0, 0.30)] {
                let pts: Vec<egui::Pos2> = (0..=8)
                    .map(|i| {
                        let ang = -a + 2.0 * a * i as f32 / 8.0;
                        egui::pos2(c.x + 4.0 + r * ang.cos(), c.y + r * ang.sin())
                    })
                    .collect();
                p.add(egui::Shape::line(pts, egui::Stroke::new(1.2, col)));
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
fn gradient_rect(p: &egui::Painter, rect: egui::Rect, top: egui::Color32, bottom: egui::Color32) {
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
    line_art: bool,
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
    if line_art {
        gradient_rect(
            p,
            scr,
            egui::Color32::from_rgb(235, 235, 238),
            egui::Color32::from_rgb(200, 200, 206),
        );
    } else {
        gradient_rect(
            p,
            scr,
            egui::Color32::from_rgb(24, 31, 44),
            egui::Color32::from_rgb(9, 11, 16),
        );
    }
    // geometric TUF-style backdrop
    for i in 0..5 {
        let x = scr.left() + scr.width() * (0.16 + i as f32 * 0.17);
        p.add(egui::Shape::line(
            vec![
                egui::pos2(x, scr.top() + scr.height() * 0.16),
                egui::pos2(x - scr.width() * 0.07, scr.bottom() - scr.height() * 0.14),
            ],
            egui::Stroke::new(
                1.0,
                egui::Color32::from_rgba_unmultiplied(120, 170, 220, 40),
            ),
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
        egui::pos2(
            rect.right() - rect.width() * 0.05,
            dy + rect.height() * 0.30,
        ),
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
        {
            let a = accent();
            egui::Color32::from_rgba_unmultiplied(a.r(), a.g(), a.b(), 14)
        },
    );
}

/// Draw the official product render fitted into the hero area, with a soft glow.
fn draw_hero_texture(p: &egui::Painter, rect: egui::Rect, tex: &egui::TextureHandle) {
    let size = tex.size_vec2();
    if size.x <= 0.0 || size.y <= 0.0 {
        return;
    }
    // Armoury shows the render slightly cropped at the sides so it fills the
    // hero slot instead of being letterboxed.
    let scale = (rect.width() / size.x).max(rect.height() / size.y);
    let drawn = egui::vec2(size.x * scale, size.y * scale);
    let u = (rect.width() / drawn.x).clamp(0.0, 1.0);
    let v = (rect.height() / drawn.y).clamp(0.0, 1.0);
    let uv = egui::Rect::from_min_max(
        egui::pos2((1.0 - u) / 2.0, (1.0 - v) / 2.0),
        egui::pos2((1.0 + u) / 2.0, (1.0 + v) / 2.0),
    );
    p.image(tex.id(), rect, uv, egui::Color32::WHITE);
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
                    vec![egui::pos2(cc.x - r, cc.y), egui::pos2(cc.x + r, cc.y)],
                    thin,
                ));
                p.add(egui::Shape::line(
                    vec![egui::pos2(cc.x, cc.y - r), egui::pos2(cc.x, cc.y + r)],
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
                        vec![egui::pos2(c.x + dx, y), egui::pos2(c.x + dx, y + 4.0)],
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

/// Larger, filled device-control glyphs for the Configuration cards.
fn device_icon(p: &egui::Painter, c: egui::Pos2, kind: u8, col: egui::Color32) {
    let s = egui::Stroke::new(1.9, col);
    match kind {
        0 => {
            // gauge + needle
            let pts: Vec<egui::Pos2> = (0..=20)
                .map(|i| {
                    let a = std::f32::consts::PI * (0.12 + 0.76 * i as f32 / 20.0);
                    egui::pos2(c.x + 12.0 * a.cos(), c.y + 6.0 - 12.0 * a.sin())
                })
                .collect();
            p.add(egui::Shape::line(pts, s));
            p.add(egui::Shape::line(
                vec![c, egui::pos2(c.x + 7.0, c.y - 3.0)],
                egui::Stroke::new(2.2, col),
            ));
            p.circle_filled(c, 2.2, col);
            for a in [0.20f32, 0.50, 0.80] {
                let ang = std::f32::consts::PI * a;
                p.add(egui::Shape::line(
                    vec![
                        egui::pos2(c.x + 9.0 * ang.cos(), c.y + 3.0 - 9.0 * ang.sin()),
                        egui::pos2(c.x + 11.0 * ang.cos(), c.y + 3.0 - 11.0 * ang.sin()),
                    ],
                    egui::Stroke::new(1.4, col),
                ));
            }
        }
        1 => {
            // keyboard with keys
            let kb = egui::Rect::from_center_size(c, egui::vec2(28.0, 19.0));
            p.rect_stroke(kb, 3.5, s);
            for row in 0..2 {
                for col_i in 0..5 {
                    p.circle_filled(
                        egui::pos2(
                            kb.left() + 5.0 + col_i as f32 * 4.8,
                            kb.top() + 6.0 + row as f32 * 4.6,
                        ),
                        1.1,
                        col,
                    );
                }
            }
            p.add(egui::Shape::line(
                vec![
                    egui::pos2(kb.left() + 8.0, kb.bottom() - 4.0),
                    egui::pos2(kb.right() - 8.0, kb.bottom() - 4.0),
                ],
                egui::Stroke::new(1.6, col),
            ));
        }
        2 => {
            // battery + bolt
            let bat = egui::Rect::from_center_size(c, egui::vec2(28.0, 15.0));
            p.rect_stroke(bat, 3.0, s);
            p.rect_filled(
                egui::Rect::from_min_max(
                    egui::pos2(bat.right(), c.y - 3.0),
                    egui::pos2(bat.right() + 3.0, c.y + 3.0),
                ),
                1.0,
                col,
            );
            p.rect_filled(
                egui::Rect::from_min_max(
                    egui::pos2(bat.left() + 3.0, bat.top() + 3.0),
                    egui::pos2(bat.left() + 14.0, bat.bottom() - 3.0),
                ),
                1.5,
                egui::Color32::from_rgba_unmultiplied(col.r(), col.g(), col.b(), 90),
            );
            p.add(egui::Shape::line(
                vec![
                    egui::pos2(c.x + 2.0, c.y - 7.0),
                    egui::pos2(c.x - 4.0, c.y + 1.0),
                    egui::pos2(c.x + 1.0, c.y + 1.0),
                    egui::pos2(c.x - 2.0, c.y + 8.0),
                ],
                egui::Stroke::new(1.8, col),
            ));
        }
        _ => {
            // monitor + audio wave
            let scr = egui::Rect::from_center_size(
                egui::pos2(c.x - 2.0, c.y - 2.0),
                egui::vec2(28.0, 19.0),
            );
            p.rect_stroke(scr, 2.5, s);
            p.add(egui::Shape::line(
                vec![
                    egui::pos2(c.x - 2.0, scr.bottom() + 4.0),
                    egui::pos2(c.x - 2.0, scr.bottom()),
                ],
                s,
            ));
            p.add(egui::Shape::line(
                vec![
                    egui::pos2(c.x - 9.0, scr.bottom() + 5.0),
                    egui::pos2(c.x + 5.0, scr.bottom() + 5.0),
                ],
                s,
            ));
            for (r, a) in [(4.5f32, 0.6f32), (8.0, 0.35)] {
                let pts: Vec<egui::Pos2> = (0..=8)
                    .map(|i| {
                        let ang = -a + 2.0 * a * i as f32 / 8.0;
                        egui::pos2(c.x + 10.0 + r * ang.cos(), c.y + r * ang.sin())
                    })
                    .collect();
                p.add(egui::Shape::line(pts, egui::Stroke::new(1.5, col)));
            }
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
                    egui::Rect::from_min_size(egui::pos2(c.x + dx, c.y + dy), egui::vec2(4.0, 4.0)),
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
                vec![
                    egui::pos2(c.x - 6.0, c.y + 6.0),
                    egui::pos2(c.x + 6.0, c.y - 6.0),
                ],
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
    let h = 38.0;
    let (rect, resp) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), h), egui::Sense::click());
    let p = ui.painter_at(rect);
    let orange = accent;
    if selected {
        // Armoury Crate: subtle warm fill, thin orange rail, orange icon,
        // white label. The row itself is never solid orange.
        let a = accent;
        p.rect_filled(
            rect,
            2.0,
            egui::Color32::from_rgba_unmultiplied(
                a.r(),
                a.g(),
                a.b(),
                if light_theme() { 4 } else { 5 },
            ),
        );
        p.rect_filled(
            egui::Rect::from_min_size(
                egui::pos2(rect.left(), rect.top() + 4.0),
                egui::vec2(3.0, h - 8.0),
            ),
            1.0,
            orange,
        );
    } else if resp.hovered() {
        p.rect_filled(
            rect,
            2.0,
            if light_theme() {
                egui::Color32::from_rgb(224, 224, 229)
            } else {
                egui::Color32::from_rgb(32, 32, 35)
            },
        );
    }
    nav_icon(
        &p,
        egui::pos2(rect.left() + 22.0, rect.center().y),
        icon,
        if selected { orange } else { text_main() },
    );
    p.text(
        egui::pos2(rect.left() + 40.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(13.5),
        if light_theme() {
            text_main()
        } else {
            egui::Color32::from_rgb(186, 186, 192)
        },
    );
    resp.clicked()
}

fn section_label(ui: &mut egui::Ui, text: &str) {
    let a = accent();
    ui.horizontal(|ui| {
        let (r, _) = ui.allocate_exact_size(egui::vec2(18.0, 13.0), egui::Sense::hover());
        let p = ui.painter_at(r);
        for i in 0..3 {
            let x = r.left() + i as f32 * 5.5;
            p.add(egui::Shape::line(
                vec![egui::pos2(x, r.bottom()), egui::pos2(x + 5.0, r.top())],
                egui::Stroke::new(2.0, a),
            ));
        }
        ui.colored_label(text_main(), egui::RichText::new(text).size(14.5));
    });
}

/// Reference-matched section heading used on the Home dashboard.
fn section_label_p(p: &egui::Painter, pos: egui::Pos2, text: &str) {
    let a = accent();
    for i in 0..3 {
        let x = pos.x + i as f32 * 5.5;
        p.add(egui::Shape::line(
            vec![egui::pos2(x, pos.y + 12.0), egui::pos2(x + 5.0, pos.y)],
            egui::Stroke::new(2.0, a),
        ));
    }
    p.text(
        egui::pos2(pos.x + 24.0, pos.y + 6.5),
        egui::Align2::LEFT_CENTER,
        text,
        egui::FontId::proportional(14.5),
        text_main(),
    );
}

/// Elevated surface for Devices cards (the page background stays darker).
fn card_surface() -> egui::Color32 {
    if light_theme() {
        egui::Color32::from_rgb(255, 255, 255)
    } else {
        egui::Color32::from_rgb(36, 36, 36)
    }
}

/// Small (i) affordance. Hover shows the text; clicking pins a popup until
/// the icon is clicked again or Close is pressed.
fn info_icon(ui: &mut egui::Ui, text: &str) -> egui::Response {
    let (r, mut resp) = ui.allocate_exact_size(egui::vec2(16.0, 16.0), egui::Sense::click());
    let id = ui.make_persistent_id(("info-icon", text));
    let hovered = resp.hovered() || info_is_open(ui, id);
    let p = ui.painter_at(r);
    if hovered {
        p.circle_filled(
            r.center(),
            8.5,
            egui::Color32::from_rgba_unmultiplied(accent().r(), accent().g(), accent().b(), 24),
        );
    }
    p.circle_stroke(
        r.center(),
        7.5,
        egui::Stroke::new(1.0, if hovered { accent() } else { text_muted() }),
    );
    p.text(
        r.center(),
        egui::Align2::CENTER_CENTER,
        "i",
        egui::FontId::proportional(10.0),
        if hovered { accent() } else { text_muted() },
    );
    resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
    let open = info_is_open(ui, id);
    if !open {
        resp = resp.on_hover_text(text);
    }
    if resp.clicked() {
        toggle_info(ui, id);
    }
    info_popup(ui, id, r, text);
    resp
}

fn info_is_open(ui: &egui::Ui, id: egui::Id) -> bool {
    ui.memory(|m| {
        m.data
            .get_temp::<Option<egui::Id>>(egui::Id::new("tuf-info-open"))
            .flatten()
    }) == Some(id)
}

fn toggle_info(ui: &egui::Ui, id: egui::Id) {
    let key = egui::Id::new("tuf-info-open");
    let cur = ui.memory(|m| {
        m.data
            .get_temp::<Option<egui::Id>>(key)
            .flatten()
    });
    ui.memory_mut(|m| {
        if cur == Some(id) {
            m.data.remove_temp::<Option<egui::Id>>(key);
        } else {
            m.data.insert_temp(key, Some(id));
        }
    });
}

fn info_popup(ui: &egui::Ui, id: egui::Id, anchor: egui::Rect, text: &str) {
    if !info_is_open(ui, id) {
        return;
    }
    let screen = ui.ctx().screen_rect();
    let mut pos = anchor.left_bottom() + egui::vec2(0.0, 6.0);
    pos.x = pos.x.min(screen.right() - 390.0).max(screen.left() + 8.0);
    pos.y = pos.y.min(screen.bottom() - 80.0);
    let mut close = false;
    egui::Area::new(id.with("info-popup"))
        .order(egui::Order::Tooltip)
        .fixed_pos(pos)
        .show(ui.ctx(), |ui| {
            egui::Frame::popup(ui.style())
                .fill(card_surface())
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(64, 64, 70)))
                .inner_margin(12.0)
                .show(ui, |ui| {
                    let galley = ui.painter().layout(
                        text.to_string(),
                        egui::FontId::proportional(12.5),
                        text_main(),
                        340.0,
                    );
                    let (text_rect, _) = ui.allocate_exact_size(
                        galley.size(),
                        egui::Sense::hover(),
                    );
                    ui.painter().galley(text_rect.min, galley, text_main());
                    ui.add_space(4.0);
                    if ui.small_button("Close").clicked() {
                        close = true;
                    }
                });
        });
    if close {
        toggle_info(ui, id);
    }
}

/// Collapsible section presented as an elevated card with a right-side chevron.
fn collapsible_card<R>(
    ui: &mut egui::Ui,
    id_source: &str,
    title: &str,
    info: &str,
    default_open: bool,
    mut body: impl FnMut(&mut egui::Ui) -> R,
) {
    let id = ui.make_persistent_id(id_source);
    let mut open = ui
        .memory(|m| m.data.get_temp::<bool>(id))
        .unwrap_or(default_open);
    let mut toggle = false;
    egui::Frame::none()
        .fill(card_surface())
        .rounding(10.0)
        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(52, 52, 58)))
        .shadow(egui::epaint::Shadow {
            offset: egui::vec2(0.0, 3.0),
            blur: 10.0,
            spread: 0.0,
            color: egui::Color32::from_black_alpha(55),
        })
        .inner_margin(16.0)
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            let (hdr, resp) =
                ui.allocate_exact_size(egui::vec2(ui.available_width(), 28.0), egui::Sense::click());
            let p = ui.painter_at(hdr);
            p.text(
                egui::pos2(hdr.left(), hdr.center().y),
                egui::Align2::LEFT_CENTER,
                title,
                egui::FontId::proportional(14.5),
                text_main(),
            );
            let inf = egui::Rect::from_center_size(
                egui::pos2(hdr.right() - 44.0, hdr.center().y),
                egui::vec2(18.0, 18.0),
            );
            let info_id = id.with("info");
            let iresp = ui.interact(inf, info_id, egui::Sense::click());
            let info_open = info_is_open(ui, info_id);
            let info_hovered = iresp.hovered() || info_open;
            if info_hovered {
                p.circle_filled(
                    inf.center(),
                    8.5,
                    egui::Color32::from_rgba_unmultiplied(
                        accent().r(),
                        accent().g(),
                        accent().b(),
                        24,
                    ),
                );
            }
            p.circle_stroke(
                inf.center(),
                7.5,
                egui::Stroke::new(1.0, if info_hovered { accent() } else { text_muted() }),
            );
            p.text(
                inf.center(),
                egui::Align2::CENTER_CENTER,
                "i",
                egui::FontId::proportional(10.0),
                if info_hovered { accent() } else { text_muted() },
            );
            iresp.clone().on_hover_cursor(egui::CursorIcon::PointingHand);
            if !info_open {
                iresp.clone().on_hover_text(info);
            }
            if iresp.clicked() {
                toggle_info(ui, info_id);
            }
            info_popup(ui, info_id, inf, info);
            let c = egui::pos2(hdr.right() - 16.0, hdr.center().y);
            if open {
                p.add(egui::Shape::line(
                    vec![
                        egui::pos2(c.x - 5.0, c.y + 2.0),
                        egui::pos2(c.x, c.y - 3.0),
                        egui::pos2(c.x + 5.0, c.y + 2.0),
                    ],
                    egui::Stroke::new(1.4, text_muted()),
                ));
            } else {
                p.add(egui::Shape::line(
                    vec![
                        egui::pos2(c.x - 5.0, c.y - 2.0),
                        egui::pos2(c.x, c.y + 3.0),
                        egui::pos2(c.x + 5.0, c.y - 2.0),
                    ],
                    egui::Stroke::new(1.4, text_muted()),
                ));
            }
            if resp.clicked() && !info_hovered {
                toggle = true;
            }
            if open {
                ui.add_space(8.0);
                ui.separator();
                ui.add_space(10.0);
                body(ui);
            }
        });
    if toggle {
        open = !open;
    }
    ui.memory_mut(|m| m.data.insert_temp(id, open));
    ui.add_space(10.0);
}

/// Armoury-style ring gauge (CPU/GPU temps, fan RPM, battery).
#[allow(dead_code)]
fn ring_gauge(ui: &mut egui::Ui, fraction: f32, value: &str, caption: &str, color: egui::Color32) {
    let size = 92.0;
    let (rect, _) = ui.allocate_exact_size(egui::vec2(size, size + 18.0), egui::Sense::hover());
    let center = rect.center_top() + egui::vec2(0.0, size / 2.0);
    let radius = size / 2.0 - 7.0;
    let painter = ui.painter_at(rect);
    painter.circle_stroke(center, radius, egui::Stroke::new(7.0, meter_bg()));
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
        text_muted(),
    );
}

const TUF_ACCENT: egui::Color32 = egui::Color32::from_rgb(255, 107, 0);
static THEME_ACCENT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0x00FF_6B00);

/// Current theme accent, shared by all custom-painted widgets.
fn accent() -> egui::Color32 {
    let v = THEME_ACCENT.load(std::sync::atomic::Ordering::Relaxed);
    egui::Color32::from_rgb((v >> 16) as u8, (v >> 8) as u8, v as u8)
}

fn set_accent(c: egui::Color32) {
    let v = ((c.r() as u32) << 16) | ((c.g() as u32) << 8) | (c.b() as u32);
    THEME_ACCENT.store(v, std::sync::atomic::Ordering::Relaxed);
}

static LIGHT_THEME: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

fn light_theme() -> bool {
    LIGHT_THEME.load(std::sync::atomic::Ordering::Relaxed)
}

fn page_bg() -> egui::Color32 {
    if light_theme() {
        egui::Color32::from_rgb(242, 242, 245)
    } else {
        egui::Color32::from_rgb(22, 22, 22)
    }
}

fn sidebar_bg() -> egui::Color32 {
    if light_theme() {
        egui::Color32::from_rgb(233, 233, 238)
    } else {
        egui::Color32::from_rgb(23, 23, 24)
    }
}

fn card_bg() -> egui::Color32 {
    if light_theme() {
        egui::Color32::from_rgb(255, 255, 255)
    } else {
        egui::Color32::from_rgb(24, 24, 27)
    }
}

fn card_bg2() -> egui::Color32 {
    if light_theme() {
        egui::Color32::from_rgb(247, 247, 250)
    } else {
        egui::Color32::from_rgb(22, 22, 25)
    }
}

fn text_main() -> egui::Color32 {
    if light_theme() {
        egui::Color32::from_rgb(27, 27, 31)
    } else {
        egui::Color32::from_rgb(255, 255, 255)
    }
}

fn text_muted() -> egui::Color32 {
    if light_theme() {
        egui::Color32::from_rgb(90, 90, 98)
    } else {
        egui::Color32::from_rgb(160, 160, 160)
    }
}

fn meter_bg() -> egui::Color32 {
    if light_theme() {
        egui::Color32::from_rgb(216, 216, 222)
    } else {
        egui::Color32::from_rgb(44, 46, 54)
    }
}

fn rog_family() -> egui::FontFamily {
    egui::FontFamily::Name("rog".into())
}

fn rog_font(size: f32) -> egui::FontId {
    egui::FontId::new(size, rog_family())
}

/// Small rounded status chip used in the Home hero.
#[allow(dead_code)]
fn status_chip(ui: &mut egui::Ui, text: &str, color: egui::Color32) {
    let galley =
        ui.painter()
            .layout_no_wrap(text.to_string(), egui::FontId::proportional(12.0), color);
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

enum UsageRow {
    Meter(&'static str, String, f32),
    Meta(&'static str, String),
}

/// Paint one System Usage block exactly like Armoury Crate: large sans title,
/// grey slash watermark, label/value rows and a quiet fill of placeholder rows.
fn draw_usage_block(p: &egui::Painter, rect: egui::Rect, title: &str, rows: &[UsageRow]) {
    let white = text_main();
    let muted = text_muted();
    let font = egui::FontId::proportional(12.5);
    p.text(
        rect.left_top(),
        egui::Align2::LEFT_TOP,
        title,
        egui::FontId::proportional(31.0),
        white,
    );
    // decorative grey slashes at the top-right of the heading
    for i in 0..3 {
        let x = rect.right() - 30.0 + i as f32 * 6.0;
        p.add(egui::Shape::line(
            vec![
                egui::pos2(x, rect.top() + 20.0),
                egui::pos2(x + 7.0, rect.top() + 6.0),
            ],
            egui::Stroke::new(2.5, egui::Color32::from_rgb(62, 64, 72)),
        ));
    }

    let mut y = rect.top() + 39.0;
    let bar_bg = meter_bg();
    let orange = accent();
    for row in rows {
        match row {
            UsageRow::Meter(label, value, frac) => {
                if y + 44.0 > rect.bottom() {
                    break;
                }
                p.text(
                    egui::pos2(rect.left(), y + 8.0),
                    egui::Align2::LEFT_CENTER,
                    *label,
                    font.clone(),
                    muted,
                );
                p.text(
                    egui::pos2(rect.right(), y + 8.0),
                    egui::Align2::RIGHT_CENTER,
                    value,
                    font.clone(),
                    muted,
                );
                let bar = egui::Rect::from_min_size(
                    egui::pos2(rect.left(), y + 30.0),
                    egui::vec2(rect.width(), 2.0),
                );
                p.rect_filled(bar, 1.0, bar_bg);
                let f = frac.clamp(0.0, 1.0);
                if f > 0.0 {
                    p.rect_filled(
                        egui::Rect::from_min_size(bar.min, egui::vec2(bar.width() * f, 2.0)),
                        1.0,
                        orange,
                    );
                }
                y += 46.0;
            }
            UsageRow::Meta(label, value) => {
                if y + 22.0 > rect.bottom() {
                    break;
                }
                p.text(
                    egui::pos2(rect.left(), y + 8.0),
                    egui::Align2::LEFT_CENTER,
                    *label,
                    font.clone(),
                    muted,
                );
                p.text(
                    egui::pos2(rect.right(), y + 8.0),
                    egui::Align2::RIGHT_CENTER,
                    value,
                    font.clone(),
                    muted,
                );
                p.add(egui::Shape::line(
                    vec![
                        egui::pos2(rect.left(), y + 23.0),
                        egui::pos2(rect.right(), y + 23.0),
                    ],
                    egui::Stroke::new(
                        1.0,
                        egui::Color32::from_rgba_unmultiplied(255, 255, 255, 10),
                    ),
                ));
                y += 25.0;
            }
        }
    }
    // Armoury keeps the blocks visually anchored with dim "--" rows.
    while y + 23.0 <= rect.bottom() {
        p.text(
            egui::pos2(rect.left(), y + 8.0),
            egui::Align2::LEFT_CENTER,
            "--",
            font.clone(),
            egui::Color32::from_rgb(92, 94, 102),
        );
        p.text(
            egui::pos2(rect.right(), y + 8.0),
            egui::Align2::RIGHT_CENTER,
            "--",
            font.clone(),
            egui::Color32::from_rgb(92, 94, 102),
        );
        p.add(egui::Shape::line(
            vec![
                egui::pos2(rect.left(), y + 23.0),
                egui::pos2(rect.right(), y + 23.0),
            ],
            egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(255, 255, 255, 7)),
        ));
        y += 25.0;
    }
}

/// Dashed rounded outline used by Armoury's "add device" placeholder.
fn dashed_rounded_rect(p: &egui::Painter, rect: egui::Rect, r: f32, stroke: egui::Stroke) {
    let mut pts = Vec::with_capacity(64);
    let arc = |cx: f32, cy: f32, from: f32, to: f32, pts: &mut Vec<egui::Pos2>| {
        for i in 0..=5 {
            let a = from + (to - from) * i as f32 / 5.0;
            pts.push(egui::pos2(cx + r * a.cos(), cy + r * a.sin()));
        }
    };
    use std::f32::consts::{FRAC_PI_2, PI};
    arc(
        rect.left() + r,
        rect.top() + r,
        PI,
        PI + FRAC_PI_2,
        &mut pts,
    );
    arc(rect.right() - r, rect.top() + r, -FRAC_PI_2, 0.0, &mut pts);
    arc(
        rect.right() - r,
        rect.bottom() - r,
        0.0,
        FRAC_PI_2,
        &mut pts,
    );
    arc(rect.left() + r, rect.bottom() - r, FRAC_PI_2, PI, &mut pts);
    pts.push(pts[0]);
    for shape in egui::Shape::dashed_line(&pts, stroke, 5.0, 5.0) {
        p.add(shape);
    }
}

/// Windows Armoury Crate header cluster: ROG mark, keyboard, audio, battery.
fn header_icons(ui: &mut egui::Ui, battery: Option<u32>, profile: Option<&str>) {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(250.0, 32.0), egui::Sense::hover());
    let p = ui.painter_at(rect);
    let muted = egui::Color32::from_rgb(168, 168, 178);
    // battery tile at the far right
    let tile = egui::Rect::from_min_size(
        egui::pos2(rect.right() - 38.0, rect.top() + 1.0),
        egui::vec2(38.0, 30.0),
    );
    p.rect_filled(
        tile,
        5.0,
        egui::Color32::from_rgba_unmultiplied(255, 255, 255, 12),
    );
    nav_icon(&p, tile.center(), 6, muted);
    // speaker
    let sp = egui::pos2(rect.right() - 121.0, rect.center().y);
    p.add(egui::Shape::convex_polygon(
        vec![
            egui::pos2(sp.x - 8.0, sp.y - 4.0),
            egui::pos2(sp.x - 3.0, sp.y - 4.0),
            egui::pos2(sp.x + 3.0, sp.y - 9.0),
            egui::pos2(sp.x + 3.0, sp.y + 9.0),
            egui::pos2(sp.x - 3.0, sp.y + 4.0),
            egui::pos2(sp.x - 8.0, sp.y + 4.0),
        ],
        muted,
        egui::Stroke::NONE,
    ));
    for (r, a) in [(7.0f32, 0.55f32), (11.0, 0.30)] {
        let pts: Vec<egui::Pos2> = (0..=8)
            .map(|i| {
                let ang = -a + 2.0 * a * i as f32 / 8.0;
                egui::pos2(sp.x + r * ang.cos(), sp.y + r * ang.sin())
            })
            .collect();
        p.add(egui::Shape::line(pts, egui::Stroke::new(1.4, muted)));
    }
    // keyboard
    let kb = egui::Rect::from_center_size(
        egui::pos2(rect.right() - 177.0, rect.center().y),
        egui::vec2(24.0, 14.0),
    );
    p.rect_stroke(kb, 2.0, egui::Stroke::new(1.4, muted));
    for row in 0..2 {
        for col in 0..5 {
            let c = egui::pos2(
                kb.left() + 4.0 + col as f32 * 4.0,
                kb.top() + 4.5 + row as f32 * 4.5,
            );
            p.circle_filled(c, 0.9, muted);
        }
    }
    // ROG diagonal mark
    for i in 0..3 {
        let x = rect.right() - 241.0 + i as f32 * 6.5;
        p.add(egui::Shape::line(
            vec![
                egui::pos2(x, rect.center().y + 10.0),
                egui::pos2(x + 8.0, rect.center().y - 9.0),
            ],
            egui::Stroke::new(3.0, muted),
        ));
    }
    let tip = match (battery, profile) {
        (Some(b), Some(p)) => format!("Battery {b}% · profile {p}"),
        (Some(b), None) => format!("Battery {b}%"),
        (None, Some(p)) => format!("profile {p}"),
        (None, None) => "System status".into(),
    };
    resp.on_hover_text(tip);
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
    ticks: u64,
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
            let ticks = std::fs::read_to_string(e.path().join("stat"))
                .ok()
                .and_then(|st| {
                    st.rsplit_once(')').and_then(|(_, rest)| {
                        let fields: Vec<&str> = rest.split_whitespace().collect();
                        match (
                            fields.get(11).and_then(|x| x.parse::<u64>().ok()),
                            fields.get(12).and_then(|x| x.parse::<u64>().ok()),
                        ) {
                            (Some(u), Some(s)) => Some(u + s),
                            _ => None,
                        }
                    })
                })
                .unwrap_or(0);
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
                    s.lines().find(|l| l.starts_with("Uid:")).and_then(|l| {
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
                    ticks,
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
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        for (value, label) in items.iter() {
            let selected = *current == *value;
            let galley = ui.painter().layout_no_wrap(
                (*label).to_string(),
                egui::FontId::proportional(14.0),
                egui::Color32::WHITE,
            );
            let w = galley.size().x + 34.0;
            let (rect, resp) = ui.allocate_exact_size(egui::vec2(w, 38.0), egui::Sense::click());
            let p = ui.painter_at(rect);
            if resp.hovered() && !selected {
                p.rect_filled(
                    rect,
                    3.0,
                    egui::Color32::from_rgba_unmultiplied(255, 255, 255, 8),
                );
            }
            p.text(
                egui::pos2(rect.center().x, rect.center().y - 1.0),
                egui::Align2::CENTER_CENTER,
                *label,
                egui::FontId::proportional(14.0),
                if selected { text_main() } else { text_muted() },
            );
            if selected {
                p.rect_filled(
                    egui::Rect::from_min_size(
                        egui::pos2(rect.left() + 10.0, rect.bottom() - 3.0),
                        egui::vec2(rect.width() - 20.0, 2.5),
                    ),
                    1.0,
                    accent,
                );
            }
            if resp.clicked() {
                *current = *value;
            }
            ui.add_space(4.0);
        }
    });
    ui.add_space(2.0);
    ui.separator();
    ui.add_space(6.0);
}

fn detect_display_output() -> Option<String> {
    let out = std::process::Command::new("xrandr")
        .arg("--query")
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    for line in text.lines() {
        if let Some((name, _)) = line.split_once(" connected") {
            let name = name.trim();
            if !name.is_empty()
                && name != "screen"
                && !name.contains(' ')
                && !line.contains("disconnected")
            {
                return Some(name.to_string());
            }
        }
    }
    None
}

/// Current mode and supported refresh rates for an X11 output.
fn display_mode_rates(output: &str) -> Option<(String, Vec<String>)> {
    let out = std::process::Command::new("xrandr")
        .arg("--query")
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let mut lines = text.lines().peekable();
    let mut current_mode = None;
    let mut in_output = false;
    let mut rates: Vec<String> = Vec::new();
    while let Some(line) = lines.next() {
        if line.starts_with(output) {
            in_output = true;
            for tok in line.split_whitespace() {
                if tok.contains('x') && (tok.contains('+') || tok.contains('*')) {
                    current_mode = Some(tok.split('+').next()?.to_string());
                    break;
                }
            }
            // the first mode line after the output header is the current mode
            if let Some(next) = lines.peek() {
                if next.starts_with(' ') {
                    let mode_line = lines.next()?;
                    let mut toks = mode_line.split_whitespace();
                    let mode = toks.next()?.to_string();
                    if current_mode.is_none() {
                        current_mode = Some(mode);
                    }
                    for tok in toks {
                        let rate = tok.trim_end_matches(['*', '+']);
                        if rate.parse::<f32>().is_ok() && !rates.contains(&rate.to_string()) {
                            rates.push(rate.to_string());
                        }
                    }
                }
            }
            break;
        }
    }
    if !in_output {
        return None;
    }
    Some((current_mode.unwrap_or_else(|| "--".into()), rates))
}

fn apply_refresh_rate(output: &str, mode: &str, rate: &str) -> Result<(), String> {
    let out = std::process::Command::new("xrandr")
        .args(["--output", output, "--mode", mode, "--rate", rate])
        .output()
        .map_err(|e| format!("xrandr unavailable: {e}"))?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr)
            .lines()
            .next()
            .unwrap_or("xrandr rejected the mode")
            .to_string())
    }
}

fn display_refresh_rate(output: &str) -> Option<String> {
    let out = std::process::Command::new("xrandr")
        .arg("--current")
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    for line in text.lines() {
        if line.starts_with(output) {
            for tok in line.split_whitespace() {
                if tok.ends_with('*') {
                    return Some(tok.trim_end_matches('*').to_string());
                }
            }
        }
    }
    None
}

/// GameVisual profiles are implemented as reversible X11 gamma curves.
const DISPLAY_MODES: [(&str, &str, [f32; 3]); 8] = [
    ("Default", "Balanced desktop colour", [1.00, 1.00, 1.00]),
    ("Racing", "Cooler, higher contrast", [1.06, 1.00, 0.96]),
    ("Scenery", "Warmer greens and sky", [1.03, 1.01, 0.92]),
    ("Cinema", "Warm, lower blue", [1.00, 0.97, 0.88]),
    ("RTS/RPG", "Bright detail for strategy", [1.04, 1.02, 1.00]),
    ("FPS", "Cool shadows, crisp highlights", [0.95, 1.00, 1.07]),
    ("sRGB", "Neutral colour accuracy", [1.00, 1.00, 1.00]),
    ("Night", "Low-blue evening profile", [1.00, 0.92, 0.78]),
];

fn draw_display_scene(p: &egui::Painter, rect: egui::Rect, gamma: [f32; 3]) {
    let tint = |c: (f32, f32, f32)| {
        egui::Color32::from_rgb(
            (c.0 * gamma[0]).clamp(0.0, 255.0) as u8,
            (c.1 * gamma[1]).clamp(0.0, 255.0) as u8,
            (c.2 * gamma[2]).clamp(0.0, 255.0) as u8,
        )
    };
    let horizon = rect.top() + rect.height() * 0.58;
    for i in 0..14 {
        let f = i as f32 / 13.0;
        let y0 = rect.top() + (horizon - rect.top()) * i as f32 / 14.0;
        let y1 = rect.top() + (horizon - rect.top()) * (i + 1) as f32 / 14.0;
        let top = tint((24.0 + 36.0 * f, 30.0 + 48.0 * f, 54.0 + 58.0 * f));
        p.rect_filled(
            egui::Rect::from_min_max(
                egui::pos2(rect.left(), y0),
                egui::pos2(rect.right(), y1 + 1.0),
            ),
            0.0,
            top,
        );
    }
    p.circle_filled(
        egui::pos2(
            rect.left() + rect.width() * 0.72,
            rect.top() + rect.height() * 0.30,
        ),
        rect.height() * 0.10,
        tint((245.0, 206.0, 150.0)),
    );
    p.rect_filled(
        egui::Rect::from_min_max(egui::pos2(rect.left(), horizon), rect.max),
        0.0,
        tint((30.0, 28.0, 27.0)),
    );
    // perspective road
    p.add(egui::Shape::convex_polygon(
        vec![
            egui::pos2(rect.left() + rect.width() * 0.36, horizon),
            egui::pos2(rect.left() + rect.width() * 0.48, horizon),
            egui::pos2(rect.left() + rect.width() * 0.82, rect.bottom()),
            egui::pos2(rect.left() + rect.width() * 0.12, rect.bottom()),
        ],
        tint((46.0, 44.0, 46.0)),
        egui::Stroke::NONE,
    ));
    for i in 0..4 {
        let f = i as f32 / 4.0;
        let y = horizon + (rect.bottom() - horizon) * (0.15 + f * 0.7);
        let half = rect.width() * (0.03 + f * 0.05);
        let cx = rect.left() + rect.width() * 0.47;
        p.rect_filled(
            egui::Rect::from_min_size(
                egui::pos2(cx - half * 0.15, y),
                egui::vec2(half * 0.3, 2.0 + f * 2.0),
            ),
            1.0,
            tint((200.0, 190.0, 160.0)),
        );
    }
}

fn apply_display_gamma(output: &str, rgb: [f32; 3]) -> Result<(), String> {
    let gamma = format!("{:.2}:{:.2}:{:.2}", rgb[0], rgb[1], rgb[2]);
    let out = std::process::Command::new("xrandr")
        .args(["--output", output, "--gamma", &gamma])
        .output()
        .map_err(|e| format!("xrandr unavailable: {e}"))?;
    if out.status.success() {
        Ok(())
    } else {
        let err = String::from_utf8_lossy(&out.stderr);
        Err(err
            .lines()
            .next()
            .unwrap_or("xrandr rejected the gamma value")
            .to_string())
    }
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
    LIGHT_THEME.store(theme == Theme::Light, std::sync::atomic::Ordering::Relaxed);
    let mut v = if theme == Theme::Light {
        egui::Visuals::light()
    } else {
        egui::Visuals::dark()
    };
    if theme != Theme::Light {
        v.panel_fill = page_bg();
        v.window_fill = egui::Color32::from_rgb(22, 24, 28);
        v.extreme_bg_color = sidebar_bg();
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

fn set_xfce_wallpaper_style(style: u32) -> Result<usize, String> {
    let out = std::process::Command::new("xfconf-query")
        .args(["-c", "xfce4-desktop", "-l"])
        .output()
        .map_err(|e| format!("xfconf-query not available: {e}"))?;
    if !out.status.success() {
        return Err("cannot list XFCE desktop properties".into());
    }
    let props: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| l.ends_with("/image-style"))
        .map(|l| l.to_string())
        .collect();
    if props.is_empty() {
        return Err("no XFCE image-style property found".into());
    }
    let mut count = 0;
    for prop in props {
        let ok = std::process::Command::new("xfconf-query")
            .args(["-c", "xfce4-desktop", "-p", &prop, "-s", &style.to_string()])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if ok {
            count += 1;
        }
    }
    if count == 0 {
        Err("XFCE rejected the wallpaper style".into())
    } else {
        Ok(count)
    }
}

fn read_default_volume() -> Option<(f32, bool)> {
    let out = std::process::Command::new("wpctl")
        .args(["get-volume", "@DEFAULT_AUDIO_SINK@"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let vol = text
        .split_whitespace()
        .nth(1)
        .and_then(|v| v.parse::<f32>().ok())?;
    Some((vol, text.contains("MUTED")))
}

fn default_sink_name(status: &str) -> Option<String> {
    let mut in_sinks = false;
    for line in status.lines() {
        if line.contains("Sinks:") {
            in_sinks = true;
            continue;
        }
        if in_sinks {
            if line.contains("Sources:") {
                break;
            }
            if line.contains('*') {
                if let Some(pos) = line.find(". ") {
                    let rest = &line[pos + 2..];
                    return Some(rest.split(" [").next().unwrap_or(rest).trim().to_string());
                }
            }
        }
    }
    None
}

fn apply_default_volume(percent: f32) {
    let _ = std::process::Command::new("wpctl")
        .args([
            "set-volume",
            "@DEFAULT_AUDIO_SINK@",
            &format!("{:.0}%", percent.clamp(0.0, 100.0)),
        ])
        .status();
}

fn apply_default_mute(on: bool) {
    let _ = std::process::Command::new("wpctl")
        .args([
            "set-mute",
            "@DEFAULT_AUDIO_SINK@",
            if on { "1" } else { "0" },
        ])
        .status();
}

struct TufApp {
    backend: Box<dyn Backend>,
    telemetry: Option<Telemetry>,
    telemetry_error: Option<String>,
    last_poll: std::time::Instant,
    writes_enabled: bool,
    status_msg: Option<(String, bool)>, // (message, is_error)
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
    hero_model: String,
    hero_image: Option<egui::TextureHandle>,
    hero_line: Option<egui::TextureHandle>,
    anim_cpu: f32,
    anim_gpu: f32,
    anim_fan: f32,
    anim_batt: f32,
    last_anim: std::time::Instant,
    last_tab: Tab,
    tab_changed_at: std::time::Instant,
    devices_sub: DeviceTab,
    aura_sub: AuraTab,
    settings_sub: SettingsTab,
    theme: Theme,
    theme_previews: Vec<(Theme, Option<egui::TextureHandle>)>,
    launch_page: String,
    free_confirm: bool,
    free_selected: std::collections::BTreeSet<i32>,
    free_expanded: std::collections::BTreeSet<String>,
    res_expanded: std::collections::BTreeSet<String>,
    wallpapers: Vec<(String, Option<egui::TextureHandle>)>,
    wallpaper_idx: usize,
    device_line_art: bool,
    update_text: String,
    audio_volume: f32,
    audio_muted: bool,
    audio_loaded: bool,
    audio_last_set: std::time::Instant,
    proc_cpu: std::collections::HashMap<i32, f32>,
    proc_last: std::collections::HashMap<i32, u64>,
    proc_last_at: std::time::Instant,
    status_at: std::time::Instant,
    game_rules: Vec<games::GameRule>,
    game_input: String,
    game_profile: String,
    active_game_rule: Option<String>,
    restore_profile: Option<String>,
    last_game_check: std::time::Instant,
    cpu_util: Option<u32>,
    cpu_mhz: Option<u64>,
    cpu_mhz_hist: std::collections::VecDeque<u64>,
    fan_hist: std::collections::VecDeque<(u32, u32)>,
    native_ppp: f32,
    ui_scale: f32,
    power_profile_switch: bool,
    ac_profile: String,
    battery_profile: String,
    last_ac: Option<bool>,
    prev_stat: Option<(u64, u64)>,
    last_page: (Tab, DeviceTab, AuraTab, SettingsTab),
    scroll_reset: bool,
    display_output: Option<String>,
    display_mode: usize,
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
        // Armoury renders at 1:1 on this 1920x1080 panel. Some X11 sessions
        // report a non-1.0 native scale (e.g. 1.333); neutralise it so the
        // reference layout keeps its real pixel geometry. The user can opt
        // into a larger UI scale from Settings.
        let native_ppp = (ctx.pixels_per_point() / ctx.zoom_factor()).max(0.5);
        let ui_scale = saved
            .get("ui_scale")
            .and_then(|v| v.as_f64())
            .map(|v| v as f32)
            .unwrap_or(1.0)
            .clamp(0.75, 2.0);
        let power_profile_switch = saved
            .get("power_profile_switch")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let ac_profile = saved
            .get("ac_profile")
            .and_then(|v| v.as_str())
            .unwrap_or("performance")
            .to_string();
        let battery_profile = saved
            .get("battery_profile")
            .and_then(|v| v.as_str())
            .unwrap_or("quiet")
            .to_string();
        ctx.set_zoom_factor(ui_scale / native_ppp);
        let initial_tab = match launch_page.as_str() {
            "Devices" => Tab::Devices,
            "Lighting & Aura Sync" => Tab::AuraSync,
            "Scenario Profiles" => Tab::ScenarioProfiles,
            "Settings" => Tab::Settings,
            _ => Tab::Home,
        };
        apply_theme_visuals(ctx, theme);
        let hero_model = assets::detect_model();
        let hero_image = assets::hero_texture(ctx, &hero_model, false);
        let hero_line = assets::hero_texture(ctx, &hero_model, true);
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
        let wallpapers: Vec<(String, Option<egui::TextureHandle>)> = assets::wallpaper_paths()
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
            hero_model,
            hero_image,
            hero_line,
            theme_previews,
            anim_cpu: 0.0,
            anim_gpu: 0.0,
            anim_fan: 0.0,
            anim_batt: 0.0,
            last_anim: std::time::Instant::now(),
            last_tab: initial_tab,
            tab_changed_at: std::time::Instant::now() - std::time::Duration::from_secs(1),
            devices_sub: DeviceTab::SystemConfig,
            aura_sub: AuraTab::Effects,
            settings_sub: SettingsTab::General,
            theme,
            launch_page,
            free_confirm: false,
            free_selected: std::collections::BTreeSet::new(),
            free_expanded: std::collections::BTreeSet::new(),
            res_expanded: std::collections::BTreeSet::new(),
            wallpapers,
            wallpaper_idx: 0,
            device_line_art,
            update_text: String::new(),
            audio_volume: 50.0,
            audio_muted: false,
            audio_loaded: false,
            audio_last_set: std::time::Instant::now(),
            proc_cpu: std::collections::HashMap::new(),
            proc_last: std::collections::HashMap::new(),
            proc_last_at: std::time::Instant::now(),
            status_at: std::time::Instant::now() - std::time::Duration::from_secs(10),
            game_rules: games::load_all(),
            game_input: String::new(),
            game_profile: "performance".to_string(),
            active_game_rule: None,
            restore_profile: None,
            last_game_check: std::time::Instant::now(),
            cpu_util: None,
            cpu_mhz: None,
            cpu_mhz_hist: std::collections::VecDeque::new(),
            fan_hist: std::collections::VecDeque::new(),
            native_ppp,
            ui_scale,
            power_profile_switch,
            ac_profile,
            battery_profile,
            last_ac: None,
            prev_stat: None,
            last_page: (
                initial_tab,
                DeviceTab::SystemConfig,
                AuraTab::Effects,
                SettingsTab::General,
            ),
            scroll_reset: false,
            display_output: detect_display_output(),
            display_mode: 0,
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
                            let busy = dt.saturating_sub(idle.saturating_sub(pi));
                            let measured = ((busy as f64 / dt as f64) * 100.0).round() as u32;
                            // EMA takes the jitter out of the 0.5 s delta without
                            // lagging behind real load changes.
                            self.cpu_util = Some(match self.cpu_util {
                                Some(prev) => (prev * 2 + measured) / 3,
                                None => measured,
                            });
                        }
                    }
                    self.prev_stat = Some((total, idle));
                }
            }
        }
        // Highest core clock is what users expect from "CPU frequency"; cpu0
        // alone idles while other cores boost.
        let mut max_khz = 0u64;
        if let Ok(rd) = std::fs::read_dir("/sys/devices/system/cpu") {
            for e in rd.flatten() {
                let name = e.file_name().to_string_lossy().into_owned();
                if !name.starts_with("cpu") || name == "cpuidle" {
                    continue;
                }
                if let Ok(s) = std::fs::read_to_string(e.path().join("cpufreq/scaling_cur_freq")) {
                    if let Ok(khz) = s.trim().parse::<u64>() {
                        max_khz = max_khz.max(khz);
                    }
                }
            }
        }
        if max_khz > 0 {
            let mhz = max_khz / 1000;
            self.cpu_mhz_hist.push_back(mhz);
            while self.cpu_mhz_hist.len() > 6 {
                self.cpu_mhz_hist.pop_front();
            }
            let mut samples: Vec<u64> = self.cpu_mhz_hist.iter().copied().collect();
            samples.sort_unstable();
            // 3-second median: stable to read, still follows real load.
            self.cpu_mhz = Some(samples[samples.len() / 2]);
        }
        match self.backend.telemetry() {
            Ok(t) => {
                self.fan_hist
                    .push_back((t.cpu_fan_rpm.unwrap_or(0), t.gpu_fan_rpm.unwrap_or(0)));
                while self.fan_hist.len() > 120 {
                    self.fan_hist.pop_front();
                }
                let ac = t.battery.as_ref().map(|b| b.ac_online);
                self.telemetry = Some(t);
                self.telemetry_error = None;
                if let Some(ac) = ac {
                    if let Some(prev) = self.last_ac {
                        if prev != ac && self.power_profile_switch {
                            let want = if ac {
                                self.ac_profile.clone()
                            } else {
                                self.battery_profile.clone()
                            };
                            if let Some(profile) = Profile::from_kernel_str(&want) {
                                match self.backend.set_profile(profile, true) {
                                    Ok(()) => self.report(
                                        format!(
                                            "{} power → {} profile",
                                            if ac { "AC" } else { "Battery" },
                                            profile.label()
                                        ),
                                        false,
                                    ),
                                    Err(e) => self.report(
                                        format!(
                                            "power profile switch failed: {}",
                                            friendly_error(&e)
                                        ),
                                        true,
                                    ),
                                }
                            }
                        }
                    }
                    self.last_ac = Some(ac);
                }
            }
            Err(e) => {
                self.telemetry_error = Some(e.to_string());
            }
        }
    }

    fn report(&mut self, msg: String, is_error: bool) {
        self.status_msg = Some((msg, is_error));
        self.status_at = std::time::Instant::now();
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
        self.last_soft_apply = std::time::Instant::now() - std::time::Duration::from_secs(1);
        self.last_soft_rgb = [0, 0, 0];
        self.soft_note.clear();
        self.report(
            "Smart effect started — colour follows CPU temperature".to_string(),
            false,
        );
        self.tick_soft_effect();
    }

    /// Adaptive Color: colour follows the average of the screen (X11).
    fn start_adaptive(&mut self) {
        self.stop_soft_effect();
        self.soft_effect = Some(music::ADAPTIVE_EFFECT_ID);
        self.last_soft_apply = std::time::Instant::now() - std::time::Duration::from_secs(1);
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
                self.soft_note = format!(
                    "level {}",
                    if beat { 1000 } else { (level * 1000.0) as u32 }
                );
                Self::music_color(self.rgb_hue, self.rgb_sat, if beat { 1.0 } else { level })
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
                        "Adaptive Color needs an X11 display and ImageMagick (import)".to_string(),
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
                self.report(
                    format!("Software frame failed: {}", friendly_error(&e)),
                    true,
                );
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
        HalError::Unsupported => "not exposed by this kernel/daemon on this model".to_string(),
        HalError::Safety(se) => format!("safety check rejected this: {se}"),
        HalError::Daemon(msg) if msg.contains("Permission denied") => {
            "control daemon access denied — log out/in once, or re-run sudo scripts/install.sh"
                .to_string()
        }
        HalError::Daemon(msg) if msg.contains("unreachable") => {
            "control daemon not running — re-run sudo scripts/install.sh".to_string()
        }
        other => format!("{other}"),
    }
}

impl eframe::App for TufApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Enforce the requested interface scale against the actual native
        // scale. Some X11 sessions report 1.333×; without this the reference
        // 1:1 layout would render 33 % too large and clip.
        let native = (ctx.pixels_per_point() / ctx.zoom_factor()).max(0.5);
        let desired_zoom = (self.ui_scale / native).clamp(0.4, 2.0);
        if (ctx.zoom_factor() - desired_zoom).abs() > 0.002 {
            ctx.set_zoom_factor(desired_zoom);
        }
        self.native_ppp = native;

        const SPLASH_SECS: f32 = 2.2;
        let splash_t = self.splash_start.elapsed().as_secs_f32();
        if std::env::var("TUF_CRATE_SKIP_SPLASH").is_err() && splash_t < SPLASH_SECS {
            self.draw_splash(ctx, splash_t, SPLASH_SECS);
            ctx.request_repaint_after(std::time::Duration::from_millis(16));
            return;
        }
        if self.last_poll.elapsed() >= std::time::Duration::from_millis(500) {
            self.poll();
            self.last_poll = std::time::Instant::now();
        }
        // request repaint so the polling continues; the Lighting tab animates
        // and software effects need a steady frame clock on every tab.
        if matches!(self.tab, Tab::AuraSync | Tab::Home) || self.soft_effect.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
        } else {
            ctx.request_repaint_after(std::time::Duration::from_millis(500));
        }
        self.tick_soft_effect();
        self.check_game_rules();
        self.tick_animations();

        if self.status_msg.is_some() || self.telemetry_error.is_some() {
            let page_key = (self.tab, self.devices_sub, self.aura_sub, self.settings_sub);
            if page_key != self.last_page {
                self.last_page = page_key;
                self.scroll_reset = true;
            }

            egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
                if let Some((msg, err)) = &self.status_msg {
                    ui.horizontal(|ui| {
                        if *err {
                            ui.colored_label(
                                egui::Color32::from_rgb(255, 68, 68),
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
                        egui::Color32::from_rgb(255, 68, 68),
                        format!("telemetry: {err}"),
                    );
                }
            });
        }

        egui::SidePanel::left("nav")
            .exact_width(240.0)
            .frame(egui::Frame::none().fill(sidebar_bg()))
            .show(ctx, |ui| {
                ui.add_space(18.0);
                ui.horizontal(|ui| {
                    let (r, _) =
                        ui.allocate_exact_size(egui::vec2(26.0, 24.0), egui::Sense::hover());
                    let p = ui.painter_at(r);
                    for i in 0..3 {
                        p.add(egui::Shape::line(
                            vec![
                                egui::pos2(r.left() + 3.0, r.top() + 7.0 + i as f32 * 5.0),
                                egui::pos2(r.right() - 3.0, r.top() + 7.0 + i as f32 * 5.0),
                            ],
                            egui::Stroke::new(1.5, egui::Color32::from_rgb(190, 190, 196)),
                        ));
                    }
                });
                ui.add_space(35.0);

                let group = |ui: &mut egui::Ui, t: &str| {
                    ui.add_space(24.0);
                    ui.colored_label(text_muted(), egui::RichText::new(t).size(12.0));
                    ui.add_space(3.0);
                };

                let accent = self.theme.accent();
                if nav_item_c(ui, 0, "Home", self.tab == Tab::Home, accent) {
                    self.tab = Tab::Home;
                }
                if nav_item_c(ui, 1, "Devices", self.tab == Tab::Devices, accent) {
                    self.tab = Tab::Devices;
                }
                group(ui, "Playground");
                if nav_item_c(ui, 2, "Aura Sync", self.tab == Tab::AuraSync, accent) {
                    self.tab = Tab::AuraSync;
                }
                if nav_item_c(
                    ui,
                    7,
                    "Aura Wallpaper",
                    self.tab == Tab::AuraWallpaper,
                    accent,
                ) {
                    self.tab = Tab::AuraWallpaper;
                }
                group(ui, "Assistant");
                if nav_item_c(
                    ui,
                    3,
                    "Scenario Profiles",
                    self.tab == Tab::ScenarioProfiles,
                    accent,
                ) {
                    self.tab = Tab::ScenarioProfiles;
                }
                if nav_item_c(ui, 7, "GameVisual", self.tab == Tab::GameVisual, accent) {
                    self.tab = Tab::GameVisual;
                }

                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    ui.add_space(6.0);
                    if nav_item_c(
                        ui,
                        8,
                        "Settings",
                        self.tab == Tab::Settings,
                        self.theme.accent(),
                    ) {
                        self.tab = Tab::Settings;
                    }
                    if nav_item_c(
                        ui,
                        11,
                        "Feature Library",
                        self.tab == Tab::FeatureLibrary,
                        self.theme.accent(),
                    ) {
                        self.tab = Tab::FeatureLibrary;
                    }
                    ui.add_space(8.0);
                    ui.separator();
                });
            });

        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(page_bg())
                    .inner_margin(egui::Margin {
                        left: 47.0,
                        right: 56.0,
                        top: 50.0,
                        bottom: 6.0,
                    }),
            )
            .show(ctx, |ui| {
                let (title, _subtitle) = match self.tab {
                    Tab::Home => ("Home", "System overview and operating mode"),
                    Tab::Devices => ("System Settings", "Device configuration and tools"),
                    Tab::AuraSync => ("Lighting & Aura Sync", "Keyboard lighting and effects"),
                    Tab::AuraWallpaper => ("Aura Wallpaper", "Wallpaper gallery and display modes"),
                    Tab::ScenarioProfiles => {
                        ("Scenario Profiles", "Automatic profiles per application")
                    }
                    Tab::GameVisual => ("GameVisual", "Display colour and visual modes"),
                    Tab::FeatureLibrary => {
                        ("Feature Library", "Installed and available capabilities")
                    }
                    Tab::Settings => ("Settings", "General, updates and about"),
                };
                let battery_pct = self
                    .telemetry
                    .as_ref()
                    .and_then(|t| t.battery.as_ref())
                    .map(|b| b.capacity_percent);
                let profile = self.telemetry.as_ref().and_then(|t| t.profile.clone());
                ui.horizontal(|ui| {
                    ui.colored_label(text_main(), egui::RichText::new(title).size(28.0).strong());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        header_icons(ui, battery_pct, profile.as_deref());
                    });
                });
                ui.add_space(33.0);
                egui::ScrollArea::vertical()
                    .scroll_bar_visibility(
                        egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded,
                    )
                    .show(ui, |ui| match self.tab {
                        Tab::Home => self.ui_home(ui),
                        Tab::Devices => self.ui_devices(ui),
                        Tab::AuraSync => self.ui_aura_sync(ui),
                        Tab::AuraWallpaper => self.ui_aura_wallpaper(ui),
                        Tab::ScenarioProfiles => self.ui_scenario_profiles(ui),
                        Tab::GameVisual => self.ui_gamevisual(ui),
                        Tab::FeatureLibrary => self.ui_feature_library(ui),
                        Tab::Settings => self.ui_settings(ui),
                    });
            });

        if self.tab != self.last_tab {
            self.last_tab = self.tab;
            self.tab_changed_at = std::time::Instant::now();
        }
        let transition = self.tab_changed_at.elapsed().as_secs_f32();
        if transition < 0.20 {
            let alpha = ((1.0 - transition / 0.20) * 90.0) as u8;
            let p = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Foreground,
                egui::Id::new("tab-transition"),
            ));
            p.rect_filled(
                ctx.screen_rect(),
                0.0,
                egui::Color32::from_rgba_unmultiplied(0, 0, 0, alpha),
            );
            ctx.request_repaint_after(std::time::Duration::from_millis(16));
        }

        // Transient toast notification for the last action.
        let toast_age = self.status_at.elapsed().as_secs_f32();
        if toast_age < 3.5 {
            if let Some((msg, err)) = &self.status_msg {
                let fade_in = (toast_age / 0.12).min(1.0);
                let fade_out = ((3.5 - toast_age) / 0.5).clamp(0.0, 1.0);
                let alpha = (fade_in.min(fade_out) * 235.0) as u8;
                let c = if *err {
                    egui::Color32::from_rgb(255, 68, 68)
                } else {
                    accent()
                };
                let screen = ctx.screen_rect();
                let rect = egui::Rect::from_min_size(
                    egui::pos2(
                        screen.right() - 380.0,
                        screen.bottom() - 86.0 + (1.0 - fade_in) * 10.0,
                    ),
                    egui::vec2(340.0, 52.0),
                );
                let p = ctx.layer_painter(egui::LayerId::new(
                    egui::Order::Foreground,
                    egui::Id::new("toast"),
                ));
                p.rect_filled(
                    rect,
                    8.0,
                    egui::Color32::from_rgba_unmultiplied(20, 20, 24, alpha),
                );
                p.rect_stroke(
                    rect,
                    8.0,
                    egui::Stroke::new(
                        1.0,
                        egui::Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), alpha),
                    ),
                );
                p.rect_filled(
                    egui::Rect::from_min_size(
                        rect.min + egui::vec2(0.0, 8.0),
                        egui::vec2(3.0, 36.0),
                    ),
                    1.5,
                    egui::Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), alpha),
                );
                p.text(
                    rect.min + egui::vec2(16.0, 26.0),
                    egui::Align2::LEFT_CENTER,
                    msg.chars().take(64).collect::<String>(),
                    egui::FontId::proportional(12.5),
                    egui::Color32::from_rgba_unmultiplied(235, 235, 240, alpha),
                );
                ctx.request_repaint_after(std::time::Duration::from_millis(16));
            }
        }
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
    GameVisual,
    FeatureLibrary,
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
            egui::Stroke::new(2.0, {
                let a = accent();
                egui::Color32::from_rgba_unmultiplied(a.r(), a.g(), a.b(), alpha)
            }),
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
                {
                    let a = accent();
                    egui::Color32::from_rgba_unmultiplied(a.r(), a.g(), a.b(), alpha)
                },
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
            egui::Rect::from_min_size(bar.min, egui::vec2(bar.width() * prog, bar.height())),
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
        let t = self.telemetry.clone();
        ui.heading("System Configuration");
        ui.colored_label(
            text_muted(),
            "Live controls backed by verified kernel interfaces. Every write is capability-checked, clamped and audited by tuf-crated.",
        );
        let writable = self.backend.name() == "daemon" || self.writes_enabled;
        let profile = t
            .as_ref()
            .and_then(|t| t.profile.clone())
            .unwrap_or_else(|| "--".into());
        let profile_label = match profile.as_str() {
            "quiet" => "Silent",
            "balanced" => "Balanced",
            "performance" => "Turbo",
            _ => "Unknown",
        };
        let brightness = t.as_ref().and_then(|t| t.kbd_brightness).unwrap_or(0);
        let max_bright = t
            .as_ref()
            .and_then(|t| t.kbd_max_brightness)
            .unwrap_or(3)
            .max(1);
        let bright_pct = (brightness * 100 / max_bright).min(100);
        let panel_od = t.as_ref().and_then(|t| t.panel_od).unwrap_or(false);
        let charge_limit = t
            .as_ref()
            .and_then(|t| t.battery.as_ref())
            .and_then(|b| b.charge_threshold);
        let pl1 = t.as_ref().and_then(|t| t.pl1_watts).unwrap_or(0);
        let effect = Self::aura_mode_name(self.rgb_mode);
        ui.horizontal(|ui| {
            let (dot, _) = ui.allocate_exact_size(egui::vec2(8.0, 8.0), egui::Sense::hover());
            ui.painter_at(dot).circle_filled(
                dot.center(),
                3.5,
                if writable {
                    egui::Color32::from_rgb(120, 200, 140)
                } else {
                    egui::Color32::from_rgb(200, 160, 80)
                },
            );
            ui.colored_label(
                if writable {
                    egui::Color32::from_rgb(120, 200, 140)
                } else {
                    egui::Color32::from_rgb(200, 160, 80)
                },
                if writable {
                    "Hardware writes: daemon connected (verified + audited)"
                } else {
                    "Monitoring only — run the installer to enable tuf-crated"
                },
            );
        });
        ui.add_space(12.0);
        section_label(ui, "Device controls");
        ui.add_space(8.0);
        let cols = if ui.available_width() > 900.0 { 2 } else { 1 };
        let gap = 14.0;
        let card_w = ((ui.available_width() - gap * (cols as f32 - 1.0)) / cols as f32).max(300.0);
        let medallion = |ui: &mut egui::Ui, icon: u8, active: bool| {
            let (r, _) = ui.allocate_exact_size(egui::vec2(50.0, 50.0), egui::Sense::hover());
            let p = ui.painter_at(r);
            p.circle_filled(
                r.center(),
                25.0,
                if active {
                    egui::Color32::from_rgb(42, 42, 46)
                } else {
                    egui::Color32::from_rgb(34, 34, 38)
                },
            );
            p.circle_stroke(
                r.center(),
                24.0,
                egui::Stroke::new(
                    1.3,
                    egui::Color32::from_rgba_unmultiplied(
                        accent.r(),
                        accent.g(),
                        accent.b(),
                        if active { 150 } else { 55 },
                    ),
                ),
            );
            device_icon(
                &p,
                r.center(),
                icon,
                if active {
                    egui::Color32::WHITE
                } else {
                    text_muted()
                },
            );
        };
        for row in 0..4usize.div_ceil(cols) {
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                for col in 0..cols {
                    let idx = row * cols + col;
                    if idx >= 4 { break; }
                    let card = egui::Frame::none()
                        .fill(card_surface())
                        .rounding(12.0)
                        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(52, 52, 58)))
                        .shadow(egui::epaint::Shadow {
                            offset: egui::vec2(0.0, 3.0),
                            blur: 10.0,
                            spread: 0.0,
                            color: egui::Color32::from_black_alpha(55),
                        })
                        .inner_margin(18.0)
                        .show(ui, |ui| {
                            ui.set_width(card_w - 36.0);
                            ui.set_min_height(164.0);
                            ui.horizontal_top(|ui| {
                                ui.vertical(|ui| {
                                    ui.set_width((card_w - 36.0) * 0.42);
                                    match idx {
                                        0 => {
                                            ui.horizontal(|ui| {
                                                medallion(ui, 0, true);
                                                ui.add_space(10.0);
                                                ui.vertical(|ui| {
                                                    ui.label(egui::RichText::new("Performance").size(16.0).strong());
                                                    ui.colored_label(text_muted(), "Firmware thermal policy");
                                                });
                                            });
                                            ui.add_space(10.0);
                                            ui.label(egui::RichText::new(profile_label).size(26.0).strong().color(accent));
                                            ui.label(egui::RichText::new(format!("CPU cap {} W sustained", pl1)).size(11.5).color(text_muted()));
                                        }
                                        1 => {
                                            ui.horizontal(|ui| {
                                                medallion(ui, 1, true);
                                                ui.add_space(10.0);
                                                ui.vertical(|ui| {
                                                    ui.label(egui::RichText::new("Keyboard").size(16.0).strong());
                                                    ui.colored_label(text_muted(), "Aura lighting & backlight");
                                                });
                                            });
                                            ui.add_space(10.0);
                                            ui.horizontal(|ui| {
                                                let (sw, _) = ui.allocate_exact_size(egui::vec2(24.0, 24.0), egui::Sense::hover());
                                                ui.painter_at(sw).rect_filled(sw, 4.0, egui::Color32::from_rgb(self.rgb_color[0], self.rgb_color[1], self.rgb_color[2]));
                                                ui.label(egui::RichText::new(effect).size(22.0).strong());
                                            });
                                            ui.label(egui::RichText::new(format!("#{:02X}{:02X}{:02X} · {} % brightness", self.rgb_color[0], self.rgb_color[1], self.rgb_color[2], bright_pct)).size(11.5).color(text_muted()));
                                        }
                                        2 => {
                                            ui.horizontal(|ui| {
                                                medallion(ui, 2, charge_limit.is_some_and(|v| v < 100));
                                                ui.add_space(10.0);
                                                ui.vertical(|ui| {
                                                    ui.label(egui::RichText::new("Power & battery").size(16.0).strong());
                                                    ui.colored_label(text_muted(), "Charge care");
                                                });
                                            });
                                            ui.add_space(10.0);
                                            let batt = t.as_ref().and_then(|t| t.battery.as_ref()).map(|b| b.capacity_percent).unwrap_or(0);
                                            ui.label(egui::RichText::new(format!("{batt} % charge")).size(24.0).strong().color(accent));
                                            ui.label(egui::RichText::new(charge_limit.map(|v| format!("{v} % limit")).unwrap_or_else(|| "No charge limit".into())).size(11.5).color(text_muted()));
                                        }
                                        _ => {
                                            ui.horizontal(|ui| {
                                                medallion(ui, 3, panel_od);
                                                ui.add_space(10.0);
                                                ui.vertical(|ui| {
                                                    ui.label(egui::RichText::new("Display & audio").size(16.0).strong());
                                                    ui.colored_label(text_muted(), "Panel and output");
                                                });
                                            });
                                            ui.add_space(10.0);
                                            ui.label(egui::RichText::new(format!("Overdrive {}", if panel_od { "On" } else { "Off" })).size(24.0).strong().color(accent));
                                            ui.label(egui::RichText::new(format!("PipeWire output {:.0} %", self.audio_volume)).size(11.5).color(text_muted()));
                                        }
                                    }
                                });
                                ui.add_space(14.0);
                                ui.vertical(|ui| {
                                    ui.set_width((card_w - 36.0) * 0.58 - 14.0);
                                    match idx {
                                        0 => {
                                            ui.label(egui::RichText::new("Operating mode").size(11.0).color(text_muted()));
                                            ui.add_space(4.0);
                                            for (label, p) in [("Silent", Profile::Quiet), ("Balanced", Profile::Balanced), ("Turbo", Profile::Performance)] {
                                                let selected = profile == p.as_str();
                                                let (r, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 28.0), egui::Sense::click());
                                                let pnt = ui.painter_at(r);
                                                pnt.rect_filled(r, 5.0, if selected { egui::Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), 35) } else { egui::Color32::from_rgb(28, 28, 31) });
                                                pnt.rect_stroke(r.shrink(0.5), 5.0, egui::Stroke::new(if selected { 1.3 } else { 1.0 }, if selected { accent } else { egui::Color32::from_rgb(58, 58, 64) }));
                                                pnt.text(egui::pos2(r.left() + 12.0, r.center().y), egui::Align2::LEFT_CENTER, label, egui::FontId::proportional(12.0), if selected { accent } else { text_main() });
                                                if resp.clicked() {
                                                    if let Err(e) = self.backend.set_profile(p, true) {
                                                        self.report(format!("profile failed: {}", friendly_error(&e)), true);
                                                    }
                                                    self.poll();
                                                }
                                            }
                                            ui.add_space(2.0);
                                            if ui.small_button("Restore 65 W cap").clicked() {
                                                if let Err(e) = self.backend.set_cpu_power_limit(65, true) {
                                                    self.report(format!("power limit failed: {}", friendly_error(&e)), true);
                                                } else {
                                                    self.report("CPU cap -> 65 W default".into(), false);
                                                }
                                                self.poll();
                                            }
                                        }
                                        1 => {
                                            ui.label(egui::RichText::new("Brightness").size(11.0).color(text_muted()));
                                            ui.add_space(4.0);
                                            ui.horizontal(|ui| {
                                                for lvl in 0..=max_bright {
                                                    let label = format!("{} %", lvl * 100 / max_bright);
                                                    if ui.selectable_label(brightness == lvl, label).clicked() {
                                                        if let Err(e) = self.backend.set_kbd_brightness(lvl, true) {
                                                            self.report(format!("brightness failed: {}", friendly_error(&e)), true);
                                                        }
                                                        self.poll();
                                                    }
                                                }
                                            });
                                            ui.add_space(6.0);
                                            if ui.small_button("Open Aura Effects >").clicked() {
                                                self.tab = Tab::AuraSync;
                                                self.aura_sub = AuraTab::Effects;
                                            }
                                        }
                                        2 => {
                                            ui.label(egui::RichText::new("Charge limit").size(11.0).color(text_muted()));
                                            ui.add_space(4.0);
                                            ui.horizontal(|ui| {
                                                for (label, target) in [("80 %", 80u32), ("100 %", 100)] {
                                                    if ui.selectable_label(charge_limit == Some(target), label).clicked() {
                                                        if let Err(e) = self.backend.set_charge_threshold(target, true) {
                                                            self.report(format!("charge limit failed: {}", friendly_error(&e)), true);
                                                        }
                                                        self.poll();
                                                    }
                                                }
                                            });
                                            ui.add_space(8.0);
                                            ui.colored_label(text_muted(), t.as_ref().and_then(|t| t.charge_mode.clone()).map(|m| format!("Charge mode {m}")).unwrap_or_else(|| "Charge mode unavailable".into()));
                                        }
                                        _ => {
                                            ui.label(egui::RichText::new("Panel overdrive").size(11.0).color(text_muted()));
                                            ui.add_space(4.0);
                                            ui.horizontal(|ui| {
                                                for (label, on) in [("On", true), ("Off", false)] {
                                                    if ui.selectable_label(panel_od == on, label).clicked() {
                                                        if let Err(e) = self.backend.set_panel_overdrive(on, true) {
                                                            self.report(format!("panel_od failed: {}", friendly_error(&e)), true);
                                                        }
                                                        self.poll();
                                                    }
                                                }
                                            });
                                            ui.add_space(8.0);
                                            ui.horizontal(|ui| {
                                                if ui.small_button("Audio >").clicked() {
                                                    self.devices_sub = DeviceTab::Audio;
                                                }
                                                if ui.small_button("Monitor >").clicked() {
                                                    self.devices_sub = DeviceTab::ResourceMonitor;
                                                }
                                            });
                                        }
                                    }
                                });
                            });
                        });
                    let rect = card.response.rect;
                    let hovered = ui.ctx().pointer_hover_pos().map(|p| rect.contains(p)).unwrap_or(false);
                    let t = ui.ctx().animate_bool_with_time(ui.id().with(("devcard-hover", idx)), hovered, 0.12);
                    if t > 0.01 {
                        ui.painter().rect_stroke(
                            rect.shrink(0.5),
                            12.0,
                            egui::Stroke::new(
                                1.0 + 1.2 * t,
                                egui::Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), (150.0 * t) as u8),
                            ),
                        );
                    }
                    if col + 1 < cols && idx + 1 < 4 {
                        ui.add_space(gap);
                    }
                }
            });
            ui.add_space(12.0);
        }

        ui.add_space(6.0);
        let (bios, kernel) = (
            std::fs::read_to_string("/sys/class/dmi/id/bios_version")
                .unwrap_or_else(|_| "--".into())
                .trim()
                .to_string(),
            std::fs::read_to_string("/proc/sys/kernel/osrelease")
                .unwrap_or_else(|_| "--".into())
                .trim()
                .to_string(),
        );
        egui::Frame::none()
            .fill(card_bg2())
            .rounding(8.0)
            .inner_margin(14.0)
            .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(44, 44, 50)))
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.horizontal_wrapped(|ui| {
                    for (k, v) in [
                        ("Model", self.hero_model.clone()),
                        ("BIOS", bios),
                        ("Kernel", kernel),
                        (
                            "Control",
                            if writable {
                                "tuf-crated · verified writes".into()
                            } else {
                                "monitoring only".into()
                            },
                        ),
                        ("RGB channel", "WMNB 0x00100056".into()),
                    ] {
                        ui.colored_label(text_muted(), format!("{k}:"));
                        ui.label(v);
                        ui.add_space(18.0);
                    }
                });
            });

        ui.add_space(10.0);
        egui::CollapsingHeader::new("Firmware features not exposed on this model")
            .default_open(false)
            .show(ui, |ui| {
                ui.colored_label(
                    text_muted(),
                    "These are Armoury Crate's Windows device features. They need firmware/OS hooks this Linux installation does not expose, so tuf-crate reports them instead of offering dead buttons.",
                );
                for (name, why) in [
                    ("Win Key lock", "requires the ASUS keyboard filter driver; Linux input is managed by the desktop"),
                    ("Touchpad disable", "XFCE/libinput owns the touchpad; no XInput helper is installed"),
                    ("Modern Standby Assistant", "Windows Modern Standby ACPI is not used by the Linux s2idle path"),
                    ("Panel Power Saver", "panel self-refresh is driven by i915/NVIDIA, with no ASUS control hook"),
                    ("Power mode synchronization", "TLP owns the CPU governor on this installation; tuf-crate does not fight it"),
                ] {
                    ui.horizontal(|ui| {
                        ui.colored_label(egui::Color32::from_rgb(150, 150, 160), "-");
                        ui.colored_label(text_main(), name);
                        ui.colored_label(text_muted(), why);
                    });
                }
            });

        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Advanced controls").size(16.0).strong());
            info_icon(
                ui,
                "Every control is capability-checked, clamped and audited by tuf-crated.                  Dangerous or firmware-rejected operations are not implemented.",
            );
        });
        ui.add_space(8.0);
        collapsible_card(
            ui,
            "adv_perf",
            "Performance & power",
            "Silent/Balanced/Turbo use platform_profile; the CPU cap uses intel-rapl PL1.",
            true,
            |ui| self.ui_performance(ui),
        );
        collapsible_card(
            ui,
            "adv_fans",
            "Fans & thermals",
            "Fan RPM is reported by the ASUS EC. The EC acknowledges FULLSPEED/AUTO but does not              change RPM on this model; the thermal profile is the effective control. Custom fan              curves are not exposed by this firmware.",
            false,
            |ui| self.ui_fans(ui),
        );
        collapsible_card(
            ui,
            "adv_battery",
            "Battery",
            "Charge threshold and charge mode are native kernel/ASUS attributes. Every change is              reversible and audited.",
            false,
            |ui| self.ui_battery(ui),
        );
        collapsible_card(
            ui,
            "adv_display",
            "Display & GPU",
            "Panel overdrive is a verified kernel attribute. Eco Mode is boot-time only; live              dGPU power-off is blocked because this EC froze the compositor when written live.",
            false,
            |ui| self.ui_display_gpu(ui),
        );
        collapsible_card(
            ui,
            "adv_inventory",
            "Hardware inventory",
            "Live sensor snapshot: circular gauges for CPU/GPU/fans, secondary values in the grid.              No raw duplicate list or thermal bars.",
            false,
            |ui| self.ui_dashboard(ui),
        );
    }

    fn ui_free_memory(&mut self, ui: &mut egui::Ui) {
        let accent = self.theme.accent();
        ui.horizontal(|ui| {
            ui.heading("Free up memory");
            info_icon(
                ui,
                "Select non-critical processes to stop. Only your own processes are listed;                  system and session-critical processes are excluded and protected.",
            );
        });
        ui.add_space(8.0);
        let rows = read_proc_rows(30);
        let ram = self
            .telemetry
            .as_ref()
            .and_then(|t| match (t.ram_used_gb, t.ram_total_gb) {
                (Some(u), Some(tot)) => Some((u, tot)),
                _ => None,
            });
        let cards = [
            (
                "Memory in use",
                ram.map(|(u, tot)| format!("{u:.1} / {tot:.1} GB"))
                    .unwrap_or_else(|| "--".into()),
                ram.map(|(u, tot)| u / tot.max(0.1)).unwrap_or(0.0),
            ),
            (
                "CPU load",
                format!("{} %", self.cpu_util.unwrap_or(0)),
                self.cpu_util.unwrap_or(0) as f32 / 100.0,
            ),
            (
                "Selected",
                format!("{} process(es)", self.free_selected.len()),
                (self.free_selected.len() as f32 / 8.0).clamp(0.0, 1.0),
            ),
        ];
        let gap = 10.0;
        let cw = ((ui.available_width() - 2.0 * gap) / 3.0).max(220.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            for (i, (label, value, frac)) in cards.iter().enumerate() {
                let (r, _) = ui.allocate_exact_size(egui::vec2(cw, 86.0), egui::Sense::hover());
                let p = ui.painter_at(r);
                p.rect_filled(r, 8.0, card_bg());
                p.rect_stroke(
                    r.shrink(0.5),
                    8.0,
                    egui::Stroke::new(1.0, egui::Color32::from_rgb(44, 44, 50)),
                );
                p.text(
                    egui::pos2(r.left() + 16.0, r.top() + 18.0),
                    egui::Align2::LEFT_CENTER,
                    *label,
                    egui::FontId::proportional(11.5),
                    text_muted(),
                );
                p.text(
                    egui::pos2(r.left() + 16.0, r.top() + 43.0),
                    egui::Align2::LEFT_CENTER,
                    value,
                    egui::FontId::proportional(18.0),
                    text_main(),
                );
                let bar = egui::Rect::from_min_size(
                    egui::pos2(r.left() + 16.0, r.bottom() - 18.0),
                    egui::vec2(r.width() - 32.0, 3.0),
                );
                p.rect_filled(bar, 1.5, meter_bg());
                if *frac > 0.0 {
                    p.rect_filled(
                        egui::Rect::from_min_size(
                            bar.min,
                            egui::vec2(bar.width() * frac.clamp(0.0, 1.0), 3.0),
                        ),
                        1.5,
                        accent,
                    );
                }
                if i < 2 {
                    ui.add_space(gap);
                }
            }
        });
        ui.add_space(10.0);
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
                self.report(format!("Sent SIGTERM to {ok} process(es)"), ok == 0);
                self.free_selected.clear();
                self.free_confirm = false;
            }
        });
        ui.add_space(6.0);
        let now = std::time::Instant::now();
        let dt = now
            .duration_since(self.proc_last_at)
            .as_secs_f32()
            .max(0.05);
        // Show percentage of the whole machine (0–100), not per-core
        // (which can exceed 100 % and looks like a telemetry bug).
        let ncpu = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1) as f32;
        for row in &rows {
            if let Some(prev) = self.proc_last.get(&row.pid) {
                let delta = row.ticks.saturating_sub(*prev) as f32;
                let pct = (delta / 100.0 / dt * 100.0 / ncpu).clamp(0.0, 100.0);
                self.proc_cpu.insert(row.pid, pct);
            }
            self.proc_last.insert(row.pid, row.ticks);
        }
        self.proc_last_at = now;
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Processes").size(14.5));
            info_icon(
                ui,
                "Only your own non-critical processes are listed. System/session processes are                  protected. Identical process names are grouped; click a group to expand it.",
            );
        });
        ui.add_space(6.0);
        // group by comm
        type GroupData = (Vec<i32>, f32, u64);
        let mut groups: std::collections::BTreeMap<String, GroupData> =
            std::collections::BTreeMap::new();
        for row in &rows {
            let cpu = self.proc_cpu.get(&row.pid).copied().unwrap_or(0.0);
            let g = groups.entry(row.comm.clone()).or_insert((Vec::new(), 0.0, 0));
            g.0.push(row.pid);
            g.1 += cpu;
            g.2 += row.rss_mb;
        }
        let mut list: Vec<(String, GroupData)> = groups.into_iter().collect();
        list.sort_by_key(|b| std::cmp::Reverse(b.1 .2));
        let details: std::collections::HashMap<i32, (String, f32, u64)> = rows
            .iter()
            .map(|r| {
                (
                    r.pid,
                    (
                        r.comm.clone(),
                        self.proc_cpu.get(&r.pid).copied().unwrap_or(0.0),
                        r.rss_mb,
                    ),
                )
            })
            .collect();
        egui::Frame::none()
            .fill(card_surface())
            .rounding(10.0)
            .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(52, 52, 58)))
            .shadow(egui::epaint::Shadow {
                offset: egui::vec2(0.0, 3.0),
                blur: 10.0,
                spread: 0.0,
                color: egui::Color32::from_black_alpha(55),
            })
            .inner_margin(14.0)
            .show(ui, |ui| {
                let full_w = ui.available_width();
                let (head, _) = ui.allocate_exact_size(egui::vec2(full_w, 26.0), egui::Sense::hover());
                let hp = ui.painter_at(head);
                let muted = text_muted();
                hp.text(egui::pos2(head.left() + 40.0, head.center().y), egui::Align2::LEFT_CENTER, "Stop", egui::FontId::proportional(11.0), muted);
                hp.text(egui::pos2(head.left() + 64.0, head.center().y), egui::Align2::LEFT_CENTER, "Process", egui::FontId::proportional(11.0), muted);
                hp.text(egui::pos2(head.right() - 150.0, head.center().y), egui::Align2::RIGHT_CENTER, "CPU", egui::FontId::proportional(11.0), muted);
                hp.text(egui::pos2(head.right() - 16.0, head.center().y), egui::Align2::RIGHT_CENTER, "RAM", egui::FontId::proportional(11.0), muted);

                for (gi, (name, (pids, cpu, rss))) in list.iter().enumerate() {
                    let safe = safe_to_stop(name);
                    let all_selected = pids.iter().all(|p| self.free_selected.contains(p));
                    let expanded = self.free_expanded.contains(name);
                    let (rect, resp) = ui.allocate_exact_size(egui::vec2(full_w, 42.0), egui::Sense::click());
                    let p = ui.painter_at(rect);
                    if gi % 2 == 1 {
                        p.rect_filled(rect, 0.0, egui::Color32::from_rgba_unmultiplied(255, 255, 255, 5));
                    }
                    // chevron
                    let cx = rect.left() + 16.0;
                    let cy = rect.center().y;
                    let col = egui::Color32::from_rgb(170, 170, 180);
                    if pids.len() > 1 {
                        if expanded {
                            p.add(egui::Shape::line(vec![egui::pos2(cx - 4.0, cy - 1.0), egui::pos2(cx, cy + 3.0), egui::pos2(cx + 4.0, cy - 1.0)], egui::Stroke::new(1.3, col)));
                        } else {
                            p.add(egui::Shape::line(vec![egui::pos2(cx - 1.0, cy - 4.0), egui::pos2(cx + 3.0, cy), egui::pos2(cx - 1.0, cy + 4.0)], egui::Stroke::new(1.3, col)));
                        }
                    }
                    let chk = egui::Rect::from_min_size(egui::pos2(rect.left() + 32.0, cy - 9.0), egui::vec2(18.0, 18.0));
                    let chk_resp = ui.interact(chk, egui::Id::new(("fm_group_chk", name)), egui::Sense::click());
                    p.rect_stroke(chk, 3.0, egui::Stroke::new(1.2, if all_selected { accent } else if safe { egui::Color32::from_rgb(100, 100, 110) } else { egui::Color32::from_rgb(70, 65, 60) }));
                    if all_selected {
                        p.add(egui::Shape::line(vec![egui::pos2(chk.left() + 4.0, chk.center().y), egui::pos2(chk.center().x - 1.0, chk.bottom() - 5.0), egui::pos2(chk.right() - 4.0, chk.top() + 5.0)], egui::Stroke::new(1.8, accent)));
                    }
                    let label = if pids.len() > 1 {
                        format!("{}  ({} processes)", name, pids.len())
                    } else {
                        format!("{}  ({})", name, pids[0])
                    };
                    p.text(egui::pos2(rect.left() + 64.0, cy - 7.0), egui::Align2::LEFT_CENTER, &label, egui::FontId::proportional(12.5), if safe { text_main() } else { egui::Color32::from_rgb(150, 140, 120) });
                    p.text(egui::pos2(rect.left() + 64.0, cy + 11.0), egui::Align2::LEFT_CENTER, if pids.len() > 1 { "grouped by process name" } else { "single process" }, egui::FontId::proportional(10.0), text_muted());
                    p.text(egui::pos2(rect.right() - 150.0, cy), egui::Align2::RIGHT_CENTER, format!("{cpu:.1} %"), egui::FontId::proportional(12.0), text_muted());
                    p.text(egui::pos2(rect.right() - 16.0, cy), egui::Align2::RIGHT_CENTER, format!("{} MB", rss), egui::FontId::proportional(12.0), text_muted());
                    if chk_resp.clicked() && safe {
                        if all_selected {
                            for pid in pids { self.free_selected.remove(pid); }
                        } else {
                            for pid in pids { self.free_selected.insert(*pid); }
                        }
                    } else if resp.clicked() && pids.len() > 1 {
                        if expanded { self.free_expanded.remove(name); } else { self.free_expanded.insert(name.clone()); }
                    }
                    if expanded {
                        for (pi, pid) in pids.iter().enumerate() {
                            if let Some((nm, pcpu, prss)) = details.get(pid) {
                                let (sr, sr_resp) = ui.allocate_exact_size(egui::vec2(full_w, 30.0), egui::Sense::click());
                                let sp = ui.painter_at(sr);
                                if (gi + pi) % 2 == 0 {
                                    sp.rect_filled(sr, 0.0, egui::Color32::from_rgba_unmultiplied(255, 255, 255, 3));
                                }
                                let schk = egui::Rect::from_min_size(egui::pos2(sr.left() + 56.0, sr.center().y - 8.0), egui::vec2(16.0, 16.0));
                                let schk_resp = ui.interact(schk, egui::Id::new(("fm_pid_chk", pid)), egui::Sense::click());
                                let selected = self.free_selected.contains(pid);
                                sp.rect_stroke(schk, 3.0, egui::Stroke::new(1.1, if selected { accent } else { egui::Color32::from_rgb(80, 80, 90) }));
                                if selected {
                                    sp.add(egui::Shape::line(vec![egui::pos2(schk.left() + 3.0, schk.center().y), egui::pos2(schk.center().x - 1.0, schk.bottom() - 5.0), egui::pos2(schk.right() - 3.0, schk.top() + 5.0)], egui::Stroke::new(1.6, accent)));
                                }
                                sp.text(egui::pos2(sr.left() + 80.0, sr.center().y), egui::Align2::LEFT_CENTER, format!("{nm}  ({pid})"), egui::FontId::proportional(11.5), text_muted());
                                sp.text(egui::pos2(sr.right() - 150.0, sr.center().y), egui::Align2::RIGHT_CENTER, format!("{pcpu:.1} %"), egui::FontId::proportional(11.0), text_muted());
                                sp.text(egui::pos2(sr.right() - 16.0, sr.center().y), egui::Align2::RIGHT_CENTER, format!("{prss} MB"), egui::FontId::proportional(11.0), text_muted());
                                if schk_resp.clicked() || sr_resp.clicked() {
                                    if selected {
                                        self.free_selected.remove(pid);
                                    } else {
                                        self.free_selected.insert(*pid);
                                    }
                                }
                            }
                        }
                    }
                }
            });
        let _ = accent;
    }

    fn ui_gpu_performance(&mut self, ui: &mut egui::Ui) {
        let accent = self.theme.accent();
        ui.horizontal(|ui| {
            ui.heading("GPU Performance");
            info_icon(
                ui,
                "This laptop has no MUX switch. GPU mode is a firmware/EC setting that is only                  safe to change at boot; live Eco power-off froze the compositor on this machine.",
            );
        });
        ui.add_space(8.0);
        let dgpu_off = self
            .telemetry
            .as_ref()
            .and_then(|t| t.dgpu_disable)
            .unwrap_or(false);
        let nv = self.telemetry.as_ref().and_then(|t| t.nvidia.clone());
        let stat =
            |label: &str, value: String, sub: &str| (label.to_string(), value, sub.to_string());
        let stats = [
            stat(
                "GPU temperature",
                nv.as_ref()
                    .map(|g| format!("{} °C", g.temperature_c))
                    .unwrap_or_else(|| "--".into()),
                "nvidia-smi",
            ),
            stat(
                "GPU utilisation",
                nv.as_ref()
                    .map(|g| format!("{} %", g.utilization_percent))
                    .unwrap_or_else(|| "--".into()),
                "live",
            ),
            stat(
                "Board power",
                nv.as_ref()
                    .map(|g| format!("{:.1} W", g.power_watts))
                    .unwrap_or_else(|| "--".into()),
                "read-only",
            ),
            stat(
                "Performance state",
                nv.as_ref()
                    .map(|g| g.perf_state.clone())
                    .unwrap_or_else(|| "--".into()),
                "driver",
            ),
        ];
        let avail = ui.available_width();
        let gap = 10.0;
        let sw = ((avail - 3.0 * gap) / 4.0).max(150.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            for (i, (label, value, sub)) in stats.iter().enumerate() {
                let (r, _) = ui.allocate_exact_size(egui::vec2(sw, 78.0), egui::Sense::hover());
                let p = ui.painter_at(r);
                p.rect_filled(r, 8.0, card_bg());
                p.rect_stroke(
                    r.shrink(0.5),
                    8.0,
                    egui::Stroke::new(1.0, egui::Color32::from_rgb(44, 44, 50)),
                );
                p.text(
                    egui::pos2(r.left() + 14.0, r.top() + 18.0),
                    egui::Align2::LEFT_CENTER,
                    label,
                    egui::FontId::proportional(11.0),
                    text_muted(),
                );
                p.text(
                    egui::pos2(r.left() + 14.0, r.top() + 42.0),
                    egui::Align2::LEFT_CENTER,
                    value,
                    egui::FontId::proportional(19.0),
                    if dgpu_off { text_muted() } else { text_main() },
                );
                p.text(
                    egui::pos2(r.left() + 14.0, r.bottom() - 13.0),
                    egui::Align2::LEFT_CENTER,
                    sub,
                    egui::FontId::proportional(10.0),
                    text_muted(),
                );
                if i < 3 {
                    ui.add_space(gap);
                }
            }
        });
        ui.add_space(12.0);
        if dgpu_off {
            egui::Frame::none()
                .fill(egui::Color32::from_rgba_unmultiplied(235, 140, 50, 22))
                .rounding(8.0)
                .stroke(egui::Stroke::new(1.0, accent))
                .inner_margin(12.0)
                .show(ui, |ui| {
                    ui.colored_label(
                        accent,
                        egui::RichText::new("Eco Mode is active — the NVIDIA GPU is powered off.")
                            .strong(),
                    );
                    ui.colored_label(
                        text_muted(),
                        "Restore Standard here; a reboot may be required before nvidia-smi and the NVIDIA driver return.",
                    );
                    if ui.button("Restore Standard (safe direction)").clicked() {
                        match self.backend.set_dgpu_disabled(false, true) {
                            Ok(()) => {
                                self.schedule_boot_gpu(None);
                                self.report(
                                    "Standard restored — reboot if the NVIDIA driver is still absent".into(),
                                    false,
                                );
                            }
                            Err(e) => self.report(
                                format!("GPU mode failed: {}", friendly_error(&e)),
                                true,
                            ),
                        }
                        self.poll();
                    }
                });
            ui.add_space(8.0);
        }
        let boot = self
            .backend
            .get_boot_config()
            .ok()
            .flatten()
            .and_then(|v| v.get("dgpu_disable").and_then(|d| d.as_bool()));
        if !dgpu_off {
            egui::Frame::none()
                .fill(card_bg())
                .rounding(8.0)
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(44, 44, 50)))
                .inner_margin(12.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                ui.colored_label(
                                    egui::Color32::from_rgb(255, 107, 0),
                                    egui::RichText::new("Eco Mode — safe boot-time switch").strong(),
                                );
                                info_icon(
                                    ui,
                                    "The daemon applies dgpu_disable before the display manager                                      starts, then you reboot. No live EC write while NVIDIA is bound.",
                                );
                            });
                            if boot == Some(true) {
                                ui.colored_label(
                                    egui::Color32::from_rgb(120, 200, 140),
                                    "● Scheduled: dGPU will be disabled on the next boot.",
                                );
                            }
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if boot == Some(true) {
                                if ui.button("Cancel schedule").clicked() {
                                    self.schedule_boot_gpu(None);
                                }
                            } else if ui.button("Schedule Eco for next boot").clicked() {
                                self.schedule_boot_gpu(Some(true));
                            }
                        });
                    });
                });
            ui.add_space(10.0);
        }
        let modes = [
            (
                "Standard",
                "Hybrid graphics — dGPU available for rendering.",
                false,
            ),
            (
                "Eco Mode",
                "Boot-time only on this build. Live power-off is blocked because this EC can hang the display.",
                true,
            ),
            (
                "Optimized",
                "Requires a MUX switch — not present on the FX706HF.",
                false,
            ),
        ];
        let mode_w = (((ui.available_width() - 2.0 * 12.0) / 3.0) - 28.0).max(190.0);
        ui.horizontal(|ui| {
            for (i, (name, desc, is_eco)) in modes.iter().enumerate() {
                let selected = if *is_eco {
                    dgpu_off
                } else if i == 0 {
                    !dgpu_off
                } else {
                    false
                };
                egui::Frame::none()
                    .fill(if selected {
                        egui::Color32::from_rgba_unmultiplied(
                            accent.r(),
                            accent.g(),
                            accent.b(),
                            24,
                        )
                    } else {
                        card_bg()
                    })
                    .rounding(8.0)
                    .stroke(egui::Stroke::new(
                        if selected { 1.5 } else { 1.0 },
                        if selected {
                            accent
                        } else {
                            egui::Color32::from_rgb(44, 44, 48)
                        },
                    ))
                    .inner_margin(14.0)
                    .show(ui, |ui| {
                        ui.set_width(mode_w);
                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new(*name).size(16.0).strong());
                            ui.colored_label(text_muted(), *desc);
                            if i == 2 {
                                ui.colored_label(
                                    egui::Color32::from_rgb(160, 140, 110),
                                    "Unsupported on this hardware",
                                );
                            } else if i == 1 {
                                ui.add_enabled(
                                    false,
                                    egui::Button::new(if selected {
                                        "Active (firmware)"
                                    } else {
                                        "Eco Mode — boot-time only"
                                    }),
                                )
                                .on_disabled_hover_text(
                                    "Live dGPU power-off is blocked: it hangs the compositor.\
                                     A reboot-safe boot-time switch is not installed.",
                                );
                            } else if selected {
                                ui.add_enabled(false, egui::Button::new("Active"));
                            } else if ui.button("Use Standard").clicked() {
                                match self.backend.set_dgpu_disabled(false, true) {
                                    Ok(()) => self.report(
                                        "dGPU enabled — reboot if the NVIDIA driver is still absent"
                                            .to_string(),
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
        ui.add_space(12.0);
        section_label(ui, "GPU compute processes");
        ui.add_space(6.0);
        egui::Frame::none()
            .fill(card_bg())
            .rounding(8.0)
            .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(44, 44, 50)))
            .inner_margin(14.0)
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                match std::process::Command::new("nvidia-smi")
                    .args([
                        "--query-compute-apps=pid,process_name,used_gpu_memory",
                        "--format=csv,noheader",
                    ])
                    .output()
                {
                    Ok(o) if o.status.success() => {
                        let text = String::from_utf8_lossy(&o.stdout);
                        if text.trim().is_empty() {
                            ui.horizontal(|ui| {
                                ui.colored_label(text_muted(), "-");
                                ui.label("No GPU compute processes running.");
                                ui.colored_label(
                                    text_muted(),
                                    "The dGPU is idle or handled by the desktop.",
                                );
                            });
                        } else {
                            egui::Grid::new("gpu_procs")
                                .num_columns(3)
                                .spacing([26.0, 6.0])
                                .striped(true)
                                .show(ui, |ui| {
                                    ui.colored_label(text_muted(), "PID");
                                    ui.colored_label(text_muted(), "Process");
                                    ui.colored_label(text_muted(), "VRAM");
                                    ui.end_row();
                                    for line in text.lines() {
                                        let parts: Vec<&str> =
                                            line.split(',').map(str::trim).collect();
                                        if parts.len() >= 3 {
                                            ui.label(parts[0]);
                                            ui.label(parts[1]);
                                            ui.label(parts[2]);
                                            ui.end_row();
                                        } else {
                                            ui.monospace(line);
                                            ui.end_row();
                                        }
                                    }
                                });
                        }
                    }
                    _ => {
                        ui.colored_label(
                            text_muted(),
                            "nvidia-smi unavailable — GPU process list not readable.",
                        );
                    }
                }
            });
    }

    fn ui_audio(&mut self, ui: &mut egui::Ui) {
        if !self.audio_loaded {
            if let Some((v, m)) = read_default_volume() {
                self.audio_volume = v * 100.0;
                self.audio_muted = m;
            }
            self.audio_loaded = true;
        }
        let accent = self.theme.accent();
        let status = std::process::Command::new("wpctl")
            .arg("status")
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).into_owned());
        let sink = status
            .as_deref()
            .and_then(default_sink_name)
            .unwrap_or_else(|| "Default PipeWire output".into());
        ui.colored_label(
            text_muted(),
            "PipeWire output volume and mute. Device effects belong to the desktop audio server.",
        );
        ui.add_space(10.0);

        let av = ui.available_width();
        let left_w = (av * 0.52).clamp(420.0, 760.0);
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.set_width(left_w);
                egui::Frame::none()
                    .fill(card_bg())
                    .rounding(10.0)
                    .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(46, 46, 52)))
                    .inner_margin(18.0)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.colored_label(egui::Color32::from_rgb(120, 200, 140), "DEFAULT OUTPUT");
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.small_button("Refresh").clicked() {
                                    self.audio_loaded = false;
                                }
                            });
                        });
                        ui.add_space(8.0);
                        ui.label(egui::RichText::new("Master volume").size(12.0).color(text_muted()));
                        ui.label(
                            egui::RichText::new(format!("{:.0} %", self.audio_volume))
                                .size(34.0)
                                .strong()
                                .color(if self.audio_muted { text_muted() } else { accent }),
                        );
                        ui.label(egui::RichText::new(&sink).size(14.0));
                        ui.add_space(8.0);
                        let mut vol = self.audio_volume;
                        let resp = ui.add_sized(
                            [ui.available_width(), 24.0],
                            egui::Slider::new(&mut vol, 0.0..=100.0).show_value(false),
                        );
                        self.audio_volume = vol;
                        if resp.changed()
                            && self.audio_last_set.elapsed() > std::time::Duration::from_millis(180)
                        {
                            apply_default_volume(self.audio_volume);
                            self.audio_last_set = std::time::Instant::now();
                        }
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            let label = if self.audio_muted { "Unmute" } else { "Mute" };
                            if ui
                                .add_sized([110.0, 30.0], egui::Button::new(label))
                                .clicked()
                            {
                                self.audio_muted = !self.audio_muted;
                                apply_default_mute(self.audio_muted);
                            }
                            ui.colored_label(
                                text_muted(),
                                if self.audio_muted { "output muted" } else { "output active" },
                            );
                        });
                    });
            });
            ui.add_space(12.0);
            ui.vertical(|ui| {
                ui.set_width((ui.available_width() - 4.0).max(280.0));
                egui::Frame::none()
                    .fill(card_bg2())
                    .rounding(10.0)
                    .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(46, 46, 52)))
                    .inner_margin(16.0)
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new("Routing").size(13.5).strong());
                        ui.add_space(6.0);
                        for (k, v) in [
                            ("Server", "PipeWire / WirePlumber"),
                            ("Target", sink.as_str()),
                            ("Control", "wpctl set-volume / set-mute"),
                            ("Effects", "desktop-owned"),
                        ] {
                            ui.horizontal(|ui| {
                                ui.colored_label(text_muted(), k);
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    ui.label(v);
                                });
                            });
                            ui.add_space(2.0);
                            ui.separator();
                        }
                        ui.add_space(6.0);
                        ui.colored_label(
                            text_muted(),
                            "Use the desktop mixer for per-application routing, equalizers and TV/enhancement features.",
                        );
                    });
            });
        });

        ui.add_space(14.0);
        section_label(ui, "PipeWire device inventory");
        ui.add_space(8.0);
        egui::Frame::none()
            .fill(card_bg())
            .rounding(8.0)
            .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(44, 44, 50)))
            .inner_margin(14.0)
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                if let Some(text) = status {
                    let lines: Vec<&str> = text.lines().take(30).collect();
                    ui.monospace(lines.join("\n"));
                } else {
                    ui.colored_label(
                        egui::Color32::from_rgb(220, 160, 90),
                        "wpctl not available — audio controls require PipeWire/WirePlumber.",
                    );
                }
            });
    }

    fn ui_resource_monitor(&mut self, ui: &mut egui::Ui) {
        let Some(t) = self.telemetry.clone() else {
            return;
        };
        let accent = self.theme.accent();
        ui.colored_label(
            text_muted(),
            "Live CPU, GPU, fan and memory telemetry. Values refresh every 0.5 s; fan RPM is reported by the ASUS EC.",
        );
        ui.add_space(10.0);
        let cards = [
            (
                "CPU",
                Self::temp_c(t.cpu_package_temp_mC),
                format!(
                    "{} % load · {} MHz",
                    self.cpu_util.unwrap_or(0),
                    self.cpu_mhz.unwrap_or(0)
                ),
                egui::Color32::from_rgb(255, 68, 68),
            ),
            (
                "GPU",
                t.nvidia
                    .as_ref()
                    .map(|g| format!("{} °C", g.temperature_c))
                    .unwrap_or_else(|| "--".into()),
                t.nvidia
                    .as_ref()
                    .map(|g| format!("{} % · {:.1} W", g.utilization_percent, g.power_watts))
                    .unwrap_or_else(|| "not visible".into()),
                egui::Color32::from_rgb(120, 200, 140),
            ),
            (
                "Fans",
                format!(
                    "{} / {}",
                    t.cpu_fan_rpm.unwrap_or(0),
                    t.gpu_fan_rpm.unwrap_or(0)
                ),
                "CPU / GPU RPM (EC)".into(),
                egui::Color32::from_rgb(100, 170, 235),
            ),
            (
                "Memory",
                t.ram_used_gb
                    .map(|u| format!("{u:.1} GB"))
                    .unwrap_or_else(|| "--".into()),
                t.ram_total_gb
                    .map(|t| format!("of {t:.1} GB"))
                    .unwrap_or_else(|| "unavailable".into()),
                accent,
            ),
        ];
        let avail = ui.available_width();
        let gap = 10.0;
        let cw = ((avail - 3.0 * gap) / 4.0).max(160.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            for (i, (label, value, sub, color)) in cards.iter().enumerate() {
                let (r, _) = ui.allocate_exact_size(egui::vec2(cw, 100.0), egui::Sense::hover());
                let p = ui.painter_at(r);
                p.rect_filled(r, 8.0, card_bg());
                p.rect_stroke(
                    r.shrink(0.5),
                    8.0,
                    egui::Stroke::new(1.0, egui::Color32::from_rgb(44, 44, 50)),
                );
                p.text(
                    egui::pos2(r.left() + 16.0, r.top() + 18.0),
                    egui::Align2::LEFT_CENTER,
                    *label,
                    egui::FontId::proportional(11.5),
                    text_muted(),
                );
                p.text(
                    egui::pos2(r.left() + 16.0, r.top() + 47.0),
                    egui::Align2::LEFT_CENTER,
                    value,
                    egui::FontId::proportional(23.0),
                    *color,
                );
                p.text(
                    egui::pos2(r.left() + 16.0, r.bottom() - 18.0),
                    egui::Align2::LEFT_CENTER,
                    sub,
                    egui::FontId::proportional(11.0),
                    text_muted(),
                );
                if i < 3 {
                    ui.add_space(gap);
                }
            }
        });
        ui.add_space(12.0);
        // second row: battery / storage / profile / uptime-ish summary
        let summary: [(String, String, String); 4] = [
            (
                "Battery".into(),
                t.battery
                    .as_ref()
                    .map(|b| format!("{} %", b.capacity_percent))
                    .unwrap_or_else(|| "--".into()),
                t.battery
                    .as_ref()
                    .map(|b| {
                        if b.ac_online {
                            "AC connected"
                        } else {
                            "on battery"
                        }
                    })
                    .unwrap_or("no battery")
                    .to_string(),
            ),
            (
                "Storage".into(),
                t.storage_used_gb
                    .map(|u| format!("{u:.1} GB"))
                    .unwrap_or_else(|| "--".into()),
                t.storage_total_gb
                    .map(|tot| format!("of {tot:.1} GB"))
                    .unwrap_or_else(|| "unavailable".into()),
            ),
            (
                "Profile".into(),
                t.profile.clone().unwrap_or_else(|| "--".into()),
                "platform_profile".to_string(),
            ),
            (
                "Chassis".into(),
                self.hero_model.clone(),
                "detected model".to_string(),
            ),
        ];
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            for (i, (label, value, sub)) in summary.iter().enumerate() {
                let (r, _) = ui.allocate_exact_size(egui::vec2(cw, 72.0), egui::Sense::hover());
                let p = ui.painter_at(r);
                p.rect_filled(r, 8.0, card_bg2());
                p.text(
                    egui::pos2(r.left() + 16.0, r.top() + 16.0),
                    egui::Align2::LEFT_CENTER,
                    label,
                    egui::FontId::proportional(10.5),
                    text_muted(),
                );
                p.text(
                    egui::pos2(r.left() + 16.0, r.top() + 42.0),
                    egui::Align2::LEFT_CENTER,
                    value,
                    egui::FontId::proportional(16.0),
                    text_main(),
                );
                p.text(
                    egui::pos2(r.left() + 16.0, r.bottom() - 12.0),
                    egui::Align2::LEFT_CENTER,
                    sub,
                    egui::FontId::proportional(10.0),
                    text_muted(),
                );
                if i < 3 {
                    ui.add_space(gap);
                }
            }
        });
        ui.add_space(14.0);
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Top processes by resident memory").size(14.5));
            info_icon(
                ui,
                "Processes are grouped by executable name; click a group to expand its PIDs.                  Resident memory is summed per group.",
            );
        });
        ui.add_space(8.0);
        let rows = read_proc_rows(15);
        type RGroup = (Vec<i32>, u64);
        let mut groups: std::collections::BTreeMap<String, RGroup> =
            std::collections::BTreeMap::new();
        for r in &rows {
            let g = groups.entry(r.comm.clone()).or_insert((Vec::new(), 0));
            g.0.push(r.pid);
            g.1 += r.rss_mb;
        }
        let mut list: Vec<(String, RGroup)> = groups.into_iter().collect();
        list.sort_by_key(|b| std::cmp::Reverse(b.1 .1));
        let details: std::collections::HashMap<i32, (String, u64)> = rows
            .iter()
            .map(|r| (r.pid, (r.comm.clone(), r.rss_mb)))
            .collect();
        egui::Frame::none()
            .fill(card_surface())
            .rounding(10.0)
            .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(52, 52, 58)))
            .shadow(egui::epaint::Shadow {
                offset: egui::vec2(0.0, 3.0),
                blur: 10.0,
                spread: 0.0,
                color: egui::Color32::from_black_alpha(55),
            })
            .inner_margin(14.0)
            .show(ui, |ui| {
                let full_w = ui.available_width();
                let (head, _) =
                    ui.allocate_exact_size(egui::vec2(full_w, 26.0), egui::Sense::hover());
                let hp = ui.painter_at(head);
                hp.text(
                    egui::pos2(head.left() + 44.0, head.center().y),
                    egui::Align2::LEFT_CENTER,
                    "Process",
                    egui::FontId::proportional(11.0),
                    text_muted(),
                );
                hp.text(
                    egui::pos2(head.right() - 16.0, head.center().y),
                    egui::Align2::RIGHT_CENTER,
                    "Resident memory",
                    egui::FontId::proportional(11.0),
                    text_muted(),
                );
                for (gi, (name, (pids, rss))) in list.iter().enumerate() {
                    let expanded = self.res_expanded.contains(name);
                    let (rect, resp) =
                        ui.allocate_exact_size(egui::vec2(full_w, 40.0), egui::Sense::click());
                    let p = ui.painter_at(rect);
                    if gi % 2 == 1 {
                        p.rect_filled(
                            rect,
                            0.0,
                            egui::Color32::from_rgba_unmultiplied(255, 255, 255, 5),
                        );
                    }
                    let cx = rect.left() + 16.0;
                    let cy = rect.center().y;
                    let col = text_muted();
                    if pids.len() > 1 {
                        if expanded {
                            p.add(egui::Shape::line(
                                vec![
                                    egui::pos2(cx - 4.0, cy - 1.0),
                                    egui::pos2(cx, cy + 3.0),
                                    egui::pos2(cx + 4.0, cy - 1.0),
                                ],
                                egui::Stroke::new(1.3, col),
                            ));
                        } else {
                            p.add(egui::Shape::line(
                                vec![
                                    egui::pos2(cx - 1.0, cy - 4.0),
                                    egui::pos2(cx + 3.0, cy),
                                    egui::pos2(cx - 1.0, cy + 4.0),
                                ],
                                egui::Stroke::new(1.3, col),
                            ));
                        }
                    }
                    let label = if pids.len() > 1 {
                        format!("{}  ({} processes)", name, pids.len())
                    } else {
                        format!("{}  ({})", name, pids[0])
                    };
                    p.text(
                        egui::pos2(rect.left() + 44.0, cy),
                        egui::Align2::LEFT_CENTER,
                        label,
                        egui::FontId::proportional(12.5),
                        text_main(),
                    );
                    p.text(
                        egui::pos2(rect.right() - 16.0, cy),
                        egui::Align2::RIGHT_CENTER,
                        format!("{} MB", rss),
                        egui::FontId::proportional(12.0),
                        text_muted(),
                    );
                    if resp.clicked() && pids.len() > 1 {
                        if expanded {
                            self.res_expanded.remove(name);
                        } else {
                            self.res_expanded.insert(name.clone());
                        }
                    }
                    if expanded {
                        for (pi, pid) in pids.iter().enumerate() {
                            if let Some((nm, rss)) = details.get(pid) {
                                let (sr, _) = ui.allocate_exact_size(
                                    egui::vec2(full_w, 28.0),
                                    egui::Sense::hover(),
                                );
                                let sp = ui.painter_at(sr);
                                if (gi + pi) % 2 == 0 {
                                    sp.rect_filled(
                                        sr,
                                        0.0,
                                        egui::Color32::from_rgba_unmultiplied(255, 255, 255, 3),
                                    );
                                }
                                sp.text(
                                    egui::pos2(sr.left() + 64.0, sr.center().y),
                                    egui::Align2::LEFT_CENTER,
                                    format!("{nm}  ({pid})"),
                                    egui::FontId::proportional(11.5),
                                    text_muted(),
                                );
                                sp.text(
                                    egui::pos2(sr.right() - 16.0, sr.center().y),
                                    egui::Align2::RIGHT_CENTER,
                                    format!("{rss} MB"),
                                    egui::FontId::proportional(11.0),
                                    text_muted(),
                                );
                            }
                        }
                    }
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
        let now = ui.ctx().input(|i| i.time) as f32;
        let rate = match self.rgb_speed {
            0xe1 => 0.55,
            0xf5 => 1.7,
            _ => 1.0,
        };
        ui.colored_label(
            text_muted(),
            "One physical lighting zone is present: the TUF Gaming F17 keyboard. Aura Sync applies effects to that zone.",
        );
        ui.add_space(10.0);
        let avail = ui.available_width();
        let wide = avail > 1100.0;
        let left_w = if wide {
            (avail * 0.62).clamp(520.0, 980.0)
        } else {
            avail
        };
        if wide {
            ui.horizontal_top(|ui| {
                ui.vertical(|ui| {
                    ui.set_width(left_w);
                    egui::Frame::none()
                        .fill(card_bg())
                        .rounding(10.0)
                        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(46, 46, 52)))
                        .inner_margin(16.0)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("TUF GAMING F17").size(15.0).strong().family(rog_family()));
                                ui.colored_label(egui::Color32::from_rgb(120, 200, 140), " connected");
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    ui.colored_label(text_muted(), "Keyboard · single zone");
                                });
                            });
                            ui.add_space(6.0);
                            self.lighting_preview(ui, now, rate);
                            ui.add_space(6.0);
                            ui.horizontal(|ui| {
                                ui.label(format!(
                                    "{} · #{:02X}{:02X}{:02X} · speed {} % · brightness {} %",
                                    Self::aura_mode_name(self.rgb_mode),
                                    self.rgb_color[0],
                                    self.rgb_color[1],
                                    self.rgb_color[2],
                                    Self::speed_percent(self.rgb_speed),
                                    self.telemetry
                                        .as_ref()
                                        .and_then(|t| t.kbd_brightness)
                                        .map(|b| b * 100 / 3)
                                        .unwrap_or(0)
                                ));
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if ui.button("Open Aura Effects >").clicked() {
                                        self.aura_sub = AuraTab::Effects;
                                    }
                                });
                            });
                        });
                });
                ui.add_space(14.0);
                ui.vertical(|ui| {
                    ui.set_width((ui.available_width() - 4.0).max(280.0));
                    egui::Frame::none()
                        .fill(card_bg2())
                        .rounding(10.0)
                        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(46, 46, 52)))
                        .inner_margin(16.0)
                        .show(ui, |ui| {
                            ui.label(egui::RichText::new("Sync status").size(14.0).strong());
                            ui.add_space(8.0);
                            for (label, value) in [
                                ("Selected devices", "1 / 1"),
                                ("Scope", "Keyboard zone"),
                                ("Control channel", "WMNB DEVS 0x00100056"),
                                ("Firmware effects", "11"),
                            ] {
                                ui.horizontal(|ui| {
                                    ui.colored_label(text_muted(), label);
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        ui.label(value);
                                    });
                                });
                                ui.add_space(2.0);
                                ui.separator();
                            }
                            ui.add_space(6.0);
                            if ui.add_sized([ui.available_width(), 32.0], egui::Button::new("Apply current effect")).clicked() {
                                self.apply_rgb(self.rgb_mode);
                            }
                            if ui.add_sized([ui.available_width(), 28.0], egui::Button::new("Choose a different effect")).clicked() {
                                self.aura_sub = AuraTab::Effects;
                            }
                            ui.add_space(4.0);
                            ui.colored_label(
                                text_muted(),
                                "Aura Sync here means the keyboard zone. There are no external ASUS Aura devices paired.",
                            );
                        });
                });
            });
        } else {
            egui::Frame::none()
                .fill(card_bg())
                .rounding(10.0)
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(46, 46, 52)))
                .inner_margin(14.0)
                .show(ui, |ui| {
                    ui.label(
                        egui::RichText::new("TUF GAMING F17 · keyboard")
                            .size(15.0)
                            .strong()
                            .family(rog_family()),
                    );
                    self.lighting_preview(ui, now, rate);
                    if ui.button("Open Aura Effects").clicked() {
                        self.aura_sub = AuraTab::Effects;
                    }
                });
        }

        ui.add_space(14.0);
        section_label(ui, "How Aura Sync works on this model");
        ui.add_space(8.0);
        egui::Frame::none()
            .fill(card_bg2())
            .rounding(8.0)
            .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(42, 42, 48)))
            .inner_margin(16.0)
            .show(ui, |ui| {
                let w = (ui.available_width() - 48.0) / 3.0;
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    for (i, (title, text)) in [
                        (
                            "1 · Pick an effect",
                            "Aura Effects drives the 11 firmware effects, speed and colour.",
                        ),
                        (
                            "2 · One zone",
                            "The keyboard is the only addressable lighting zone on this model.",
                        ),
                        (
                            "3 · Applied live",
                            "Writes go through the kernel asus-wmi RGB channel and are audited.",
                        ),
                    ]
                    .iter()
                    .enumerate()
                    {
                        ui.vertical(|ui| {
                            ui.set_width(w);
                            ui.label(egui::RichText::new(*title).size(12.5).strong());
                            ui.colored_label(text_muted(), *text);
                        });
                        if i < 2 {
                            ui.add_space(24.0);
                        }
                    }
                });
            });
        ui.add_space(8.0);
        egui::Frame::none()
            .fill(card_bg())
            .rounding(8.0)
            .inner_margin(12.0)
            .show(ui, |ui| {
                ui.colored_label(
                    egui::Color32::from_rgb(170, 155, 125),
                    "Not available on this laptop (no dead controls):",
                );
                for line in [
                    "Aura Performance Mode — Windows rendering service; no Linux equivalent.",
                    "Hue Bridge discovery — needs the Windows network integration and an account.",
                    "Multi-device sync — this machine has one keyboard lighting zone.",
                ] {
                    ui.colored_label(text_muted(), format!("· {line}"));
                }
            });
        let _ = accent;
    }

    fn ui_ai_aura(&mut self, ui: &mut egui::Ui) {
        let accent = self.theme.accent();
        let w = ui.available_width();
        let (rect, _) = ui.allocate_exact_size(egui::vec2(w, 260.0), egui::Sense::hover());
        let p = ui.painter_at(rect);
        let mut mesh = egui::Mesh::default();
        mesh.colored_vertex(rect.left_top(), egui::Color32::from_rgb(52, 30, 10));
        mesh.colored_vertex(rect.right_top(), egui::Color32::from_rgb(32, 20, 44));
        mesh.colored_vertex(rect.right_bottom(), egui::Color32::from_rgb(12, 28, 20));
        mesh.colored_vertex(rect.left_bottom(), egui::Color32::from_rgb(46, 30, 12));
        mesh.add_triangle(0, 1, 2);
        mesh.add_triangle(0, 2, 3);
        p.add(egui::Shape::mesh(mesh));
        // soft light blobs
        for (cx, cy, r, c) in [
            (0.22, 0.45, 120.0, (235, 140, 50)),
            (0.72, 0.35, 90.0, (120, 80, 220)),
            (0.55, 0.75, 110.0, (60, 180, 160)),
        ] {
            p.circle_filled(
                egui::pos2(
                    rect.left() + rect.width() * cx,
                    rect.top() + rect.height() * cy,
                ),
                r,
                egui::Color32::from_rgba_unmultiplied(c.0, c.1, c.2, 22),
            );
        }
        p.text(
            egui::pos2(rect.left() + 32.0, rect.top() + 70.0),
            egui::Align2::LEFT_CENTER,
            "AI Aura Lighting",
            egui::FontId::proportional(34.0),
            egui::Color32::WHITE,
        );
        p.text(
            egui::pos2(rect.left() + 32.0, rect.top() + 112.0),
            egui::Align2::LEFT_CENTER,
            "Turn inspiration into live lighting. The ASUS cloud generator is unavailable;\nthese local effects run on the keyboard today.",
            egui::FontId::proportional(14.0),
            egui::Color32::from_rgb(225, 225, 230),
        );
        ui.add_space(12.0);
        section_label(ui, "Local Aura effects");
        ui.add_space(8.0);
        let cards: [(&str, &str, u8); 3] = [
            (
                "Smart",
                "Colour follows the CPU temperature; brighter and hotter red as the package warms.",
                3,
            ),
            (
                "Adaptive",
                "Samples the average screen colour and streams it to the keyboard.",
                7,
            ),
            (
                "Music",
                "Audio-reactive colour from the default PipeWire output.",
                2,
            ),
        ];
        let gap = 12.0;
        let cw = ((ui.available_width() - 2.0 * gap) / 3.0).max(220.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            for (i, (name, desc, icon)) in cards.iter().enumerate() {
                let active = if *name == "Smart" {
                    self.soft_effect == Some(music::SMART_EFFECT_ID)
                } else if *name == "Adaptive" {
                    self.soft_effect == Some(music::ADAPTIVE_EFFECT_ID)
                } else {
                    self.soft_effect == Some(music::MUSIC_EFFECT_ID)
                };
                let (r, resp) = ui.allocate_exact_size(egui::vec2(cw, 132.0), egui::Sense::click());
                let p = ui.painter_at(r);
                p.rect_filled(r, 9.0, if active { card_bg2() } else { card_bg() });
                p.rect_stroke(
                    r.shrink(0.5),
                    9.0,
                    egui::Stroke::new(
                        if active { 1.6 } else { 1.0 },
                        if active {
                            accent
                        } else {
                            egui::Color32::from_rgb(46, 46, 52)
                        },
                    ),
                );
                nav_icon(
                    &p,
                    egui::pos2(r.left() + 30.0, r.top() + 34.0),
                    *icon,
                    if active {
                        accent
                    } else {
                        egui::Color32::from_rgb(210, 210, 220)
                    },
                );
                p.text(
                    egui::pos2(r.left() + 52.0, r.top() + 28.0),
                    egui::Align2::LEFT_CENTER,
                    *name,
                    egui::FontId::proportional(15.0),
                    text_main(),
                );
                p.text(
                    egui::pos2(r.left() + 16.0, r.top() + 60.0),
                    egui::Align2::LEFT_TOP,
                    *desc,
                    egui::FontId::proportional(11.5),
                    text_muted(),
                );
                p.text(
                    egui::pos2(r.left() + 16.0, r.bottom() - 16.0),
                    egui::Align2::LEFT_CENTER,
                    if active { "RUNNING" } else { "click to start" },
                    egui::FontId::proportional(10.5),
                    if active {
                        egui::Color32::from_rgb(120, 200, 140)
                    } else {
                        text_muted()
                    },
                );
                if resp.clicked() {
                    match *name {
                        "Smart" => self.start_smart(),
                        "Adaptive" => self.start_adaptive(),
                        _ => self.start_music(),
                    }
                }
                if i < 2 {
                    ui.add_space(gap);
                }
            }
        });
        ui.add_space(12.0);
        egui::Frame::none()
            .fill(card_bg())
            .rounding(8.0)
            .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(44, 44, 50)))
            .inner_margin(14.0)
            .show(ui, |ui| {
                ui.colored_label(
                    egui::Color32::from_rgb(170, 155, 125),
                    "ASUS cloud features unavailable (no dead buttons):",
                );
                for line in [
                    "Create new — needs the ASUS AI service, sign-in and cloud generation.",
                    "Lighting Gallery — ASUS cloud asset catalogue.",
                    "AI scene recognition — Windows service component.",
                ] {
                    ui.colored_label(text_muted(), format!("· {line}"));
                }
            });
    }

    fn ui_aura_wallpaper(&mut self, ui: &mut egui::Ui) {
        let accent = self.theme.accent();
        ui.horizontal(|ui| {
            ui.colored_label(
                egui::Color32::from_rgb(120, 200, 140),
                format!("● {} local wallpapers", self.wallpapers.len()),
            );
            ui.colored_label(text_muted(), "· XFCE desktop integration");
        });
        ui.colored_label(
            text_muted(),
            "Local gallery with native XFCE application. No cloud download, no account.",
        );
        ui.add_space(10.0);
        if self.wallpapers.is_empty() {
            ui.colored_label(
                egui::Color32::from_rgb(220, 160, 90),
                "No wallpapers found. Add images under assets/wallpapers/ or the vendor reference wallpapers folder.",
            );
            return;
        }
        self.wallpaper_idx = self.wallpaper_idx.min(self.wallpapers.len() - 1);
        let idx = self.wallpaper_idx;
        let path = self.wallpapers[idx].0.clone();
        let tex = self.wallpapers[idx].1.clone();
        let wide = ui.available_width() > 1150.0;

        let draw_preview =
            |ui: &mut egui::Ui, w: f32, h: f32, tex: Option<&egui::TextureHandle>| {
                let (rect, _) = ui.allocate_exact_size(egui::vec2(w, h), egui::Sense::hover());
                let p = ui.painter_at(rect);
                p.rect_filled(rect, 10.0, egui::Color32::from_rgb(12, 12, 15));
                if let Some(tex) = tex {
                    let size = tex.size_vec2();
                    let scale = (rect.width() / size.x).max(rect.height() / size.y);
                    let drawn = egui::vec2(size.x * scale, size.y * scale);
                    let u = (rect.width() / drawn.x).clamp(0.0, 1.0);
                    let v = (rect.height() / drawn.y).clamp(0.0, 1.0);
                    let uv = egui::Rect::from_min_max(
                        egui::pos2((1.0 - u) / 2.0, (1.0 - v) / 2.0),
                        egui::pos2((1.0 + u) / 2.0, (1.0 + v) / 2.0),
                    );
                    p.image(tex.id(), rect.shrink(3.0), uv, egui::Color32::WHITE);
                }
                p.rect_stroke(
                    rect.shrink(0.5),
                    10.0,
                    egui::Stroke::new(1.0, egui::Color32::from_rgb(52, 52, 58)),
                );
            };

        if wide {
            ui.horizontal_top(|ui| {
                let preview_w = (ui.available_width() * 0.62).min(900.0);
                draw_preview(ui, preview_w, preview_w * 9.0 / 16.0, tex.as_ref());
                ui.add_space(16.0);
                ui.vertical(|ui| {
                    ui.set_width((ui.available_width() - 8.0).max(280.0));
                    egui::Frame::none()
                        .fill(card_bg())
                        .rounding(8.0)
                        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(44, 44, 48)))
                        .inner_margin(16.0)
                        .show(ui, |ui| {
                            ui.label(egui::RichText::new("Selected wallpaper").size(12.0).color(text_muted()));
                            let name = std::path::Path::new(&path)
                                .file_name()
                                .map(|n| n.to_string_lossy().into_owned())
                                .unwrap_or_else(|| path.clone());
                            ui.label(egui::RichText::new(name).size(18.0).strong());
                            let meta = std::fs::metadata(&path).ok();
                            if let Some(m) = meta {
                                ui.colored_label(
                                    text_muted(),
                                    format!("{:.1} MB · {}", m.len() as f64 / 1_048_576.0, path),
                                );
                            }
                            ui.add_space(10.0);
                            if ui
                                .add_sized(
                                    [ui.available_width(), 34.0],
                                    egui::Button::new("Set as XFCE wallpaper"),
                                )
                                .on_hover_text("xfconf-query sets last-image for every XFCE workspace")
                                .clicked()
                            {
                                match set_xfce_wallpaper(&path) {
                                    Ok(n) => self.report(
                                        format!("Wallpaper set on {n} XFCE workspace(s)"),
                                        false,
                                    ),
                                    Err(e) => self.report(format!("Wallpaper: {e}"), true),
                                }
                            }
                            ui.add_space(6.0);
                            ui.horizontal(|ui| {
                                for (label, style) in [("Zoom", 5u32), ("Fit", 4), ("Center", 1)] {
                                    if ui.small_button(label).clicked() {
                                        match set_xfce_wallpaper_style(style) {
                                            Ok(n) => self.report(
                                                format!("Wallpaper style set on {n} workspace(s)"),
                                                false,
                                            ),
                                            Err(e) => self.report(format!("Wallpaper style: {e}"), true),
                                        }
                                    }
                                }
                            });
                            if ui
                                .add_sized([ui.available_width(), 28.0], egui::Button::new("Open containing folder"))
                                .clicked()
                            {
                                let dir = std::path::Path::new(&path)
                                    .parent()
                                    .map(|p| p.to_string_lossy().into_owned())
                                    .unwrap_or_else(|| ".".into());
                                let _ = std::process::Command::new("xdg-open").arg(dir).spawn();
                            }
                            ui.add_space(10.0);
                            ui.separator();
                            ui.add_space(6.0);
                            ui.colored_label(
                                text_muted(),
                                "The gallery and XFCE application are native. Per-display and AC/battery wallpaper switching stays with XFCE power management.",
                            );
                        });
                });
            });
        } else {
            draw_preview(
                ui,
                ui.available_width(),
                (ui.available_width() * 9.0 / 16.0).min(420.0),
                tex.as_ref(),
            );
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button("Set as XFCE wallpaper").clicked() {
                    match set_xfce_wallpaper(&path) {
                        Ok(n) => {
                            self.report(format!("Wallpaper set on {n} XFCE workspace(s)"), false)
                        }
                        Err(e) => self.report(format!("Wallpaper: {e}"), true),
                    }
                }
                ui.colored_label(text_muted(), path.clone());
            });
        }

        ui.add_space(14.0);
        section_label(ui, "Gallery");
        ui.add_space(8.0);
        let thumbs: Vec<Option<egui::TextureHandle>> =
            self.wallpapers.iter().map(|(_, t)| t.clone()).collect();
        let avail = ui.available_width();
        let cols = if avail > 1100.0 {
            5
        } else if avail > 800.0 {
            4
        } else {
            3
        };
        let gap = 10.0;
        let card_w = ((avail - gap * (cols as f32 - 1.0)) / cols as f32).max(140.0);
        let card_h = card_w * 9.0 / 16.0;
        for row in 0..thumbs.len().div_ceil(cols) {
            ui.horizontal(|ui| {
                for col in 0..cols {
                    let i = row * cols + col;
                    let Some(tex) = thumbs.get(i) else { continue };
                    let selected = i == idx;
                    let (rect, resp) =
                        ui.allocate_exact_size(egui::vec2(card_w, card_h), egui::Sense::click());
                    let p = ui.painter_at(rect);
                    p.rect_filled(rect, 7.0, card_bg());
                    if let Some(tex) = tex {
                        let size = tex.size_vec2();
                        let scale = (rect.width() / size.x).max(rect.height() / size.y);
                        let drawn = egui::vec2(size.x * scale, size.y * scale);
                        let u = (rect.width() / drawn.x).clamp(0.0, 1.0);
                        let v = (rect.height() / drawn.y).clamp(0.0, 1.0);
                        let uv = egui::Rect::from_min_max(
                            egui::pos2((1.0 - u) / 2.0, (1.0 - v) / 2.0),
                            egui::pos2((1.0 + u) / 2.0, (1.0 + v) / 2.0),
                        );
                        p.image(tex.id(), rect.shrink(3.0), uv, egui::Color32::WHITE);
                    }
                    p.rect_stroke(
                        rect.shrink(0.5),
                        7.0,
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
                    ui.add_space(gap);
                }
            });
            ui.add_space(8.0);
        }
    }

    fn ui_scenario_profiles(&mut self, ui: &mut egui::Ui) {
        let accent = self.theme.accent();
        ui.colored_label(
            text_muted(),
            "Apply a performance mode automatically while a game or application runs, then restore the previous mode when it exits.",
        );
        ui.add_space(6.0);
        let watching = self.active_game_rule.is_some();
        ui.horizontal(|ui| {
            let (dot, _) = ui.allocate_exact_size(egui::vec2(8.0, 8.0), egui::Sense::hover());
            ui.painter_at(dot).circle_filled(
                dot.center(),
                3.5,
                if watching {
                    egui::Color32::from_rgb(120, 200, 140)
                } else {
                    egui::Color32::from_rgb(150, 150, 160)
                },
            );
            ui.colored_label(
                if watching {
                    egui::Color32::from_rgb(120, 200, 140)
                } else {
                    text_muted()
                },
                if watching {
                    format!(
                        "Active profile: {} — matched process is running",
                        self.active_game_rule.as_deref().unwrap_or("•")
                    )
                } else {
                    "Watcher running — checks /proc every 4 s".into()
                },
            );
        });
        ui.add_space(12.0);
        let current_mode = self
            .telemetry
            .as_ref()
            .and_then(|t| t.profile.clone())
            .unwrap_or_else(|| "--".into());
        let stats = [
            (
                "Rules",
                format!("{}", self.game_rules.len()),
                "process → mode",
            ),
            (
                "Watcher",
                if self.active_game_rule.is_some() {
                    "Active"
                } else {
                    "Running"
                }
                .into(),
                "checks every 4 s",
            ),
            (
                "Current mode",
                match current_mode.as_str() {
                    "quiet" => "Silent",
                    "balanced" => "Balanced",
                    "performance" => "Turbo",
                    _ => "Unknown",
                }
                .into(),
                "platform_profile",
            ),
        ];
        let avail = ui.available_width();
        let gap = 10.0;
        let stat_w = ((avail - 2.0 * gap) / 3.0).max(220.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            for (i, (label, value, sub)) in stats.iter().enumerate() {
                let (rect, _) =
                    ui.allocate_exact_size(egui::vec2(stat_w, 82.0), egui::Sense::hover());
                let p = ui.painter_at(rect);
                p.rect_filled(rect, 8.0, card_bg());
                p.rect_stroke(
                    rect,
                    8.0,
                    egui::Stroke::new(1.0, egui::Color32::from_rgb(44, 44, 50)),
                );
                p.text(
                    egui::pos2(rect.left() + 16.0, rect.top() + 18.0),
                    egui::Align2::LEFT_CENTER,
                    *label,
                    egui::FontId::proportional(11.5),
                    text_muted(),
                );
                p.text(
                    egui::pos2(rect.left() + 16.0, rect.top() + 42.0),
                    egui::Align2::LEFT_CENTER,
                    value,
                    egui::FontId::proportional(20.0),
                    text_main(),
                );
                p.text(
                    egui::pos2(rect.left() + 16.0, rect.bottom() - 14.0),
                    egui::Align2::LEFT_CENTER,
                    *sub,
                    egui::FontId::proportional(10.5),
                    text_muted(),
                );
                if i + 1 < stats.len() {
                    ui.add_space(gap);
                }
            }
        });
        ui.add_space(14.0);
        section_label(ui, "Gaming profiles");
        ui.add_space(8.0);

        if self.game_rules.is_empty() {
            egui::Frame::none()
                .fill(card_bg())
                .rounding(8.0)
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(44, 44, 48)))
                .inner_margin(18.0)
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    ui.horizontal(|ui| {
                        let (r, _) = ui.allocate_exact_size(egui::vec2(44.0, 44.0), egui::Sense::hover());
                        ui.painter_at(r).circle_filled(
                            r.center(),
                            22.0,
                            egui::Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), 20),
                        );
                        nav_icon(&ui.painter_at(r), r.center(), 4, accent);
                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new("No gaming rules yet").size(15.0).strong());
                            ui.colored_label(
                                text_muted(),
                                "Add a process name below. When it appears in /proc, its profile is applied automatically.",
                            );
                        });
                    });
                });
            ui.add_space(12.0);
        } else {
            let rules = self.game_rules.clone();
            for rule in &rules {
                let (rect, _) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width(), 58.0),
                    egui::Sense::hover(),
                );
                let p = ui.painter_at(rect);
                p.rect_filled(rect, 7.0, card_bg());
                p.rect_stroke(
                    rect,
                    7.0,
                    egui::Stroke::new(1.0, egui::Color32::from_rgb(42, 42, 48)),
                );
                let icon_c = egui::pos2(rect.left() + 30.0, rect.center().y);
                p.circle_filled(
                    icon_c,
                    16.0,
                    egui::Color32::from_rgba_unmultiplied(
                        accent.r(),
                        accent.g(),
                        accent.b(),
                        if rule.enabled { 30 } else { 12 },
                    ),
                );
                nav_icon(
                    &p,
                    icon_c,
                    4,
                    if rule.enabled {
                        accent
                    } else {
                        egui::Color32::from_rgb(130, 130, 140)
                    },
                );
                p.text(
                    egui::pos2(rect.left() + 56.0, rect.center().y - 9.0),
                    egui::Align2::LEFT_CENTER,
                    &rule.process,
                    egui::FontId::proportional(14.0),
                    text_main(),
                );
                p.text(
                    egui::pos2(rect.left() + 56.0, rect.center().y + 11.0),
                    egui::Align2::LEFT_CENTER,
                    if rule.enabled {
                        "enabled · matched by /proc/<pid>/comm"
                    } else {
                        "disabled"
                    },
                    egui::FontId::proportional(11.0),
                    text_muted(),
                );
                // profile badge
                let badge = egui::Rect::from_min_size(
                    egui::pos2(rect.right() - 190.0, rect.center().y - 12.0),
                    egui::vec2(92.0, 24.0),
                );
                p.rect_filled(
                    badge,
                    12.0,
                    egui::Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), 22),
                );
                p.rect_stroke(badge, 12.0, egui::Stroke::new(1.0, accent));
                p.text(
                    badge.center(),
                    egui::Align2::CENTER_CENTER,
                    rule.profile.to_uppercase(),
                    egui::FontId::proportional(10.5),
                    accent,
                );
                let mut enabled = rule.enabled;
                let chk = egui::Rect::from_min_size(
                    egui::pos2(rect.right() - 84.0, rect.center().y - 9.0),
                    egui::vec2(18.0, 18.0),
                );
                let chk_resp = ui.interact(
                    chk,
                    egui::Id::new(("rule-enable", &rule.process)),
                    egui::Sense::click(),
                );
                p.rect_stroke(
                    chk,
                    3.0,
                    egui::Stroke::new(
                        1.2,
                        if enabled {
                            accent
                        } else {
                            egui::Color32::from_rgb(90, 90, 100)
                        },
                    ),
                );
                if enabled {
                    p.add(egui::Shape::line(
                        vec![
                            egui::pos2(chk.left() + 4.0, chk.center().y),
                            egui::pos2(chk.center().x - 1.0, chk.bottom() - 5.0),
                            egui::pos2(chk.right() - 4.0, chk.top() + 5.0),
                        ],
                        egui::Stroke::new(1.8, accent),
                    ));
                }
                if chk_resp.clicked() {
                    enabled = !enabled;
                    let _ = games::set_enabled(&rule.process, enabled);
                    self.game_rules = games::load_all();
                }
                let del = egui::Rect::from_min_size(
                    egui::pos2(rect.right() - 42.0, rect.center().y - 12.0),
                    egui::vec2(24.0, 24.0),
                );
                let del_resp = ui.interact(
                    del,
                    egui::Id::new(("rule-del", &rule.process)),
                    egui::Sense::click(),
                );
                p.text(
                    del.center(),
                    egui::Align2::CENTER_CENTER,
                    "✕",
                    egui::FontId::proportional(15.0),
                    if del_resp.hovered() {
                        egui::Color32::from_rgb(255, 68, 68)
                    } else {
                        text_muted()
                    },
                );
                if del_resp.clicked() {
                    let _ = games::remove(&rule.process);
                    self.game_rules = games::load_all();
                }
                ui.add_space(8.0);
            }
        }

        ui.add_space(4.0);
        section_label(ui, "Add a rule");
        ui.add_space(8.0);
        egui::Frame::none()
            .fill(card_bg())
            .rounding(8.0)
            .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(42, 42, 48)))
            .inner_margin(14.0)
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.label("Process");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.game_input)
                            .desired_width(210.0)
                            .hint_text("e.g. cs2, eldenring, kdenlive"),
                    );
                    ui.add_space(8.0);
                    ui.label("Mode");
                    egui::ComboBox::from_id_salt("game_profile")
                        .selected_text(match self.game_profile.as_str() {
                            "quiet" => "Silent",
                            "balanced" => "Balanced",
                            _ => "Turbo",
                        })
                        .show_ui(ui, |ui| {
                            for p in games::RULE_PROFILES {
                                ui.selectable_value(
                                    &mut self.game_profile,
                                    p.to_string(),
                                    match p {
                                        "quiet" => "Silent",
                                        "balanced" => "Balanced",
                                        _ => "Turbo",
                                    },
                                );
                            }
                        });
                    if ui.button("＋ Add rule").clicked() {
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
                ui.add_space(4.0);
                ui.colored_label(
                    text_muted(),
                    "Steam/Wine games often show a wrapper name — use the exact name from `ps -eo comm` (max 15 characters).",
                );
            });
        ui.add_space(16.0);
        section_label(ui, "How matching works");
        ui.add_space(8.0);
        egui::Frame::none()
            .fill(card_bg2())
            .rounding(8.0)
            .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(42, 42, 48)))
            .inner_margin(16.0)
            .show(ui, |ui| {
                let w = (ui.available_width() - 2.0 * 24.0) / 3.0;
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    for (i, (title, text)) in [
                        (
                            "1 · Add a process",
                            "Use the name from /proc/<pid>/comm. The match is case-insensitive.",
                        ),
                        (
                            "2 · Launch it",
                            "The watcher checks every 4 seconds and applies that rule's mode.",
                        ),
                        (
                            "3 · Exit restores",
                            "The mode from before the game started is put back automatically.",
                        ),
                    ]
                    .iter()
                    .enumerate()
                    {
                        ui.vertical(|ui| {
                            ui.set_width(w);
                            ui.label(egui::RichText::new(*title).size(13.0).strong());
                            ui.colored_label(text_muted(), *text);
                        });
                        if i < 2 {
                            ui.add_space(24.0);
                        }
                    }
                });
            });
    }

    fn ui_gamevisual(&mut self, ui: &mut egui::Ui) {
        let accent = self.theme.accent();
        let output = self.display_output.clone();
        let refresh = output
            .as_deref()
            .and_then(display_refresh_rate)
            .unwrap_or_else(|| "--".into());
        ui.colored_label(
            text_muted(),
            "Display colour profiles applied through X11 gamma, plus panel overdrive. Gamma is session-level and reversible with Reset.",
        );
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.colored_label(
                egui::Color32::from_rgb(120, 200, 140),
                format!(
                    "● {}  ·  {} Hz",
                    output
                        .as_deref()
                        .unwrap_or("no X11 output (headless session)"),
                    refresh
                ),
            );
            if output.is_some() && ui.small_button("Reset colour").clicked() {
                self.display_mode = 0;
                if let Err(e) = apply_display_gamma(output.as_deref().unwrap(), [1.0, 1.0, 1.0]) {
                    self.report(format!("display reset: {e}"), true);
                } else {
                    self.report("display colour reset".into(), false);
                }
            }
        });
        ui.add_space(10.0);
        let gamma = DISPLAY_MODES[self.display_mode].2;
        let (banner, _) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), 190.0),
            egui::Sense::hover(),
        );
        let bp = ui.painter_at(banner);
        draw_display_scene(&bp, banner, gamma);
        bp.rect_stroke(
            banner,
            10.0,
            egui::Stroke::new(1.0, egui::Color32::from_rgb(52, 52, 58)),
        );
        bp.text(
            egui::pos2(banner.left() + 24.0, banner.top() + 26.0),
            egui::Align2::LEFT_CENTER,
            format!("GameVisual · {}", DISPLAY_MODES[self.display_mode].0),
            egui::FontId::proportional(22.0),
            egui::Color32::WHITE,
        );
        bp.text(
            egui::pos2(banner.left() + 24.0, banner.top() + 50.0),
            egui::Align2::LEFT_CENTER,
            "Live gamma preview — the selected profile is applied to the X11 output.",
            egui::FontId::proportional(12.0),
            egui::Color32::from_rgba_unmultiplied(235, 235, 240, 210),
        );
        ui.add_space(12.0);
        section_label(ui, "Display modes");
        ui.add_space(8.0);

        let avail = ui.available_width();
        let cols = if avail > 1300.0 {
            4
        } else if avail > 900.0 {
            3
        } else {
            2
        };
        let gap = 10.0;
        let card_w = ((avail - gap * (cols as f32 - 1.0)) / cols as f32).max(180.0);
        let mut selected = None;
        for row in 0..DISPLAY_MODES.len().div_ceil(cols) {
            ui.horizontal(|ui| {
                for col in 0..cols {
                    let idx = row * cols + col;
                    let Some((name, desc, gamma)) = DISPLAY_MODES.get(idx).copied() else {
                        continue;
                    };
                    let active = self.display_mode == idx;
                    let (rect, resp) =
                        ui.allocate_exact_size(egui::vec2(card_w, 142.0), egui::Sense::click());
                    let p = ui.painter_at(rect);
                    p.rect_filled(rect, 8.0, if active { card_bg2() } else { card_bg() });
                    p.rect_stroke(
                        rect.shrink(0.5),
                        8.0,
                        egui::Stroke::new(
                            if active { 1.6 } else { 1.0 },
                            if active {
                                accent
                            } else {
                                egui::Color32::from_rgb(44, 44, 50)
                            },
                        ),
                    );
                    let preview = egui::Rect::from_min_size(
                        egui::pos2(rect.left() + 10.0, rect.top() + 10.0),
                        egui::vec2(rect.width() - 20.0, 66.0),
                    );
                    let clip = p.with_clip_rect(preview);
                    draw_display_scene(&clip, preview, gamma);
                    p.text(
                        egui::pos2(rect.left() + 12.0, rect.top() + 94.0),
                        egui::Align2::LEFT_CENTER,
                        name,
                        egui::FontId::proportional(14.0),
                        text_main(),
                    );
                    p.text(
                        egui::pos2(rect.left() + 12.0, rect.top() + 112.0),
                        egui::Align2::LEFT_CENTER,
                        desc,
                        egui::FontId::proportional(11.0),
                        text_muted(),
                    );
                    if active {
                        p.circle_filled(
                            egui::pos2(rect.right() - 16.0, rect.top() + 16.0),
                            4.0,
                            accent,
                        );
                    }
                    if resp.clicked() {
                        selected = Some(idx);
                    }
                    ui.add_space(gap);
                }
            });
            ui.add_space(8.0);
        }
        if let Some(idx) = selected {
            self.display_mode = idx;
            if let Some(out) = output.as_deref() {
                match apply_display_gamma(out, DISPLAY_MODES[idx].2) {
                    Ok(()) => {
                        self.report(format!("display mode -> {}", DISPLAY_MODES[idx].0), false)
                    }
                    Err(e) => self.report(format!("display mode: {e}"), true),
                }
            } else {
                self.report(
                    "no X11 output detected — cannot apply display mode".into(),
                    true,
                );
            }
        }

        ui.add_space(10.0);
        section_label(ui, "GPU & panel");
        ui.add_space(8.0);
        let t = self.telemetry.clone();
        let gpu = t.as_ref().and_then(|t| t.nvidia.clone());
        let gpu_value = gpu
            .as_ref()
            .map(|g| {
                format!(
                    "{} °C · {} % · {:.1} W",
                    g.temperature_c, g.utilization_percent, g.power_watts
                )
            })
            .unwrap_or_else(|| "dGPU not visible".into());
        let panel_od = t.as_ref().and_then(|t| t.panel_od).unwrap_or(false);
        ui.horizontal(|ui| {
            let (card, resp) =
                ui.allocate_exact_size(egui::vec2(280.0, 96.0), egui::Sense::hover());
            let p = ui.painter_at(card);
            p.rect_filled(card, 8.0, card_bg());
            p.rect_stroke(
                card.shrink(0.5),
                8.0,
                egui::Stroke::new(1.0, egui::Color32::from_rgb(44, 44, 50)),
            );
            p.text(
                egui::pos2(card.left() + 16.0, card.top() + 22.0),
                egui::Align2::LEFT_CENTER,
                "NVIDIA RTX 2050",
                egui::FontId::proportional(14.0),
                text_main(),
            );
            p.text(
                egui::pos2(card.left() + 16.0, card.top() + 48.0),
                egui::Align2::LEFT_CENTER,
                &gpu_value,
                egui::FontId::proportional(12.5),
                text_muted(),
            );
            p.text(
                egui::pos2(card.left() + 16.0, card.top() + 72.0),
                egui::Align2::LEFT_CENTER,
                "nvidia-smi telemetry · read-only",
                egui::FontId::proportional(11.0),
                text_muted(),
            );
            resp.on_hover_text(
                "GPU telemetry is read-only; power and clocks stay under the driver.",
            );
            ui.add_space(10.0);
            let (card2, resp2) =
                ui.allocate_exact_size(egui::vec2(280.0, 96.0), egui::Sense::click());
            let p = ui.painter_at(card2);
            p.rect_filled(card2, 8.0, card_bg());
            p.rect_stroke(
                card2.shrink(0.5),
                8.0,
                egui::Stroke::new(
                    if panel_od { 1.4 } else { 1.0 },
                    if panel_od {
                        accent
                    } else {
                        egui::Color32::from_rgb(44, 44, 50)
                    },
                ),
            );
            p.text(
                egui::pos2(card2.left() + 16.0, card2.top() + 22.0),
                egui::Align2::LEFT_CENTER,
                "Panel overdrive",
                egui::FontId::proportional(14.0),
                text_main(),
            );
            p.text(
                egui::pos2(card2.left() + 16.0, card2.top() + 48.0),
                egui::Align2::LEFT_CENTER,
                if panel_od {
                    "On · 144 Hz response optimisation"
                } else {
                    "Off"
                },
                egui::FontId::proportional(12.5),
                if panel_od { accent } else { text_muted() },
            );
            p.text(
                egui::pos2(card2.left() + 16.0, card2.top() + 72.0),
                egui::Align2::LEFT_CENTER,
                "Click to toggle (verified kernel attr)",
                egui::FontId::proportional(11.0),
                text_muted(),
            );
            if resp2.clicked() {
                match self.backend.set_panel_overdrive(!panel_od, true) {
                    Ok(()) => self.report(
                        format!("panel overdrive -> {}", if panel_od { "off" } else { "on" }),
                        false,
                    ),
                    Err(e) => self.report(format!("panel_od failed: {}", friendly_error(&e)), true),
                }
                self.poll();
            }
            ui.add_space(10.0);
            if let (Some(out), Some((mode, rates))) = (
                output.as_deref(),
                output.as_deref().and_then(display_mode_rates),
            ) {
                let current = display_refresh_rate(out).unwrap_or_else(|| "--".into());
                let (card3, _) =
                    ui.allocate_exact_size(egui::vec2(280.0, 96.0), egui::Sense::hover());
                let p = ui.painter_at(card3);
                p.rect_filled(card3, 8.0, card_bg());
                p.rect_stroke(
                    card3.shrink(0.5),
                    8.0,
                    egui::Stroke::new(1.0, egui::Color32::from_rgb(44, 44, 50)),
                );
                p.text(
                    egui::pos2(card3.left() + 16.0, card3.top() + 20.0),
                    egui::Align2::LEFT_CENTER,
                    "Refresh rate",
                    egui::FontId::proportional(14.0),
                    text_main(),
                );
                p.text(
                    egui::pos2(card3.left() + 16.0, card3.top() + 44.0),
                    egui::Align2::LEFT_CENTER,
                    format!("{current} Hz · {mode}"),
                    egui::FontId::proportional(12.5),
                    text_muted(),
                );
                let mut bx = card3.left() + 16.0;
                for rate in rates.iter().take(4) {
                    let label = format!("{rate} Hz");
                    let w = 70.0;
                    let r = egui::Rect::from_min_size(
                        egui::pos2(bx, card3.top() + 62.0),
                        egui::vec2(w, 22.0),
                    );
                    let resp =
                        ui.interact(r, egui::Id::new(("refresh", rate)), egui::Sense::click());
                    let active = current.starts_with(rate.as_str());
                    p.rect_filled(
                        r,
                        4.0,
                        if active {
                            egui::Color32::from_rgba_unmultiplied(
                                accent.r(),
                                accent.g(),
                                accent.b(),
                                40,
                            )
                        } else {
                            card_bg2()
                        },
                    );
                    p.rect_stroke(
                        r,
                        4.0,
                        egui::Stroke::new(
                            1.0,
                            if active {
                                accent
                            } else {
                                egui::Color32::from_rgb(60, 60, 68)
                            },
                        ),
                    );
                    p.text(
                        r.center(),
                        egui::Align2::CENTER_CENTER,
                        &label,
                        egui::FontId::proportional(10.5),
                        if active { accent } else { text_muted() },
                    );
                    if resp.clicked() && !active {
                        match apply_refresh_rate(out, &mode, rate) {
                            Ok(()) => {
                                // mode changes can reset gamma; re-apply the selected profile
                                let _ =
                                    apply_display_gamma(out, DISPLAY_MODES[self.display_mode].2);
                                self.report(format!("refresh rate -> {rate} Hz"), false);
                            }
                            Err(e) => self.report(format!("refresh rate: {e}"), true),
                        }
                    }
                    bx += w + 6.0;
                }
            }
        });
        ui.add_space(10.0);
        ui.colored_label(
            text_muted(),
            "Refresh-rate switching is applied live through XRandR and is reversible; gamma is re-applied after a mode change.",
        );
    }

    fn ui_feature_library(&mut self, ui: &mut egui::Ui) {
        let accent = self.theme.accent();
        ui.colored_label(
            text_muted(),
            "What this build implements on the FX706HF, what is hardware-limited, and what is intentionally absent.",
        );
        ui.add_space(10.0);
        let features: [(&str, &str, bool, u8); 12] = [
            (
                "Performance modes",
                "Silent / Balanced / Turbo — firmware thermal policy",
                true,
                3,
            ),
            (
                "Keyboard RGB",
                "11 firmware effects, speed, 0–255 mixing",
                true,
                2,
            ),
            (
                "Software lighting",
                "Music, Smart (CPU), Adaptive (screen)",
                true,
                2,
            ),
            (
                "Keyboard brightness",
                "0 / 33 / 66 / 100 % via asus::kbd_backlight",
                true,
                2,
            ),
            (
                "Fan telemetry",
                "CPU + GPU RPM from the ASUS EC (read-only)",
                true,
                5,
            ),
            (
                "Battery care",
                "Charge limit + charge mode + health",
                true,
                6,
            ),
            ("Panel overdrive", "144 Hz response optimisation", true, 7),
            (
                "CPU power cap",
                "Sustained PL1 25–65 W (reversible)",
                true,
                3,
            ),
            (
                "Scenario profiles",
                "Per-process mode rules + auto restore",
                true,
                4,
            ),
            (
                "Aura Wallpaper",
                "Local gallery + native XFCE application",
                true,
                9,
            ),
            ("Audio", "PipeWire volume/mute + device inventory", true, 2),
            (
                "Fan curves / PPT",
                "Firmware rejects curve + Intel PPT commands",
                false,
                5,
            ),
        ];
        let avail = ui.available_width();
        let cols = if avail > 1300.0 {
            4
        } else if avail > 900.0 {
            3
        } else {
            2
        };
        let gap = 10.0;
        let card_w = ((avail - gap * (cols as f32 - 1.0)) / cols as f32).max(210.0);
        for row in 0..features.len().div_ceil(cols) {
            ui.horizontal(|ui| {
                for col in 0..cols {
                    let i = row * cols + col;
                    let Some((name, desc, working, icon)) = features.get(i).copied() else {
                        continue;
                    };
                    let (rect, _) =
                        ui.allocate_exact_size(egui::vec2(card_w, 100.0), egui::Sense::hover());
                    let p = ui.painter_at(rect);
                    p.rect_filled(rect, 8.0, card_bg());
                    p.rect_stroke(
                        rect.shrink(0.5),
                        8.0,
                        egui::Stroke::new(
                            1.0,
                            if working {
                                egui::Color32::from_rgb(44, 44, 50)
                            } else {
                                egui::Color32::from_rgb(66, 54, 44)
                            },
                        ),
                    );
                    let c = egui::pos2(rect.left() + 28.0, rect.top() + 28.0);
                    p.circle_filled(
                        c,
                        16.0,
                        egui::Color32::from_rgba_unmultiplied(
                            if working { accent.r() } else { 160 },
                            if working { accent.g() } else { 140 },
                            if working { accent.b() } else { 110 },
                            22,
                        ),
                    );
                    nav_icon(
                        &p,
                        c,
                        icon,
                        if working {
                            accent
                        } else {
                            egui::Color32::from_rgb(170, 150, 120)
                        },
                    );
                    p.text(
                        egui::pos2(rect.left() + 52.0, rect.top() + 22.0),
                        egui::Align2::LEFT_CENTER,
                        name,
                        egui::FontId::proportional(13.5),
                        text_main(),
                    );
                    p.text(
                        egui::pos2(rect.left() + 16.0, rect.top() + 58.0),
                        egui::Align2::LEFT_CENTER,
                        desc,
                        egui::FontId::proportional(11.5),
                        text_muted(),
                    );
                    p.text(
                        egui::pos2(rect.left() + 16.0, rect.bottom() - 16.0),
                        egui::Align2::LEFT_CENTER,
                        if working {
                            "WORKING · this laptop"
                        } else {
                            "LIMITED · firmware"
                        },
                        egui::FontId::proportional(10.5),
                        if working {
                            egui::Color32::from_rgb(120, 200, 140)
                        } else {
                            egui::Color32::from_rgb(190, 160, 120)
                        },
                    );
                    ui.add_space(gap);
                }
            });
            ui.add_space(8.0);
        }
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
        ui.colored_label(
            text_muted(),
            "Startup, appearance and the local control channel.",
        );
        ui.add_space(10.0);
        section_label(ui, "System control");
        ui.add_space(8.0);
        let writable = self.backend.name() == "daemon" || self.writes_enabled;
        let t = self.telemetry.clone();
        egui::Frame::none()
            .fill(card_bg())
            .rounding(8.0)
            .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(44, 44, 50)))
            .inner_margin(16.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let (dot, _) =
                        ui.allocate_exact_size(egui::vec2(9.0, 9.0), egui::Sense::hover());
                    ui.painter_at(dot).circle_filled(
                        dot.center(),
                        4.0,
                        if writable {
                            egui::Color32::from_rgb(120, 200, 140)
                        } else {
                            egui::Color32::from_rgb(200, 160, 80)
                        },
                    );
                    ui.label(
                        egui::RichText::new(if writable {
                            "Hardware writes enabled — tuf-crated (verified ops, audit log)"
                        } else {
                            "Monitoring only — tuf-crated is not reachable"
                        })
                        .size(13.5),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.colored_label(text_muted(), self.backend.name().to_string());
                    });
                });
                ui.add_space(8.0);
                ui.horizontal_wrapped(|ui| {
                    if let Some(t) = &t {
                        ui.colored_label(
                            text_muted(),
                            format!(
                                "Profile: {}   ·   Battery limit: {}   ·   Keyboard: {}",
                                t.profile.clone().unwrap_or_else(|| "--".into()),
                                t.battery
                                    .as_ref()
                                    .and_then(|b| b.charge_threshold)
                                    .map(|v| format!("{v} %"))
                                    .unwrap_or_else(|| "full".into()),
                                t.kbd_brightness
                                    .map(|b| format!("level {b}"))
                                    .unwrap_or_else(|| "--".into()),
                            ),
                        );
                    }
                    ui.colored_label(
                        text_muted(),
                        format!(
                            "Audit: /var/lib/tuf-crate/audit.jsonl · v{}",
                            env!("CARGO_PKG_VERSION")
                        ),
                    );
                });
            });

        ui.add_space(12.0);
        section_label(ui, "Power source profiles");
        ui.add_space(8.0);
        egui::Frame::none()
            .fill(card_bg())
            .rounding(8.0)
            .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(44, 44, 50)))
            .inner_margin(16.0)
            .show(ui, |ui| {
                let before = (
                    self.power_profile_switch,
                    self.ac_profile.clone(),
                    self.battery_profile.clone(),
                );
                ui.checkbox(
                    &mut self.power_profile_switch,
                    "Switch performance mode automatically when AC is connected/disconnected",
                );
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label("On AC");
                    egui::ComboBox::from_id_salt("ac_profile")
                        .selected_text(match self.ac_profile.as_str() {
                            "quiet" => "Silent",
                            "balanced" => "Balanced",
                            _ => "Turbo",
                        })
                        .show_ui(ui, |ui| {
                            for p in games::RULE_PROFILES {
                                ui.selectable_value(
                                    &mut self.ac_profile,
                                    p.to_string(),
                                    match p {
                                        "quiet" => "Silent",
                                        "balanced" => "Balanced",
                                        _ => "Turbo",
                                    },
                                );
                            }
                        });
                    ui.add_space(20.0);
                    ui.label("On battery");
                    egui::ComboBox::from_id_salt("battery_profile")
                        .selected_text(match self.battery_profile.as_str() {
                            "quiet" => "Silent",
                            "balanced" => "Balanced",
                            _ => "Turbo",
                        })
                        .show_ui(ui, |ui| {
                            for p in games::RULE_PROFILES {
                                ui.selectable_value(
                                    &mut self.battery_profile,
                                    p.to_string(),
                                    match p {
                                        "quiet" => "Silent",
                                        "balanced" => "Balanced",
                                        _ => "Turbo",
                                    },
                                );
                            }
                        });
                });
                if before
                    != (
                        self.power_profile_switch,
                        self.ac_profile.clone(),
                        self.battery_profile.clone(),
                    )
                {
                    self.save_ui_settings();
                }
            });

        ui.add_space(12.0);
        section_label(ui, "Startup & device image");
        ui.add_space(8.0);
        egui::Frame::none()
            .fill(card_bg())
            .rounding(8.0)
            .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(44, 44, 50)))
            .inner_margin(16.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
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
                    ui.add_space(24.0);
                    ui.label("Device image");
                    ui.radio_value(&mut self.device_line_art, false, "Photographic");
                    ui.radio_value(&mut self.device_line_art, true, "Line art");
                });
                ui.add_space(4.0);
                ui.colored_label(
                    text_muted(),
                    format!(
                        "Detected model: {} · hero loaded from the local Armoury ProductPNG cache when present.",
                        self.hero_model
                    ),
                );
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.label("Interface scale");
                    for pct in [75u32, 100, 125, 150] {
                        let target = pct as f32 / 100.0;
                        let selected = (self.ui_scale - target).abs() < 0.01;
                        if ui
                            .selectable_label(selected, format!("{pct} %"))
                            .on_hover_text("Armoury matches 100 % on this panel")
                            .clicked()
                            && !selected
                        {
                            self.ui_scale = target;
                            ui.ctx().set_zoom_factor(target / self.native_ppp);
                            self.save_ui_settings();
                        }
                    }
                    ui.colored_label(
                        text_muted(),
                        format!("native {:.2}×", self.native_ppp),
                    );
                });
            });

        ui.add_space(12.0);
        section_label(ui, "Theme");
        ui.add_space(8.0);
        ui.colored_label(
            text_muted(),
            "Theme previews use the original Armoury Crate assets from the local reference library.",
        );
        ui.add_space(6.0);
        let previews = self.theme_previews.clone();
        let avail = ui.available_width();
        let gap = 10.0;
        let cols = if avail > 1300.0 {
            6
        } else if avail > 900.0 {
            4
        } else {
            3
        };
        let card_w = ((avail - gap * (cols as f32 - 1.0)) / cols as f32).clamp(150.0, 220.0);
        let card_h = (card_w * 0.6).clamp(92.0, 132.0);
        for row in 0..previews.len().div_ceil(cols) {
            ui.horizontal(|ui| {
                for col in 0..cols {
                    let i = row * cols + col;
                    let Some((theme, tex)) = previews.get(i) else {
                        continue;
                    };
                    let selected = self.theme == *theme;
                    let (rect, resp) =
                        ui.allocate_exact_size(egui::vec2(card_w, card_h), egui::Sense::click());
                    let p = ui.painter_at(rect);
                    p.rect_filled(rect, 7.0, card_bg());
                    if let Some(tex) = tex {
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
                        7.0,
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
                        self.theme = *theme;
                        apply_theme_visuals(ui.ctx(), *theme);
                        self.save_ui_settings();
                    }
                    ui.add_space(gap);
                }
            });
            ui.add_space(8.0);
        }
        ui.add_space(4.0);
        ui.colored_label(
            text_muted(),
            format!("#{:02X}{:02X}{:02X}", accent.r(), accent.g(), accent.b()),
        );
    }

    fn ui_settings_update(&mut self, ui: &mut egui::Ui) {
        ui.heading("Update Center");
        ui.colored_label(
            text_muted(),
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
                        self.update_text = format!("{count} upgradable package(s)\n\n{text}");
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
            text_muted(),
            "Firmware updates: use fwupd (if installed) and the vendor tooling; they are not automated here.",
        );
    }

    /// Merge/remove the boot-time dGPU flag without touching the live EC.
    fn schedule_boot_gpu(&mut self, disable: Option<bool>) {
        let mut cfg = match self.backend.get_boot_config() {
            Ok(Some(v)) if v.is_object() => v,
            _ => serde_json::json!({}),
        };
        if let Some(obj) = cfg.as_object_mut() {
            match disable {
                Some(d) => {
                    obj.insert("dgpu_disable".into(), serde_json::json!(d));
                }
                None => {
                    obj.remove("dgpu_disable");
                }
            }
        }
        match self.backend.save_boot_config(cfg) {
            Ok(()) => self.report(
                match disable {
                    Some(true) => {
                        "Eco Mode scheduled for next boot (safe, pre-display)".to_string()
                    }
                    Some(false) => "Standard scheduled for next boot".to_string(),
                    None => "Boot-time GPU schedule cleared".to_string(),
                },
                false,
            ),
            Err(e) => self.report(
                format!(
                    "GPU schedule failed: {} — reinstall tuf-crated for boot-time support",
                    friendly_error(&e)
                ),
                true,
            ),
        }
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
            "ui_scale": self.ui_scale,
            "power_profile_switch": self.power_profile_switch,
            "ac_profile": self.ac_profile,
            "battery_profile": self.battery_profile,
        });
        let _ = std::fs::write(path, serde_json::to_string_pretty(&doc).unwrap_or_default());
    }

    fn ui_home(&mut self, ui: &mut egui::Ui) {
        let Some(t) = self.telemetry.clone() else {
            return;
        };
        let orange = accent();
        let white = text_main();
        let gray = text_muted();
        let now = ui.ctx().input(|i| i.time) as f32;
        let rate = match self.rgb_speed {
            0xe1 => 0.55,
            0xf5 => 1.7,
            _ => 1.0,
        };
        let avail = ui.available_width();
        // Armoury Home: hero column ~38 % of the content at reference width,
        // with a narrower variant when the window is small.
        let wide = avail > 1200.0;
        let left_w = if wide {
            (avail * 0.38).clamp(430.0, 620.0)
        } else {
            (avail * 0.34).clamp(300.0, 520.0)
        };
        let gutter = if wide { 61.0 } else { 24.0 };
        let right_w = (avail - left_w - gutter).max(360.0);
        // Extra vertical room (e.g. a maximised 1080p window) is distributed
        // instead of leaving a dead band below the cards.
        let extra = (ui.available_height() - 880.0).clamp(0.0, 200.0);

        ui.horizontal_top(|ui| {
            // ---------------- left column: product + devices ----------------
            ui.vertical(|ui| {
                ui.set_width(left_w);
                ui.add_space(143.0);
                let (slot, _) =
                    ui.allocate_exact_size(egui::vec2(left_w, 204.0), egui::Sense::hover());
                let art = egui::Rect::from_center_size(slot.center(), egui::vec2(295.0, 204.0));
                let p = ui.painter_at(slot);
                let hero = if self.device_line_art {
                    self.hero_line.as_ref()
                } else {
                    self.hero_image.as_ref()
                };
                if let Some(tex) = hero {
                    draw_hero_texture(&p, art, tex);
                } else {
                    draw_product(
                        &p,
                        art,
                        now,
                        rate,
                        self.rgb_color,
                        self.rgb_mode,
                        self.device_line_art,
                    );
                }
                ui.add_space(151.0);
                let (txt, txtresp) =
                    ui.allocate_exact_size(egui::vec2(left_w, 74.0), egui::Sense::hover());
                txtresp.on_hover_text(format!("Detected model: {}", self.hero_model));
                let tp = ui.painter_at(txt);
                tp.text(
                    egui::pos2(txt.center().x, txt.top() + 21.0),
                    egui::Align2::CENTER_CENTER,
                    "ASUS TUF GAMING F17",
                    rog_font(24.0),
                    white,
                );
                tp.text(
                    egui::pos2(txt.center().x, txt.top() + 50.0),
                    egui::Align2::CENTER_CENTER,
                    "11th Gen Intel(R) Core(TM) i5-11260H @ 2.60GHz",
                    egui::FontId::proportional(11.5),
                    gray,
                );
                tp.text(
                    egui::pos2(txt.center().x, txt.top() + 68.0),
                    egui::Align2::CENTER_CENTER,
                    "NVIDIA GeForce RTX 2050 GDDR6 @ 4GB 64 bits",
                    egui::FontId::proportional(11.5),
                    gray,
                );
                ui.add_space(46.0);

                // ---- Devices (0) header + tall placeholder card ----
                let (hdr, hresp) =
                    ui.allocate_exact_size(egui::vec2(left_w, 22.0), egui::Sense::click());
                let hp = ui.painter_at(hdr);
                section_label_p(&hp, hdr.left_top() + egui::vec2(0.0, 4.0), "Devices (0)");
                hp.text(
                    egui::pos2(hdr.right() - 42.0, hdr.top() + 8.0),
                    egui::Align2::RIGHT_CENTER,
                    "View All",
                    egui::FontId::proportional(12.5),
                    gray,
                );
                // up-chevron
                let cx = hdr.right() - 10.0;
                hp.add(egui::Shape::line(
                    vec![
                        egui::pos2(cx - 6.0, hdr.top() + 13.0),
                        egui::pos2(cx, hdr.top() + 7.0),
                        egui::pos2(cx + 6.0, hdr.top() + 13.0),
                    ],
                    egui::Stroke::new(1.4, gray),
                ));
                ui.add_space(4.0);
                let (dev, devresp) = ui.allocate_exact_size(
                    egui::vec2(left_w, 224.0 + extra * 0.75),
                    egui::Sense::click(),
                );
                let dp = ui.painter_at(dev);
                let card_edge = dev.shrink(0.5);
                dp.rect_filled(card_edge, 10.0, page_bg());
                dp.rect_stroke(
                    card_edge,
                    10.0,
                    egui::Stroke::new(1.0, egui::Color32::from_rgb(54, 54, 60)),
                );
                let dashed = egui::Rect::from_min_size(
                    egui::pos2(dev.left() + 97.0, dev.top() + 15.0),
                    egui::vec2(81.0, 81.0),
                );
                dashed_rounded_rect(
                    &dp,
                    dashed,
                    4.0,
                    egui::Stroke::new(1.0, egui::Color32::from_rgb(100, 100, 110)),
                );
                let plus_c = dashed.center();
                dp.add(egui::Shape::line(
                    vec![
                        egui::pos2(plus_c.x - 16.0, plus_c.y),
                        egui::pos2(plus_c.x + 16.0, plus_c.y),
                    ],
                    egui::Stroke::new(1.3, egui::Color32::from_rgb(170, 170, 180)),
                ));
                dp.add(egui::Shape::line(
                    vec![
                        egui::pos2(plus_c.x, plus_c.y - 16.0),
                        egui::pos2(plus_c.x, plus_c.y + 16.0),
                    ],
                    egui::Stroke::new(1.3, egui::Color32::from_rgb(170, 170, 180)),
                ));
                let tcx = (dashed.right() + dev.right()) / 2.0;
                dp.text(
                    egui::pos2(tcx, dev.top() + 33.0),
                    egui::Align2::CENTER_CENTER,
                    "No Devices Connected",
                    egui::FontId::proportional(13.5),
                    white,
                );
                dp.text(
                    egui::pos2(tcx, dev.top() + 53.0),
                    egui::Align2::CENTER_CENTER,
                    "Ensure that the devices are connected properly.",
                    egui::FontId::proportional(11.8),
                    gray,
                );
                dp.text(
                    egui::pos2(tcx, dev.top() + 73.0),
                    egui::Align2::CENTER_CENTER,
                    "For the latest ASUS products, visit",
                    egui::FontId::proportional(11.8),
                    gray,
                );
                dp.text(
                    egui::pos2(tcx, dev.top() + 96.0),
                    egui::Align2::CENTER_CENTER,
                    "ASUS official.",
                    egui::FontId::proportional(11.8),
                    orange,
                );
                if hresp.clicked() || devresp.clicked() {
                    self.tab = Tab::Devices;
                }
            });

            ui.add_space(gutter);

            // ---------------- right column: system usage + mode ----------------
            ui.vertical(|ui| {
                ui.set_max_width(right_w);
                ui.add_space(3.0);
                let (lab, _) =
                    ui.allocate_exact_size(egui::vec2(right_w, 16.0), egui::Sense::hover());
                section_label_p(&ui.painter_at(lab), lab.left_top(), "System Usage");
                ui.add_space(16.0);

                let cpu_mhz = self.cpu_mhz;
                let cpu_util = self.cpu_util.unwrap_or(0);
                let cpu_temp = t.cpu_package_temp_mC;
                let cpu_rows = vec![
                    UsageRow::Meter(
                        "Frequency",
                        cpu_mhz
                            .map(|m| format!("{m} MHz"))
                            .unwrap_or_else(|| "--".into()),
                        cpu_mhz.map(|m| m as f32 / 4500.0).unwrap_or(0.0),
                    ),
                    UsageRow::Meter("Usage", format!("{cpu_util} %"), cpu_util as f32 / 100.0),
                    UsageRow::Meta("Memory Frequency", "--".into()),
                    UsageRow::Meta(
                        "Temperature",
                        cpu_temp
                            .map(|m| format!("{:.0} °C", m as f32 / 1000.0))
                            .unwrap_or_else(|| "--".into()),
                    ),
                    UsageRow::Meta("Voltage", "--".into()),
                ];
                let (util, temp, tgp) = t
                    .nvidia
                    .as_ref()
                    .map(|g| (g.utilization_percent, g.temperature_c, g.power_watts))
                    .unwrap_or((0, 0, 0.0));
                let gpu_rows = vec![
                    UsageRow::Meta(
                        "Frequency",
                        if util == 0 {
                            "Power Saving".into()
                        } else {
                            "Active".into()
                        },
                    ),
                    UsageRow::Meter("Usage", format!("{util} %"), util as f32 / 100.0),
                    UsageRow::Meta("Memory Frequency", "--".into()),
                    UsageRow::Meta("Temperature", format!("{temp} °C")),
                    UsageRow::Meta("Voltage", "--".into()),
                    UsageRow::Meta("TGP", format!("{tgp:.0} W")),
                ];
                let col_gap = 32.0;
                let col_w = (right_w - col_gap) / 2.0;
                let top_h = 273.0;
                ui.horizontal_top(|ui| {
                    let (r1, _) =
                        ui.allocate_exact_size(egui::vec2(col_w, top_h), egui::Sense::hover());
                    draw_usage_block(&ui.painter_at(r1), r1, "CPU", &cpu_rows);
                    ui.add_space(col_gap);
                    let (r2, _) =
                        ui.allocate_exact_size(egui::vec2(col_w, top_h), egui::Sense::hover());
                    draw_usage_block(&ui.painter_at(r2), r2, "GPU", &gpu_rows);
                });
                ui.add_space(104.0);

                let fan_rows = {
                    let mut v = Vec::new();
                    if let Some(r) = t.cpu_fan_rpm {
                        v.push(UsageRow::Meter(
                            "CPU Fan",
                            format!("{r} RPM"),
                            r as f32 / 6000.0,
                        ));
                    } else {
                        v.push(UsageRow::Meta("CPU Fan", "--".into()));
                    }
                    if let Some(r) = t.gpu_fan_rpm {
                        v.push(UsageRow::Meter(
                            "GPU Fan",
                            format!("{r} RPM"),
                            r as f32 / 6000.0,
                        ));
                    } else {
                        v.push(UsageRow::Meta("GPU Fan", "--".into()));
                    }
                    v.push(UsageRow::Meta("Fan Acoustics", "--".into()));
                    v
                };
                let mem_rows = {
                    let mut v = Vec::new();
                    if let (Some(u), Some(tot)) = (t.storage_used_gb, t.storage_total_gb) {
                        v.push(UsageRow::Meter(
                            "Storage",
                            format!("{u:.1} / {tot:.1} GB"),
                            u / tot.max(0.1),
                        ));
                    } else {
                        v.push(UsageRow::Meta("Storage", "--".into()));
                    }
                    if let (Some(u), Some(tot)) = (t.ram_used_gb, t.ram_total_gb) {
                        v.push(UsageRow::Meter(
                            "RAM",
                            format!("{u:.1} / {tot:.1} GB"),
                            u / tot.max(0.1),
                        ));
                    } else {
                        v.push(UsageRow::Meta("RAM", "--".into()));
                    }
                    v
                };
                let bot_h = 240.0;
                ui.horizontal_top(|ui| {
                    let (r1, _) =
                        ui.allocate_exact_size(egui::vec2(col_w, bot_h), egui::Sense::hover());
                    draw_usage_block(&ui.painter_at(r1), r1, "Fan", &fan_rows);
                    ui.add_space(col_gap);
                    let (r2, _) =
                        ui.allocate_exact_size(egui::vec2(col_w, bot_h), egui::Sense::hover());
                    draw_usage_block(&ui.painter_at(r2), r2, "Memory", &mem_rows);
                });
                ui.add_space(33.0 + extra);
                let (lab2, _) =
                    ui.allocate_exact_size(egui::vec2(right_w, 16.0), egui::Sense::hover());
                section_label_p(&ui.painter_at(lab2), lab2.left_top(), "Operating Mode");
                ui.add_space(19.0);

                let current = t.profile.clone().unwrap_or_default();
                let modes: [(&str, u8, Option<Profile>, &str); 3] = [
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
                let card_w = (right_w - 2.0 * 10.0) / 3.0;
                ui.horizontal(|ui| {
                    for (name, icon, profile, hint) in modes {
                        let selected = profile.map(|p| current == p.as_str()).unwrap_or(false);
                        let (rect, resp) =
                            ui.allocate_exact_size(egui::vec2(card_w, 122.0), egui::Sense::click());
                        let p = ui.painter_at(rect);
                        p.rect_filled(
                            rect,
                            7.0,
                            if selected {
                                egui::Color32::from_rgba_unmultiplied(
                                    accent().r(),
                                    accent().g(),
                                    accent().b(),
                                    52,
                                )
                            } else {
                                card_bg2()
                            },
                        );
                        p.rect_stroke(
                            rect,
                            7.0,
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
                            egui::pos2(rect.center().x, rect.top() + 20.0),
                            icon,
                            if selected {
                                orange
                            } else {
                                egui::Color32::from_rgb(200, 200, 210)
                            },
                        );
                        p.text(
                            egui::pos2(rect.center().x, rect.bottom() - 12.0),
                            egui::Align2::CENTER_CENTER,
                            name,
                            egui::FontId::proportional(13.0),
                            if selected { orange } else { gray },
                        );
                        let resp = resp.on_hover_text(hint);
                        if profile.is_some() && resp.clicked() {
                            if let Some(pr) = profile {
                                match self.backend.set_profile(pr, true) {
                                    Ok(()) => {
                                        self.report(format!("profile -> {}", pr.label()), false)
                                    }
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
            });
        });
    }

    fn ui_dashboard(&mut self, ui: &mut egui::Ui) {
        let Some(t) = self.telemetry.clone() else {
            return;
        };
        let total_w = ui.available_width();
        let right_w = (total_w - 440.0).max(300.0);
        ui.horizontal_top(|ui| {
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
            });
            ui.add_space(12.0);
            ui.allocate_ui_with_layout(
                egui::vec2(right_w, 0.0),
                egui::Layout::top_down(egui::Align::LEFT),
                |ui| {
                egui::Frame::none()
                    .fill(card_surface())
                    .rounding(10.0)
                    .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(52, 52, 58)))
                    .shadow(egui::epaint::Shadow {
                        offset: egui::vec2(0.0, 3.0),
                        blur: 10.0,
                        spread: 0.0,
                        color: egui::Color32::from_black_alpha(55),
                    })
                    .inner_margin(16.0)
                    .show(ui, |ui| {
                        ui.set_width((right_w - 32.0).max(240.0));
                        egui::Grid::new("dash_secondary")
                            .num_columns(2)
                            .spacing([28.0, 8.0])
                            .striped(true)
                            .show(ui, |ui| {
                                ui.colored_label(text_muted(), "Sensors");
                                ui.colored_label(text_muted(), "Value");
                                ui.end_row();
                                ui.label("NVMe SSD");
                                ui.label(Self::temp_c(t.nvme_temp_mC));
                                ui.end_row();
                                for (label, rpm) in &t.fan_labels {
                                    ui.label(format!("Fan · {label}"));
                                    ui.label(
                                        rpm.map(|r| format!("{r} RPM"))
                                            .unwrap_or_else(|| "--".into()),
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
                                    ui.label("Charge limit");
                                    ui.label(
                                        b.charge_threshold
                                            .map(|v| format!("{v} %"))
                                            .unwrap_or_else(|| "full".into()),
                                    );
                                    ui.end_row();
                                }
                                ui.label("Current profile");
                                ui.label(t.profile.clone().unwrap_or_else(|| "--".into()));
                                ui.end_row();
                                if let Some(g) = &t.nvidia {
                                    ui.label("NVIDIA RTX 2050");
                                    ui.label(format!(
                                        "{} °C · {} % · {:.1} W · {}",
                                        g.temperature_c,
                                        g.utilization_percent,
                                        g.power_watts,
                                        g.perf_state
                                    ));
                                    ui.end_row();
                                }
                                ui.label("Kernel");
                                ui.label(
                                    std::fs::read_to_string("/proc/sys/kernel/osrelease")
                                        .unwrap_or_else(|_| "--".into())
                                        .trim(),
                                );
                                ui.end_row();
                            });
                    });
                },
            );
        });
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
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Operating mode").size(15.0).strong());
            info_icon(
                ui,
                "Armoury Crate's Silent/Balanced/Turbo map to the kernel platform_profile.                  TLP owns the CPU governor/EPP; WMI PPT limits (SPL/SPPT/FPPT) are not                  implemented by this model's firmware.",
            );
        });
        ui.add_space(8.0);
        let gap = 10.0;
        let seg_w = ((ui.available_width() - 2.0 * gap) / 3.0).clamp(240.0, 560.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            for (i, p) in Profile::all().iter().enumerate() {
                let selected = t.profile.as_deref() == Some(p.as_str());
                let (name, desc, icon) = match p {
                    Profile::Quiet => ("Silent", "Quiet fan curve", 1u8),
                    Profile::Balanced => ("Balanced", "Default thermal policy", 2),
                    Profile::Performance => ("Turbo", "Max fan + power", 3),
                };
                let (rect, resp) =
                    ui.allocate_exact_size(egui::vec2(seg_w, 108.0), egui::Sense::click());
                let pnt = ui.painter_at(rect);
                pnt.rect_filled(
                    rect,
                    10.0,
                    if selected {
                        egui::Color32::from_rgba_unmultiplied(accent().r(), accent().g(), accent().b(), 38)
                    } else {
                        card_surface()
                    },
                );
                pnt.rect_stroke(
                    rect.shrink(0.5),
                    10.0,
                    egui::Stroke::new(
                        if selected { 1.8 } else { 1.0 },
                        if selected {
                            accent()
                        } else {
                            egui::Color32::from_rgb(58, 58, 64)
                        },
                    ),
                );
                mode_icon(
                    &pnt,
                    egui::pos2(rect.center().x, rect.top() + 30.0),
                    icon,
                    if selected {
                        accent()
                    } else {
                        egui::Color32::from_rgb(200, 200, 210)
                    },
                );
                pnt.text(
                    egui::pos2(rect.center().x, rect.top() + 62.0),
                    egui::Align2::CENTER_CENTER,
                    name,
                    egui::FontId::proportional(15.0),
                    text_main(),
                );
                pnt.text(
                    egui::pos2(rect.center().x, rect.top() + 84.0),
                    egui::Align2::CENTER_CENTER,
                    desc,
                    egui::FontId::proportional(11.0),
                    text_muted(),
                );
                if resp.clicked() {
                    match self.backend.set_profile(*p, true) {
                        Ok(()) => self.report(format!("profile -> {}", p.label()), false),
                        Err(e) => {
                            self.report(format!("profile change failed: {}", friendly_error(&e)), true)
                        }
                    }
                    self.poll();
                }
                if i < 2 {
                    ui.add_space(gap);
                }
            }
        });
        ui.add_space(14.0);
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("CPU power limit").size(15.0).strong());
            info_icon(
                ui,
                "Sustained PL1 via intel-rapl. Lowering it reduces heat but slows sustained                  games/compiles; short bursts (PL2) are untouched. Range 25–65 W, 65 W is the                  firmware default, every write is audited and resets on reboot.",
            );
        });
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
                egui::Color32::from_rgb(255, 68, 68),
                "Lowering caps sustained performance · PL2 untouched",
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
                text_muted(),
                "intel-rapl interface not exposed — unavailable.",
            );
        }
        ui.add_space(14.0);
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("24/7 persistence").size(15.0).strong());
            info_icon(
                ui,
                "Stores the current profile, CPU power cap and battery limit in the daemon boot                  config. The daemon applies them before the display manager at every boot.",
            );
        });
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
                if let Some(th) = t.battery.as_ref().and_then(|b| b.charge_threshold) {
                    cfg.insert("charge_threshold".into(), serde_json::json!(th));
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
                        egui::Color32::from_rgb(255, 68, 68),
                        format!("GPU throttle: {r}"),
                    );
                }
            }
        }
        if let Some(cpu) = t.cpu_package_temp_mC {
            if cpu >= 94_000 {
                ui.colored_label(egui::Color32::from_rgb(255, 68, 68),
                    format!("CPU at {:.0} °C — sustained high temperature. Hardware cooling service recommended (see audit report).", cpu as f32 / 1000.0));
            }
        }
        ui.add_space(10.0);
        ui.heading("Live fan history (60 s)");
        let history: Vec<(u32, u32)> = self.fan_hist.iter().copied().collect();
        if history.len() >= 2 {
            let (r, _) = ui.allocate_exact_size(
                egui::vec2(ui.available_width().min(900.0), 110.0),
                egui::Sense::hover(),
            );
            let p = ui.painter_at(r);
            p.rect_filled(r, 6.0, card_bg2());
            for i in 1..4 {
                let y = r.top() + r.height() * i as f32 / 4.0;
                p.add(egui::Shape::line(
                    vec![egui::pos2(r.left(), y), egui::pos2(r.right(), y)],
                    egui::Stroke::new(
                        1.0,
                        egui::Color32::from_rgba_unmultiplied(255, 255, 255, 10),
                    ),
                ));
            }
            let max = history
                .iter()
                .flat_map(|(a, b)| [*a, *b])
                .max()
                .unwrap_or(6000)
                .max(3000) as f32;
            for (series, color) in [
                (0usize, accent()),
                (1usize, egui::Color32::from_rgb(90, 160, 220)),
            ] {
                let pts: Vec<egui::Pos2> = history
                    .iter()
                    .enumerate()
                    .map(|(i, pair)| {
                        let v = if series == 0 { pair.0 } else { pair.1 } as f32;
                        egui::pos2(
                            r.left() + r.width() * i as f32 / (history.len() - 1) as f32,
                            r.bottom() - (v / max) * (r.height() - 12.0) - 6.0,
                        )
                    })
                    .collect();
                p.add(egui::Shape::line(pts, egui::Stroke::new(1.8, color)));
            }
            p.text(
                egui::pos2(r.left() + 8.0, r.top() + 8.0),
                egui::Align2::LEFT_TOP,
                format!(
                    "CPU fan {} RPM  ·  GPU fan {} RPM  ·  scale 0–{:.0}",
                    t.cpu_fan_rpm.unwrap_or(0),
                    t.gpu_fan_rpm.unwrap_or(0),
                    max
                ),
                egui::FontId::proportional(11.0),
                text_muted(),
            );
        } else {
            ui.colored_label(text_muted(), "Collecting samples…");
        }
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Thermal profile vs fans").size(14.5));
            info_icon(
                ui,
                "The ASUS EC owns the fan curve. Turbo (performance) ramps earlier and higher                  under load; RPM changes are shown in the graph above.",
            );
        });
        ui.horizontal(|ui| {
            for (label, profile) in [
                ("Silent", Profile::Quiet),
                ("Balanced", Profile::Balanced),
                ("Turbo", Profile::Performance),
            ] {
                if ui.button(label).clicked() {
                    match self.backend.set_profile(profile, true) {
                        Ok(()) => self.report(format!("profile -> {}", profile.label()), false),
                        Err(e) => {
                            self.report(format!("profile failed: {}", friendly_error(&e)), true)
                        }
                    }
                    self.poll();
                }
            }
        });
        ui.add_space(6.0);
        if let Some(on) = t.fan_full_speed {
            ui.horizontal(|ui| {
                ui.colored_label(
                    egui::Color32::from_rgb(190, 160, 120),
                    format!("FULLSPEED/AUTO: {}", if on { "FULLSPEED" } else { "AUTO" }),
                );
                info_icon(
                    ui,
                    "This EC acknowledges the kernel FULLSPEED/AUTO write but does not change                      RPM on this model, so it is not presented as a speed control.",
                );
            });
        }
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Fan curves").size(12.0).color(text_muted()));
            info_icon(
                ui,
                "Custom fan curves are not available: the firmware acknowledges the WMI curve                  IDs but performs no curve I/O. Arbitrary PWM writes are outside the safety policy.",
            );
        });
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
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Charge limit").size(14.5));
            info_icon(
                ui,
                "Battery longevity limit via charge_control_end_threshold. Values are clamped to                  the kernel-verified range and every change is audited.",
            );
        });
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
                    self.restore_profile = self.telemetry.as_ref().and_then(|t| t.profile.clone());
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
        let orange = accent();
        let white = text_main();
        let gray = text_muted();
        let brightness = t.kbd_brightness.unwrap_or(0);
        let max_b = t.kbd_max_brightness.unwrap_or(3).max(1);
        let bright_pct = (brightness * 100 / max_b).min(100);
        egui::Frame::none()
            .fill(card_bg2())
            .rounding(12.0)
            .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(50, 50, 56)))
            .inner_margin(18.0)
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.horizontal_top(|ui| {
                    let left_w = (ui.available_width() * 0.57).clamp(500.0, 900.0);
                    ui.vertical(|ui| {
                        ui.set_width(left_w);
                        self.lighting_preview(ui, now, rate);
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            ui.colored_label(egui::Color32::from_rgb(120, 200, 140), "LIVE");
                            ui.colored_label(
                                gray,
                                "WMNB DEVS 0x00100056 · single-zone keyboard",
                            );
                        });
                    });
                    ui.add_space(18.0);
                    ui.vertical(|ui| {
                        ui.set_width((ui.available_width() - 4.0).max(280.0));
                        ui.colored_label(
                            white,
                            egui::RichText::new(Self::aura_mode_name(self.rgb_mode))
                                .size(28.0)
                                .strong(),
                        );
                        ui.add_space(2.0);
                        ui.horizontal(|ui| {
                            let (sw, _) = ui.allocate_exact_size(egui::vec2(22.0, 22.0), egui::Sense::hover());
                            ui.painter_at(sw).rect_filled(
                                sw,
                                4.0,
                                egui::Color32::from_rgb(self.rgb_color[0], self.rgb_color[1], self.rgb_color[2]),
                            );
                            ui.label(
                                egui::RichText::new(format!(
                                    "#{:02X}{:02X}{:02X} · {} % brightness",
                                    self.rgb_color[0], self.rgb_color[1], self.rgb_color[2], bright_pct
                                ))
                                .size(12.5)
                                .color(gray),
                            );
                        });
                        ui.add_space(12.0);
                        ui.colored_label(gray, "Speed");
                        ui.horizontal(|ui| {
                            for (val, label) in [(0xe1u32, "33 %"), (0xeb, "67 %"), (0xf5, "100 %")] {
                                if ui.selectable_label(self.rgb_speed == val, label).clicked() {
                                    self.rgb_speed = val;
                                    self.apply_rgb(self.rgb_mode);
                                }
                            }
                        });
                        ui.add_space(10.0);
                        ui.colored_label(gray, "Brightness");
                        ui.horizontal(|ui| {
                            for lvl in 0..=max_b {
                                let label = format!("{} %", lvl * 100 / max_b);
                                if ui.selectable_label(brightness == lvl, label).on_hover_text(format!("kernel kbd level {lvl}")).clicked() {
                                    if let Err(e) = self.backend.set_kbd_brightness(lvl, true) {
                                        self.report(format!("brightness failed: {}", friendly_error(&e)), true);
                                    }
                                    self.poll();
                                }
                            }
                        });
                        ui.add_space(14.0);
                        ui.horizontal(|ui| {
                            if ui.add_sized([180.0, 34.0], egui::Button::new("Apply to keyboard").fill(egui::Color32::from_rgb(45, 60, 45))).clicked() {
                                self.apply_rgb(self.rgb_mode);
                            }
                            if ui.add_sized([130.0, 34.0], egui::Button::new("Reset")).clicked() {
                                self.rgb_mode = 0;
                                self.rgb_color = [255, 255, 255];
                                self.rgb_hue = 0.0;
                                self.rgb_sat = 0.0;
                                self.rgb_val = 1.0;
                                self.apply_rgb(0);
                            }
                        });

                    });
                });
            });
        ui.add_space(12.0);
        ui.horizontal(|ui| {
            section_label(ui, "Aura effects");
            info_icon(
                ui,
                "11 firmware effects: Rain, Highlight, Laser, Ripple, Star and Comet render                  multicolour regardless of the picker; Static, Breathing and Strobe follow the                  chosen colour. Music/Smart/Adaptive are software effects running over the same                  verified RGB path.",
            );
        });
        ui.add_space(8.0);
        let modes: [(&str, u32, &str); 12] = [
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
            ("Off", 999, "Keyboard backlight off"),
        ];
        let grid_avail = ui.available_width();
        let cols = if grid_avail > 1400.0 {
            6
        } else if grid_avail > 900.0 {
            4
        } else {
            3
        };
        let gap = 10.0;
        let tile_w =
            (((grid_avail - gap * (cols as f32 - 1.0)) / cols as f32) - 2.0).clamp(110.0, 250.0);
        for row in modes.chunks(cols) {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                for (i, (name, mode, desc)) in row.iter().enumerate() {
                    let is_off = *mode == 999;
                    let selected = if is_off {
                        brightness == 0
                    } else {
                        self.rgb_mode == *mode && brightness > 0
                    };
                    let (rect, resp) =
                        ui.allocate_exact_size(egui::vec2(tile_w, 102.0), egui::Sense::click());
                    let p = ui.painter_at(rect);
                    p.rect_filled(
                        rect,
                        8.0,
                        if selected {
                            egui::Color32::from_rgba_unmultiplied(
                                orange.r(),
                                orange.g(),
                                orange.b(),
                                28,
                            )
                        } else {
                            card_bg()
                        },
                    );
                    p.rect_stroke(
                        rect.shrink(0.5),
                        8.0,
                        egui::Stroke::new(
                            if selected { 1.5 } else { 1.0 },
                            if selected {
                                orange
                            } else if resp.hovered() {
                                egui::Color32::from_rgb(72, 72, 80)
                            } else {
                                egui::Color32::from_rgb(44, 44, 50)
                            },
                        ),
                    );
                    if is_off {
                        let col = if selected { orange } else { egui::Color32::from_rgb(205, 205, 215) };
                        p.circle_stroke(egui::pos2(rect.center().x, rect.top() + 38.0), 13.0, egui::Stroke::new(1.8, col));
                        p.add(egui::Shape::line(
                            vec![
                                egui::pos2(rect.center().x - 7.0, rect.top() + 45.0),
                                egui::pos2(rect.center().x + 7.0, rect.top() + 31.0),
                            ],
                            egui::Stroke::new(1.8, col),
                        ));
                    } else {
                        effect_icon(
                            &p,
                            egui::pos2(rect.center().x, rect.top() + 38.0),
                            *mode,
                            if selected {
                                orange
                            } else {
                                egui::Color32::from_rgb(205, 205, 215)
                            },
                        );
                    }
                    p.text(
                        egui::pos2(rect.center().x, rect.bottom() - 16.0),
                        egui::Align2::CENTER_CENTER,
                        *name,
                        egui::FontId::proportional(11.5),
                        if selected { orange } else { gray },
                    );
                    let resp = resp.on_hover_text(*desc);
                    if resp.clicked() {
                        if is_off {
                            if let Err(e) = self.backend.set_kbd_brightness(0, true) {
                                self.report(format!("keyboard off failed: {}", friendly_error(&e)), true);
                            } else {
                                self.report("keyboard backlight off".into(), false);
                            }
                            self.poll();
                        } else {
                            self.rgb_mode = *mode;
                            self.apply_rgb(*mode);
                        }
                    }
                    if i + 1 < row.len() {
                        ui.add_space(gap);
                    }
                }
            });
            ui.add_space(8.0);
        }
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
                (
                    music::ADAPTIVE_EFFECT_ID,
                    adaptive_active,
                    "Adaptive (screen)",
                ),
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
            text_muted(),
            if self.soft_effect.is_some() {
                "running — frames stream through the verified RGB path; click the active effect to stop"
            } else {
                "Music = desktop audio · Smart = CPU temperature · Adaptive = screen average"
            },
        );
        ui.add_space(8.0);

        // ---------- colour mixing ----------
        section_label(ui, "Colour mixing");
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            // saturation/value square
            let sv_w = 280.0;
            let sv_h = 158.0;
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
                    self.rgb_sat = ((pos.x - sv_rect.left()) / sv_rect.width()).clamp(0.0, 1.0);
                    self.rgb_val =
                        1.0 - ((pos.y - sv_rect.top()) / sv_rect.height()).clamp(0.0, 1.0);
                    self.rgb_color = Self::hsv_to_rgb(self.rgb_hue, self.rgb_sat, self.rgb_val);
                }
            }
            ui.vertical(|ui| {
                // hue strip
                let (strip_rect, strip_resp) =
                    ui.allocate_exact_size(egui::vec2(300.0, 22.0), egui::Sense::click());
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
                            egui::vec2(strip_rect.width() / 72.0 + 0.5, 22.0),
                        ),
                        0.0,
                        egui::Color32::from_rgb(c[0], c[1], c[2]),
                    );
                }
                let marker_x = strip_rect.left() + (self.rgb_hue / 360.0) * strip_rect.width();
                p.line_segment(
                    [
                        egui::pos2(marker_x, strip_rect.top() - 2.0),
                        egui::pos2(marker_x, strip_rect.bottom() + 2.0),
                    ],
                    egui::Stroke::new(2.0, egui::Color32::WHITE),
                );
                if strip_resp.clicked() {
                    if let Some(pos) = strip_resp.interact_pointer_pos() {
                        self.rgb_hue = ((pos.x - strip_rect.left()) / strip_rect.width())
                            .clamp(0.0, 0.9999)
                            * 360.0;
                        self.rgb_color = Self::hsv_to_rgb(self.rgb_hue, self.rgb_sat, self.rgb_val);
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
            ui.add_space(16.0);
            ui.vertical(|ui| {
                egui::Frame::none()
                    .fill(card_bg2())
                    .rounding(8.0)
                    .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(48, 48, 54)))
                    .inner_margin(14.0)
                    .show(ui, |ui| {
                        ui.set_width(250.0);
                        ui.colored_label(text_muted(), "Current colour");
                        let (sw, _) =
                            ui.allocate_exact_size(egui::vec2(222.0, 86.0), egui::Sense::hover());
                        ui.painter_at(sw).rect_filled(
                            sw,
                            7.0,
                            egui::Color32::from_rgb(
                                self.rgb_color[0],
                                self.rgb_color[1],
                                self.rgb_color[2],
                            ),
                        );
                        ui.add_space(6.0);
                        ui.label(
                            egui::RichText::new(format!(
                                "#{:02X}{:02X}{:02X}",
                                self.rgb_color[0], self.rgb_color[1], self.rgb_color[2]
                            ))
                            .size(18.0)
                            .strong(),
                        );
                        ui.label(format!(
                            "{} · speed {} %",
                            Self::aura_mode_name(self.rgb_mode),
                            Self::speed_percent(self.rgb_speed)
                        ));
                        ui.add_space(6.0);
                        if ui
                            .add_sized(
                                [ui.available_width(), 30.0],
                                egui::Button::new("Apply colour to keyboard")
                                    .fill(egui::Color32::from_rgb(45, 60, 45)),
                            )
                            .clicked()
                        {
                            self.apply_rgb(self.rgb_mode);
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
            ui.painter_at(pr)
                .rect_filled(pr, 4.0, egui::Color32::from_rgb(col[0], col[1], col[2]));
            if ui
                .add_sized(
                    [170.0, 34.0],
                    egui::Button::new("🎨 Apply to keyboard")
                        .fill(egui::Color32::from_rgb(45, 90, 60)),
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
                text_muted(),
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
                let tw =
                    (0.5 + 0.5 * (t * 2.5 * rate + seed * std::f32::consts::TAU).sin()).powi(5);
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
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Display").size(14.5));
            info_icon(
                ui,
                "Panel overdrive is a verified kernel attribute. Refresh-rate and gamma controls                  live in GameVisual → Refresh rate.",
            );
        });
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
            text_muted(),
            "prime-run <application>   (wrapper installed during the system audit)",
        );
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("dGPU mode — reboot-safe").size(14.5));
            info_icon(
                ui,
                "Live dGPU power-off froze the compositor on this EC. Only the safe direction                  (Standard restore) is exposed; Eco is scheduled for the next boot.",
            );
        });
        let caps = self.backend.capabilities();
        if !caps.dgpu_disable {
            ui.label("dgpu_disable not exposed — unavailable.");
            return;
        }
        let disabled = t.dgpu_disable.unwrap_or(false);
        ui.label(format!(
            "Current firmware state: {}",
            if disabled {
                "Eco Mode (dGPU powered off)"
            } else {
                "Standard (hybrid / dGPU enabled)"
            }
        ));
        if disabled {
            ui.colored_label(
                egui::Color32::from_rgb(255, 68, 68),
                "Restoring Standard is safe. Reboot afterwards if the NVIDIA driver is still absent.",
            );
            if ui.button("Restore Standard").clicked() {
                match self.backend.set_dgpu_disabled(false, true) {
                    Ok(()) => {
                        self.schedule_boot_gpu(None);
                        self.report(
                            "Standard restored — reboot if the NVIDIA driver is still absent"
                                .to_string(),
                            false,
                        );
                    }
                    Err(e) => {
                        self.report(format!("dgpu_disable failed: {}", friendly_error(&e)), true)
                    }
                }
                self.poll();
            }
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
            ui.colored_label(egui::Color32::from_rgb(120, 200, 120), format!("• {item}"));
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
        ui.add_space(12.0);
        ui.heading("Diagnostics");
        ui.label(
            "Generate a local log bundle with versions, capabilities and the current UI state.",
        );
        if ui.button("Generate log data").clicked() {
            let state = std::env::var("XDG_STATE_HOME").unwrap_or_else(|_| {
                format!("{}/.local/state", std::env::var("HOME").unwrap_or_default())
            });
            let dir = std::path::PathBuf::from(state).join("tuf-crate");
            let _ = std::fs::create_dir_all(&dir);
            let path = dir.join("diagnostics.txt");
            let mut text = String::new();
            text.push_str(&format!("tuf-crate {}\n", env!("CARGO_PKG_VERSION")));
            text.push_str(&format!("generated: {:?}\n", std::time::SystemTime::now()));
            text.push_str(&format!(
                "backend: {} (writes {})\n",
                self.backend.name(),
                self.writes_enabled
            ));
            text.push_str(&format!("model: {}\n", self.backend.capabilities().model));
            if let Ok(v) = std::fs::read_to_string("/proc/sys/kernel/osrelease") {
                text.push_str(&format!("kernel: {}\n", v.trim()));
            }
            if let Ok(v) = std::fs::read_to_string("/etc/debian_version") {
                text.push_str(&format!("debian: {}\n", v.trim()));
            }
            text.push_str(&format!(
                "caps: {}\n",
                serde_json::to_string(self.backend.capabilities()).unwrap_or_default()
            ));
            text.push_str(&format!("theme: {:?}\n", self.theme));
            text.push_str(&format!(
                "rgb: mode {} #{:02X}{:02X}{:02X}\n",
                self.rgb_mode, self.rgb_color[0], self.rgb_color[1], self.rgb_color[2]
            ));
            if let Some((msg, err)) = &self.status_msg {
                text.push_str(&format!("last status: {msg} (error={err})\n"));
            }
            match std::fs::write(&path, text) {
                Ok(()) => self.report(format!("diagnostics written to {}", path.display()), false),
                Err(e) => self.report(format!("diagnostics failed: {e}"), true),
            }
        }
        ui.add_space(6.0);
        ui.colored_label(
            text_muted(),
            "Notice, Privacy Policy, EULA, Responsible AI and Contact ASUS are ASUS Windows documents; this project documents its own policy in docs/SAFETY.md and docs/SCOPE.md.",
        );
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
        assert!(
            loud.iter().map(|v| *v as u32).sum::<u32>()
                > dim.iter().map(|v| *v as u32).sum::<u32>()
        );
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
