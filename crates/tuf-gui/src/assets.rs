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

/// Armoury-style series key derived from the product family string (DMI),
/// never from a hard-coded model list. Dropping a `series-<key>.png` into
/// `hero-device/` instantly covers every model Armoury groups under that
/// series, so a ROG machine shows ROG artwork and a TUF machine shows TUF.
pub fn series_key(family: &str) -> Option<&'static str> {
    let f = family.to_ascii_lowercase();
    const PAIRS: [(&str, &str); 7] = [
        ("proart", "proart"),
        ("rog", "rog"),
        ("tuf", "tuf"),
        ("zenbook", "zenbook"),
        ("vivobook", "vivobook"),
        ("expertbook", "expertbook"),
        ("chromebook", "chromebook"),
    ];
    PAIRS
        .iter()
        .find(|(kw, _)| f.contains(kw))
        .map(|(_, key)| *key)
}

/// User-local cache for runtime-downloaded hero renders — the recreation of
/// Armoury Crate's `LocalState\ProductPNG` (its downloaded images live there,
/// keyed by ASUS part number). Kept outside the repository on purpose: the
/// cache is machine-local and never committed.
pub fn hero_cache_dir() -> std::path::PathBuf {
    let base = std::env::var("XDG_DATA_HOME")
        .ok()
        .filter(|v| !v.is_empty())
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            let home = std::env::var("HOME").unwrap_or_default();
            std::path::PathBuf::from(home).join(".local/share")
        });
    base.join("tuf-crate/hero-cache")
}

/// One hero image to download: destination file name inside the cache plus
/// its HTTPS source. Built from `hero-urls.json` in the vendor hero-device
/// directory — the recreation of Armoury's cloud "device image" lookup using
/// ASUS's public CDN.
#[derive(Debug, PartialEq, Eq)]
pub struct HeroSource {
    pub name: String,
    pub url: String,
}

fn hero_manifest() -> Option<serde_json::Value> {
    let path = vendor_dir()?.join("hero-device/hero-urls.json");
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

/// Sources for the detected model, in try order: exact model entry first
/// (`{MODEL}`), then the DMI-family series entry (`series:<key>`).
/// Keys map to cache file names: `png` -> `{MODEL}.png`, `alt` ->
/// `{MODEL}_alt.png`, `line` -> `{MODEL}_line.png` (or the series equivalent).
pub fn hero_sources(model: &str, family: &str) -> Vec<HeroSource> {
    let Some(man) = hero_manifest() else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let mut push_entry = |entry: &str, prefix: String| {
        let Some(obj) = man.get(entry).and_then(|v| v.as_object()) else {
            return;
        };
        for (kind, targets) in [
            ("png", format!("{prefix}.png")),
            ("alt", format!("{prefix}_alt.png")),
            ("line", format!("{prefix}_line.png")),
        ] {
            for url in obj
                .get(kind)
                .and_then(|v| v.as_array())
                .into_iter()
                .flatten()
                .filter_map(|u| u.as_str())
            {
                out.push(HeroSource {
                    name: targets.clone(),
                    url: url.to_string(),
                });
            }
        }
    };
    push_entry(model, model.to_string());
    if let Some(key) = series_key(family) {
        push_entry(&format!("series:{key}"), format!("series-{key}"));
    }
    out
}

/// Does the named hero file exist in the runtime cache or any local asset dir?
pub fn hero_file_present(name: &str) -> bool {
    if hero_cache_dir().join(name).is_file() {
        return true;
    }
    vendor_dir()
        .map(|v| v.join("hero-device").join(name).is_file())
        .unwrap_or(false)
}

/// Download one hero image into the cache with curl (bounded, like every
/// other subprocess in this crate), validate it is a real PNG/JPEG, then
/// move it into place atomically. Never panics; returns a friendly error.
pub fn fetch_hero_file(url: &str, dest: &std::path::Path) -> Result<(), String> {
    if !url.starts_with("https://") {
        return Err("hero source must be https".into());
    }
    let dir = dest
        .parent()
        .ok_or_else(|| "destination has no parent".to_string())?;
    std::fs::create_dir_all(dir).map_err(|e| format!("cache dir: {e}"))?;
    let tmp = dest.with_extension("part");
    let out = std::process::Command::new("curl")
        .args([
            "-fsSL",
            "--max-time",
            "20",
            "--max-filesize",
            "20971520",
            "-A",
            "Mozilla/5.0 (X11; Linux x86_64) tuf-crate-hero-fetch",
            "-o",
        ])
        .arg(&tmp)
        .arg(url)
        .output()
        .map_err(|e| format!("curl unavailable: {e}"))?;
    if !out.status.success() {
        let _ = std::fs::remove_file(&tmp);
        return Err(format!(
            "download failed (curl exit {:?}){}",
            out.status.code(),
            if !out.stderr.is_empty() {
                format!(": {}", String::from_utf8_lossy(&out.stderr).trim())
            } else {
                String::new()
            }
        ));
    }
    let magic_ok = std::fs::read(&tmp)
        .map(|b| {
            b.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A])
                || b.starts_with(&[0xFF, 0xD8, 0xFF])
        })
        .unwrap_or(false);
    if !magic_ok {
        let _ = std::fs::remove_file(&tmp);
        return Err("downloaded file is not a valid image".into());
    }
    std::fs::rename(&tmp, dest).map_err(|e| format!("cache write: {e}"))
}

