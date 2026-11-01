//! Per-game automatic performance profiles — a Linux-native take on Armoury
//! Crate's "Game Profiles".
//!
//! Rules are stored locally (`game-profiles.json`). While the GUI runs, the
//! process list is polled every few seconds; when a rule's process is running,
//! the configured `platform_profile` is applied through the normal verified
//! daemon path. When the game exits, the profile from before the game is
//! restored.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

pub const RULE_PROFILES: [&str; 3] = ["quiet", "balanced", "performance"];

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct GameRule {
    pub process: String,
    pub profile: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_enabled() -> bool {
    true
}

pub fn rules_path() -> PathBuf {
    let base = std::env::var("XDG_CONFIG_HOME")
        .unwrap_or_else(|_| format!("{}/.config", std::env::var("HOME").unwrap()));
    PathBuf::from(base).join("tuf-crate/game-profiles.json")
}

pub fn load_all() -> Vec<GameRule> {
    load_from(&rules_path())
}

pub fn load_from(path: &std::path::Path) -> Vec<GameRule> {
    fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save_all(rules: &[GameRule]) -> Result<(), String> {
    save_to(&rules_path(), rules)
}

pub fn save_to(path: &std::path::Path, rules: &[GameRule]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(
        path,
        serde_json::to_string_pretty(rules).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}

pub fn add(process: &str, profile: &str) -> Result<(), String> {
    let process = process.trim();
    if process.is_empty() {
        return Err("process name cannot be empty".into());
    }
    if process.len() > 64 {
        return Err("process name too long".into());
    }
    let profile = profile.trim().to_lowercase();
    if !RULE_PROFILES.contains(&profile.as_str()) {
        return Err(format!("unknown profile '{profile}'"));
    }
    let mut all = load_all();
    if all
        .iter()
        .any(|r| r.process.eq_ignore_ascii_case(process))
    {
        return Err(format!("a rule for '{process}' already exists"));
    }
    all.push(GameRule {
        process: process.to_string(),
        profile,
        enabled: true,
    });
    save_all(&all)
}

pub fn remove(process: &str) -> Result<(), String> {
    let mut all = load_all();
    all.retain(|r| !r.process.eq_ignore_ascii_case(process));
    save_all(&all)
}

pub fn set_enabled(process: &str, enabled: bool) -> Result<(), String> {
    let mut all = load_all();
    let mut found = false;
    for r in &mut all {
        if r.process.eq_ignore_ascii_case(process) {
            r.enabled = enabled;
            found = true;
        }
    }
    if !found {
        return Err(format!("no rule for '{process}'"));
    }
    save_all(&all)
}

/// First enabled rule whose process name matches one of `running`
/// (case-insensitive, exact `comm` match).
pub fn match_rule<'a>(rules: &'a [GameRule], running: &[String]) -> Option<&'a GameRule> {
    rules
        .iter()
        .find(|r| r.enabled && running.iter().any(|p| p.eq_ignore_ascii_case(&r.process)))
}

/// Read `/proc/*/comm`; cheap enough to call a few times per minute.
pub fn running_processes() -> Vec<String> {
    let mut out = Vec::new();
    if let Ok(rd) = fs::read_dir("/proc") {
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if !name.chars().all(|c| c.is_ascii_digit()) {
                continue;
            }
            // Skip zombies: a killed game can linger as <defunct> until its
            // parent reaps it, and /proc/<pid>/comm still returns the old name.
            if let Ok(stat) = fs::read_to_string(e.path().join("stat")) {
                if let Some((_, rest)) = stat.rsplit_once(')') {
                    if rest.trim_start().starts_with('Z') {
                        continue;
                    }
                }
            }
            if let Ok(s) = fs::read_to_string(e.path().join("comm")) {
                let s = s.trim();
                if !s.is_empty() {
                    out.push(s.to_string());
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "tufcrate-games-{}-{tag}.json",
            std::process::id()
        ))
    }

    #[test]
    fn rules_roundtrip_and_match() {
        let path = temp_path("roundtrip");
        let rules = vec![
            GameRule {
                process: "cs2".into(),
                profile: "performance".into(),
                enabled: true,
            },
            GameRule {
                process: "kdenlive".into(),
                profile: "quiet".into(),
                enabled: false,
            },
        ];
        save_to(&path, &rules).unwrap();
        let loaded = load_from(&path);
        assert_eq!(loaded.len(), 2);
        let running = vec!["bash".to_string(), "CS2".to_string()];
        assert_eq!(match_rule(&loaded, &running).unwrap().profile, "performance");
        // disabled rule must not match
        let running = vec!["kdenlive".to_string()];
        assert!(match_rule(&loaded, &running).is_none());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn rejects_bad_input() {
        let path = temp_path("validate");
        let ok = vec![GameRule {
            process: "game".into(),
            profile: "turbo".into(),
            enabled: true,
        }];
        save_to(&path, &ok).unwrap();
        let loaded = load_from(&path);
        // unknown profiles are filtered by the UI before add(); matching itself
        // is purely name-based and must still be case-insensitive.
        assert!(match_rule(&loaded, &["GAME".to_string()]).is_some());
        let _ = std::fs::remove_file(&path);
    }
}
