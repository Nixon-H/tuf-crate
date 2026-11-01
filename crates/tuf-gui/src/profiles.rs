//! Local lighting-colour profiles — persisted in the user's config dir.
//! Purely local state: these never touch hardware (the keyboard colour
//! write path is firmware-blocked on this model; see docs/RESEARCH.md §14).

use std::fs;
use std::path::PathBuf;

pub fn profiles_path() -> PathBuf {
    let base = std::env::var("XDG_CONFIG_HOME")
        .unwrap_or_else(|_| format!("{}/.config", std::env::var("HOME").unwrap()));
    PathBuf::from(base).join("tuf-crate/lighting-profiles.json")
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct Profile {
    pub name: String,
    pub rgb: [u8; 3],
    pub hue: f32,
}

pub fn load_all() -> Vec<Profile> {
    fs::read_to_string(profiles_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save_profile(name: &str, rgb: [u8; 3], hue: f32) -> Result<(), String> {
    if name.trim().is_empty() {
        return Err("empty profile name".into());
    }
    let mut all = load_all();
    all.retain(|p| p.name != name);
    all.push(Profile {
        name: name.trim().to_string(),
        rgb,
        hue,
    });
    if let Some(parent) = profiles_path().parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(
        profiles_path(),
        serde_json::to_string_pretty(&all).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}

pub fn delete_profile(name: &str) -> Result<(), String> {
    let mut all = load_all();
    all.retain(|p| p.name != name);
    fs::write(
        profiles_path(),
        serde_json::to_string_pretty(&all).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_roundtrip_and_delete() {
        // isolated config dir for the test
        std::env::set_var(
            "XDG_CONFIG_HOME",
            std::env::temp_dir().join("tufcrate-test-cfg"),
        );
        let _ = fs::remove_file(profiles_path());
        assert!(load_all().is_empty());
        save_profile("Night Red", [255, 0, 40], 350.0).unwrap();
        save_profile("Night Red", [255, 0, 90], 350.0).unwrap(); // replaces same name
        let all = load_all();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].rgb, [255, 0, 90]);
        assert_eq!(all[0].name, "Night Red");
        save_profile("Ice", [0, 128, 255], 200.0).unwrap();
        assert_eq!(load_all().len(), 2);
        delete_profile("Night Red").unwrap();
        let all = load_all();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].name, "Ice");
        assert!(save_profile("  ", [0, 0, 0], 0.0).is_err()); // empty name rejected
        let _ = fs::remove_file(profiles_path());
        std::env::remove_var("XDG_CONFIG_HOME");
    }
}
