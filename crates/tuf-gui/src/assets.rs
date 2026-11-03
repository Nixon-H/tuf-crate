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
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            // ~/.local/bin/tuf-crate -> ~/.local/share/tuf-crate/vendor-reference
            candidates.push(dir.join("../share/tuf-crate/vendor-reference"));
            // cargo target/release/tuf-crate -> <repo>/assets/vendor-reference
            candidates.push(dir.join("../../assets/vendor-reference"));
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        candidates.push(PathBuf::from(&home).join("Projects/tuf-crate/assets/vendor-reference"));
        candidates.push(PathBuf::from(home).join(".local/share/tuf-crate/vendor-reference"));
    }
    candidates.push(PathBuf::from("assets/vendor-reference"));
    candidates.push(PathBuf::from("/usr/share/tuf-crate/vendor-reference"));
    candidates.push(PathBuf::from("/usr/local/share/tuf-crate/vendor-reference"));
    candidates.into_iter().find_map(|p| {
        if p.is_dir() {
            Some(std::fs::canonicalize(&p).unwrap_or(p))
        } else {
            None
        }
    })
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

/// Chassis/product serial written by the installer (root can read DMI; the GUI
/// user cannot). Falls back to direct DMI if permissions allow.
pub fn chassis_serial() -> Option<String> {
    let mut paths = Vec::new();
    if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
        paths.push(PathBuf::from(xdg).join("tuf-crate/chassis_serial"));
    }
    if let Ok(home) = std::env::var("HOME") {
        paths.push(PathBuf::from(home).join(".local/share/tuf-crate/chassis_serial"));
    }
    for name in ["chassis_serial", "product_serial", "board_serial"] {
        paths.push(PathBuf::from("/sys/class/dmi/id").join(name));
    }
    for path in paths {
        let Ok(raw) = std::fs::read_to_string(path) else {
            continue;
        };
        let value = raw.trim();
        let lower = value.to_ascii_lowercase();
        if value.is_empty()
            || lower == "default string"
            || lower == "none"
            || lower == "unknown"
            || lower.contains("to be filled")
        {
            continue;
        }
        return Some(value.to_string());
    }
    None
}

/// Detect the laptop model from DMI (e.g. "FX706HF").
pub fn detect_model() -> String {
    for path in ["/sys/class/dmi/id/board_name", "/sys/class/dmi/id/product_name"] {
        if let Ok(s) = std::fs::read_to_string(path) {
            let s = s.trim();
            if let Some(tok) = s.split_whitespace().find(|t| {
                t.len() >= 4 && t.chars().any(|c| c.is_ascii_digit())
                    && t.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
            }) {
                return tok.to_ascii_uppercase();
            }
        }
    }
    "GENERIC".to_string()
}

/// Human-readable product family from DMI, with the model token removed.
/// Example: "ASUS TUF Gaming F17 FX706HF_FX706HF" + FX706HF -> "ASUS TUF Gaming F17".
pub fn device_family(model: &str) -> String {
    for path in ["/sys/class/dmi/id/product_name", "/sys/class/dmi/id/board_name"] {
        if let Ok(raw) = std::fs::read_to_string(path) {
            let raw = raw.trim();
            if raw.is_empty() {
                continue;
            }
            if !model.is_empty() {
                if let Some(idx) = raw.to_uppercase().find(&model.to_uppercase()) {
                    let prefix = raw[..idx].trim();
                    if !prefix.is_empty() {
                        return prefix.to_string();
                    }
                }
            }
            let cleaned = raw.split('_').next().unwrap_or(raw).trim();
            if !cleaned.is_empty() {
                return cleaned.to_string();
            }
        }
    }
    "ASUS Laptop".to_string()
}

/// Load the model's local hero render (official ASUS product artwork, local only).
pub fn hero_texture(
    ctx: &egui::Context,
    model: &str,
    line_art: bool,
) -> Option<egui::TextureHandle> {
    let dir = vendor_dir()?.join("hero-device");
    let candidates: [String; 3] = if line_art {
        [
            format!("{model}_line.png"),
            format!("{model}.png"),
            format!("{model}_alt.png"),
        ]
    } else {
        [
            format!("{model}.png"),
            format!("{model}_alt.png"),
            format!("{model}_line.png"),
        ]
    };
    for name in candidates {
        let path = dir.join(&name);
        if path.is_file() {
            if let Some(tex) = load_texture_abs(ctx, &format!("hero-{name}"), &path) {
                return Some(tex);
            }
        }
    }
    None
}