/// The machine's ASUS part number, required by the cloud device-image API.
/// Sources, first match wins (all user-readable):
///   1. `/run/tuf-crate/part_number` — dropped by the privileged daemon from
///      SMBIOS OEM strings, so it works on every ASUS model with no setup
///   2. `TUF_CRATE_PART_NUMBER` environment override
///   3. `~/.config/tuf-crate/part-number` user override (one line)
///   4. `~/.local/share/tuf-crate/part_number` persisted by the installer
///   5. `/sys/class/dmi/id/product_sku` (populated by some ASUS models)
///
/// Validated against the ASUS PN shape so junk values are rejected.
pub fn part_number() -> Option<String> {
    fn valid(s: &str) -> bool {
        let t = s.trim();
        (6..=40).contains(&t.len())
            && t.starts_with("90")
            && t.chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-')
    }
    let mut candidates: Vec<String> = Vec::new();
    if let Ok(v) = std::env::var("TUF_CRATE_PART_NUMBER") {
        candidates.push(v);
    }
    let home = std::env::var("HOME").unwrap_or_default();
    for p in [
        "/run/tuf-crate/part_number".to_string(),
        format!("{home}/.config/tuf-crate/part-number"),
        format!("{home}/.local/share/tuf-crate/part_number"),
        "/sys/class/dmi/id/product_sku".to_string(),
    ] {
        if let Ok(v) = std::fs::read_to_string(p) {
            candidates.push(v);
        }
    }
    candidates
        .into_iter()
        .map(|c| c.trim().to_string())
        .find(|c| valid(c))
}

/// Parse the Odin product-info response and return the official render URL.
/// Pure so it is unit-tested without network. Only ASUS CDN HTTPS URLs pass.
pub fn parse_odin_image_path(body: &str) -> Result<String, String> {
    let doc: serde_json::Value =
        serde_json::from_str(body).map_err(|e| format!("bad product-info response: {e}"))?;
    let first = doc
        .get("Result")
        .and_then(|r| r.as_array())
        .and_then(|a| a.first())
        .ok_or_else(|| "part number not found in the ASUS catalogue".to_string())?;
    let url = first
        .get("ImagePath")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "product-info response has no ImagePath".to_string())?;
    if !url.starts_with("https://dlcdnwebimgs.asus.com/") {
        return Err("product image is not on the ASUS image CDN".into());
    }
    Ok(url.to_string())
}

/// Ask ASUS's public product-info API (the endpoint Armoury Crate itself uses)
/// for the part number's official render. Bounded curl call, no auth.
pub fn fetch_odin_image_path(pn: &str) -> Result<String, String> {
    let url = format!(
        "https://odinapi.asus.com/apiv2/GetPDInfoByMultiplePID?SystemCode=asus&WebsiteCode=global&partNumber={pn}&ModelType=3"
    );
    let out = std::process::Command::new("curl")
        .args([
            "-fsSL",
            "--max-time",
            "15",
            "--max-filesize",
            "1048576",
            "-A",
            "Mozilla/5.0 (X11; Linux x86_64) tuf-crate-hero-fetch",
            &url,
        ])
        .output()
        .map_err(|e| format!("curl unavailable: {e}"))?;
    if !out.status.success() {
        return Err(format!("product-info lookup failed (exit {:?})", out.status.code()));
    }
    parse_odin_image_path(&String::from_utf8_lossy(&out.stdout))
}

