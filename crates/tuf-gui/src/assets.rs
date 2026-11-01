//! Local vendor-reference assets (Armoury Crate resources).
//!
//! Proprietary ASUS files are **not** redistributed: they live in the gitignored
//! `assets/vendor-reference/` directory extracted from the owner's Windows install.
//! Everything here is best-effort — the application runs with its original vector
//! fallbacks when the directory is absent.

use std::path::PathBuf;

use eframe::egui;

/// Locate the vendor-reference directory (env override, repo checkout, install dir).
pub fn vendor_dir() -> Option<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(p) = std::env::var("TUF_CRATE_ASSETS") {
        candidates.push(PathBuf::from(p));
    }
    if let Ok(home) = std::env::var("HOME") {
        candidates.push(PathBuf::from(home).join("Projects/tuf-crate/assets/vendor-reference"));
    }
    candidates.push(PathBuf::from("assets/vendor-reference"));
    candidates.push(PathBuf::from("/usr/share/tuf-crate/vendor-reference"));
    candidates.into_iter().find(|p| p.is_dir())
}

/// Install the original Armoury Crate font when available (local use only).
pub fn load_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    // Dedicated display family: the original ROG font when available, otherwise
    // the default proportional font so the named family always exists.
    let mut rog: Vec<String> = fonts
        .families
        .get(&egui::FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    if let Some(dir) = vendor_dir() {
        let path = dir.join("fonts/ROGFonts-Regular.ttf");
        if let Ok(bytes) = std::fs::read(&path) {
            fonts
                .font_data
                .insert("rog".to_owned(), egui::FontData::from_owned(bytes));
            rog.insert(0, "rog".to_owned());
        }
    }
    fonts.families.insert(
        egui::FontFamily::Name("rog".into()),
        rog,
    );
    ctx.set_fonts(fonts);
}

/// Load a PNG/JPEG from the vendor directory as an egui texture.
pub fn load_texture(ctx: &egui::Context, rel: &str) -> Option<egui::TextureHandle> {
    let dir = vendor_dir()?;
    let path = dir.join(rel);
    let img = image::open(&path).ok()?.to_rgba8();
    let size = [img.width() as usize, img.height() as usize];
    let color = egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw());
    Some(ctx.load_texture(rel, color, egui::TextureOptions::LINEAR))
}

/// Wallpaper files: original (redistributable) wallpapers first, then any
/// locally extracted vendor wallpapers.
pub fn wallpaper_paths() -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    let mut dirs = Vec::new();
    if let Some(v) = vendor_dir() {
        if let Some(parent) = v.parent() {
            dirs.push(parent.join("wallpapers"));
        }
        dirs.push(v.join("wallpapers"));
    }
    for d in dirs {
        if let Ok(rd) = std::fs::read_dir(&d) {
            let mut files: Vec<_> = rd
                .flatten()
                .map(|e| e.path())
                .filter(|p| {
                    p.extension()
                        .map(|x| {
                            x.eq_ignore_ascii_case("png")
                                || x.eq_ignore_ascii_case("jpg")
                                || x.eq_ignore_ascii_case("jpeg")
                        })
                        .unwrap_or(false)
                })
                .collect();
            files.sort();
            for f in files {
                if !out.contains(&f) {
                    out.push(f);
                }
            }
        }
    }
    out
}

/// Load an absolute image path as a texture.
pub fn load_texture_abs(
    ctx: &egui::Context,
    key: &str,
    path: &std::path::Path,
) -> Option<egui::TextureHandle> {
    let img = image::open(path).ok()?.to_rgba8();
    let size = [img.width() as usize, img.height() as usize];
    let color = egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw());
    Some(ctx.load_texture(key, color, egui::TextureOptions::LINEAR))
}