/// Spawn the background hero download for the detected model/series. The GUI
/// polls the returned receiver; UI never blocks on the network. With `force`
/// (the Reload Device Image affordance) already-cached renders are removed and
/// fetched again. Sends Ok with the file names downloaded this run (empty when
/// everything was already up to date), Err when the network failed.
///
/// Primary source is the same cloud lookup Armoury Crate uses: ASUS part
/// number -> `odinapi.asus.com` -> official `dlcdnwebimgs.asus.com` render
/// (byte-identical to Armoury's `LocalState\ProductPNG` file). The
/// `hero-urls.json` manifest is the fallback for machines whose part number is
/// not available, keeping every ASUS gaming model covered by at least the
/// series render.
pub fn spawn_hero_fetch(
    model: String,
    family: String,
    force: bool,
) -> std::sync::mpsc::Receiver<Result<Vec<String>, String>> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut fetched = Vec::new();
        let mut first_err: Option<String> = None;

        // 1) exact model render via the ASUS product-info API
        let model_dest = hero_cache_dir().join(format!("{model}.png"));
        if force {
            let _ = std::fs::remove_file(&model_dest);
        }
        if !model_dest.is_file() {
            if let Some(pn) = part_number() {
                match fetch_odin_image_path(&pn)
                    .and_then(|url| fetch_hero_file(&url, &model_dest))
                {
                    Ok(()) => fetched.push(format!("{model}.png")),
                    Err(e) => first_err = Some(e),
                }
            }
        }

        // 2) manifest sources (series fallback and PN-less machines)
        let sources = hero_sources(&model, &family);
        if fetched.is_empty() && sources.is_empty() && first_err.is_none() {
            let _ = tx.send(Err(format!(
                "no ASUS device-image source registered for {model}"
            )));
            return;
        }
        // A fetched exact-model render already covers the series fallback; the
        // model file is usually the same artwork under a different name.
        let model_present = model_dest.is_file();
        for src in &sources {
            if model_present && src.name.starts_with("series-") {
                continue;
            }
            let dest = hero_cache_dir().join(&src.name);
            if force {
                let _ = std::fs::remove_file(&dest);
            } else if dest.is_file() {
                continue;
            }
            match fetch_hero_file(&src.url, &dest) {
                Ok(()) => fetched.push(src.name.clone()),
                Err(e) => {
                    if first_err.is_none() {
                        first_err = Some(e);
                    }
                }
            }
        }
        if fetched.is_empty() {
            if let Some(err) = first_err {
                let _ = tx.send(Err(err));
                return;
            }
        }
        let _ = tx.send(Ok(fetched));
    });
    rx
}

/// Load the model's local hero render (official ASUS product artwork). The
/// runtime download cache (`hero-cache`) is consulted first — Armoury's own
/// ProductPNG behaviour — then the exact model file, then the DMI family's
/// series file. When none exists the caller draws the neutral procedural
/// laptop instead of wrong-brand artwork.
pub fn hero_texture(
    ctx: &egui::Context,
    model: &str,
    family: &str,
    line_art: bool,
) -> Option<egui::TextureHandle> {
    let dir = vendor_dir()?.join("hero-device");
    let mut candidates: Vec<String> = if line_art {
        vec![
            format!("{model}_line.png"),
            format!("{model}.png"),
            format!("{model}_alt.png"),
        ]
    } else {
        vec![
            format!("{model}.png"),
            format!("{model}_alt.png"),
            format!("{model}_line.png"),
        ]
    };
    if let Some(key) = series_key(family) {
        if line_art {
            candidates.push(format!("series-{key}_line.png"));
        } else {
            candidates.push(format!("series-{key}.png"));
            candidates.push(format!("series-{key}_alt.png"));
        }
    }
    for name in candidates {
        // runtime download cache first (Armoury's ProductPNG behaviour)…
        let cache_path = hero_cache_dir().join(&name);
        if cache_path.is_file() {
            if let Some(tex) = load_texture_abs(ctx, &format!("hero-cache-{name}"), &cache_path) {
                return Some(tex);
            }
        }
        // …then the repo/vendor asset dirs
        let path = dir.join(&name);
        if path.is_file() {
            if let Some(tex) = load_texture_abs(ctx, &format!("hero-{name}"), &path) {
                return Some(tex);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::series_key;

    #[test]
    fn odin_response_parses_the_exact_product_render_url() {
        // trimmed copy of the live FX706HF response
        let body = concat!(
            r#"{"Status":"200","Message":"Success","Result":[{"ProductID":15630,"#,
            r#""partNumber":"90NR0HC4-M003D0","ProductMarketName":"2021 ASUS TUF Gaming F17","#,
            r#""ImagePath":"https://dlcdnwebimgs.asus.com/gain/51099666-2767-4410-99b8-cae7a7de8a95/"}]}"#
        );
        assert_eq!(
            super::parse_odin_image_path(body).unwrap(),
            "https://dlcdnwebimgs.asus.com/gain/51099666-2767-4410-99b8-cae7a7de8a95/"
        );
        // empty catalogue result (model names are not part numbers)
        let empty = r#"{"Status":"200","Message":"Success","Result":[]}"#;
        assert!(super::parse_odin_image_path(empty).is_err());
        // a non-ASUS CDN URL is rejected
        let foreign = r#"{"Result":[{"ImagePath":"https://evil.example/x.png"}]}"#;
        assert!(super::parse_odin_image_path(foreign).is_err());
    }

    #[test]
    fn part_number_rejects_junk_and_accepts_override() {
        std::env::set_var("TUF_CRATE_PART_NUMBER", "90NR0HC4-M003D0");
        assert_eq!(super::part_number().as_deref(), Some("90NR0HC4-M003D0"));
        std::env::set_var("TUF_CRATE_PART_NUMBER", "not-a-part");
        // junk env is skipped; files/DMI may supply a real one
        let got = super::part_number();
        assert!(got.as_deref() != Some("not-a-part"));
        std::env::remove_var("TUF_CRATE_PART_NUMBER");
    }

    #[test]
    fn hero_sources_resolves_model_then_series_and_names() {
        // prepare a temp vendor dir with a manifest; vendor_dir() honours
        // TUF_CRATE_ASSETS
        let dir = std::env::temp_dir().join(format!("tuf-hero-test-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("hero-device")).unwrap();
        std::fs::write(
            dir.join("hero-device/hero-urls.json"),
            r#"{"FX706HF":{"png":["https://cdn/pd_01.png"],"alt":["https://cdn/pd_02.png"]},
                "series:tuf":{"png":["https://cdn/series.png"]}}"#,
        )
        .unwrap();
        std::env::set_var("TUF_CRATE_ASSETS", &dir);
        let srcs = super::hero_sources("FX706HF", "ASUS TUF Gaming F17");
        let names: Vec<&str> = srcs.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(
            names,
            vec!["FX706HF.png", "FX706HF_alt.png", "series-tuf.png"]
        );
        assert_eq!(srcs[0].url, "https://cdn/pd_01.png");
        // a machine without its own manifest entry still gets the series file
        let other = super::hero_sources("FA506IV", "ASUS TUF Gaming A15");
        let names: Vec<&str> = other.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, vec!["series-tuf.png"]);
        std::env::remove_var("TUF_CRATE_ASSETS");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn fetch_hero_file_rejects_non_images_and_plain_http() {
        let dir = std::env::temp_dir().join(format!("tuf-hero-fetch-{}", std::process::id()));
        let dest = dir.join("x.png");
        assert!(super::fetch_hero_file("http://cdn/x.png", &dest).is_err());
        // curl file:// is disabled by the https guard; a bogus https URL fails
        // without creating a destination file
        assert!(super::fetch_hero_file("https://localhost.invalid/x.png", &dest).is_err());
        assert!(!dest.exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn series_key_follows_the_family_string_not_a_model_list() {
        assert_eq!(series_key("ASUS TUF Gaming F17"), Some("tuf"));
        assert_eq!(series_key("ASUS ROG Strix G16"), Some("rog"));
        assert_eq!(series_key("ASUS ProArt Studiobook"), Some("proart"));
        assert_eq!(series_key("ASUS Zenbook UX3402"), Some("zenbook"));
        assert_eq!(series_key("ASUS Vivobook S15"), Some("vivobook"));
        assert_eq!(series_key("ASUS ExpertBook B1"), Some("expertbook"));
        assert_eq!(series_key("Generic Laptop"), None);
        assert_eq!(series_key(""), None);
    }
}
