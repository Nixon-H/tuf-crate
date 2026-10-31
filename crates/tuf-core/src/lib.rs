//! tuf-core: capability model, safety layer and audit log for tuf-crate.
//!
//! Safety model (see docs/SAFETY.md):
//! - Hardware writes are only representable as `VerifiedAttr`, a closed enum of
//!   kernel-sanctioned, model-verified sysfs attributes. Raw paths cannot be
//!   written through this crate by construction.
//! - Every value passes `clamp()` before reaching the HAL; out-of-range or
//!   unverified operations return `SafetyError`.
//! - Every accepted write is recorded in a JSONL audit log.

use serde::Serialize;
use std::fs::OpenOptions;
use std::io::Write as IoWrite;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

/// Operating profile — mirrors ACPI platform_profile choices on FX706HF.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Profile {
    Quiet,
    Balanced,
    Performance,
}

impl Profile {
    pub fn as_str(self) -> &'static str {
        match self {
            Profile::Quiet => "quiet",
            Profile::Balanced => "balanced",
            Profile::Performance => "performance",
        }
    }
    pub fn from_str(s: &str) -> Option<Profile> {
        match s.trim() {
            "quiet" | "low-power" => Some(Profile::Quiet),
            "balanced" => Some(Profile::Balanced),
            "performance" => Some(Profile::Performance),
            _ => None,
        }
    }
    pub fn all() -> [Profile; 3] {
        [Profile::Quiet, Profile::Balanced, Profile::Performance]
    }
    /// Armoury Crate label for the UI.
    pub fn label(self) -> &'static str {
        match self {
            Profile::Quiet => "Silent",
            Profile::Balanced => "Balanced",
            Profile::Performance => "Performance (Turbo)",
        }
    }
}

/// The closed set of hardware writes this application may ever perform.
/// Anything not in this enum is unrepresentable — that is the safety boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerifiedAttr {
    /// /sys/firmware/acpi/platform_profile — thermal/fan policy
    PlatformProfile,
    /// /sys/class/power_supply/BAT1/charge_control_end_threshold — percent
    ChargeThreshold,
    /// /sys/class/leds/asus::kbd_backlight/brightness — 0..max
    KbdBrightness,
    /// /sys/devices/platform/asus-nb-wmi/panel_od — 0/1
    PanelOverdrive,
    /// /sys/devices/platform/asus-nb-wmi/dgpu_disable — 0/1 (gated: confirmation required)
    DgpuDisable,
}

impl VerifiedAttr {
    /// Safe clamping range for integer-valued attrs.
    pub fn safe_range(self) -> Option<(u32, u32)> {
        match self {
            VerifiedAttr::ChargeThreshold => Some((20, 100)),
            VerifiedAttr::KbdBrightness => Some((0, 3)), // re-clamped to max at runtime
            VerifiedAttr::PanelOverdrive | VerifiedAttr::DgpuDisable => Some((0, 1)),
            VerifiedAttr::PlatformProfile => None, // string-valued
        }
    }
    /// Attributes that must never be applied without an explicit UI confirmation.
    pub fn requires_confirmation(self) -> bool {
        matches!(self, VerifiedAttr::DgpuDisable)
    }
}

#[derive(Debug, Error)]
pub enum SafetyError {
    #[error("value {value} outside safe range {min}..={max} for {attr:?}")]
    OutOfRange {
        attr: VerifiedAttr,
        value: u32,
        min: u32,
        max: u32,
    },
    #[error("capability not present on this hardware: {0}")]
    MissingCapability(&'static str),
    #[error("{0} requires explicit user confirmation")]
    ConfirmationRequired(&'static str),
    #[error("unknown platform profile: {0}")]
    UnknownProfile(String),
}

/// Clamp a value into the attr's safe range.
pub fn clamp(attr: VerifiedAttr, value: u32, led_max: u32) -> Result<u32, SafetyError> {
    let (min, max) = match attr.safe_range() {
        Some(r) => r,
        None => return Ok(value),
    };
    // KbdBrightness max is dynamic (kernel exposes max_brightness); never exceed it.
    let max = if attr == VerifiedAttr::KbdBrightness {
        max.min(led_max)
    } else {
        max
    };
    if value < min || value > max {
        return Err(SafetyError::OutOfRange {
            attr,
            value,
            min,
            max,
        });
    }
    Ok(value)
}

/// Capabilities detected on the running system. Absent capability = GUI hides
/// the control AND HAL refuses the write.
#[derive(Debug, Clone, Default, Serialize)]
pub struct CapabilitySet {
    pub platform_profile: bool,
    pub profile_choices: Vec<String>,
    pub charge_threshold: bool,
    pub kbd_backlight: bool,
    pub kbd_max_brightness: u32,
    pub panel_overdrive: bool,
    pub dgpu_disable: bool,
    pub firmware_attrs: Vec<String>,
    pub fans: Vec<String>,
    pub battery: bool,
    pub nvidia: bool,
    pub model: String,
    pub is_fx706hf: bool,
}

impl CapabilitySet {
    pub fn allows(&self, attr: VerifiedAttr) -> bool {
        match attr {
            VerifiedAttr::PlatformProfile => self.platform_profile,
            VerifiedAttr::ChargeThreshold => self.charge_threshold,
            VerifiedAttr::KbdBrightness => self.kbd_backlight,
            VerifiedAttr::PanelOverdrive => self.panel_overdrive,
            VerifiedAttr::DgpuDisable => self.dgpu_disable,
        }
    }
    /// Validate a write against capabilities + confirmation policy.
    pub fn validate(&self, attr: VerifiedAttr, confirmed: bool) -> Result<(), SafetyError> {
        if !self.allows(attr) {
            return Err(SafetyError::MissingCapability(match attr {
                VerifiedAttr::PlatformProfile => "platform_profile",
                VerifiedAttr::ChargeThreshold => "charge_control_end_threshold",
                VerifiedAttr::KbdBrightness => "asus::kbd_backlight",
                VerifiedAttr::PanelOverdrive => "panel_od",
                VerifiedAttr::DgpuDisable => "dgpu_disable",
            }));
        }
        if attr.requires_confirmation() && !confirmed {
            return Err(SafetyError::ConfirmationRequired("dgpu_disable"));
        }
        Ok(())
    }
}

/// JSONL audit trail of every accepted hardware write.
pub struct AuditLog {
    path: Option<PathBuf>,
}

#[derive(Serialize)]
struct AuditEntry<'a> {
    ts_unix: u64,
    attr: &'a str,
    value: String,
    result: &'a str,
}

impl AuditLog {
    pub fn disabled() -> Self {
        AuditLog { path: None }
    }
    pub fn at(path: PathBuf) -> Self {
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        AuditLog { path: Some(path) }
    }
    pub fn record(&self, attr: VerifiedAttr, value: &str) {
        let Some(p) = &self.path else { return };
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let entry = AuditEntry {
            ts_unix: ts,
            attr: match attr {
                VerifiedAttr::PlatformProfile => "platform_profile",
                VerifiedAttr::ChargeThreshold => "charge_threshold",
                VerifiedAttr::KbdBrightness => "kbd_brightness",
                VerifiedAttr::PanelOverdrive => "panel_overdrive",
                VerifiedAttr::DgpuDisable => "dgpu_disable",
            },
            value: value.to_string(),
            result: "applied",
        };
        if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(p) {
            let _ = writeln!(f, "{}", serde_json::to_string(&entry).unwrap_or_default());
        }
    }
}

/// Model gate: writes are allowed only on the model this project was verified for.
pub fn is_fx706hf(product_name: &str) -> bool {
    let p = product_name.to_uppercase();
    p.contains("FX706HF")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamps_charge_threshold() {
        assert_eq!(clamp(VerifiedAttr::ChargeThreshold, 80, 3).unwrap(), 80);
        assert_eq!(
            clamp(VerifiedAttr::ChargeThreshold, 10, 3)
                .unwrap_err()
                .to_string(),
            "value 10 outside safe range 20..=100 for ChargeThreshold"
        );
        assert_eq!(
            clamp(VerifiedAttr::ChargeThreshold, 150, 3)
                .unwrap_err()
                .to_string(),
            "value 150 outside safe range 20..=100 for ChargeThreshold"
        );
    }

    #[test]
    fn clamps_brightness_to_led_max() {
        // Strict policy: out-of-range values error rather than clamp silently.
        assert_eq!(clamp(VerifiedAttr::KbdBrightness, 2, 3).unwrap(), 2);
        assert!(clamp(VerifiedAttr::KbdBrightness, 3, 2).is_err());
        assert!(clamp(VerifiedAttr::KbdBrightness, 5, 3).is_err());
    }

    #[test]
    fn dgpu_requires_confirmation() {
        assert!(VerifiedAttr::DgpuDisable.requires_confirmation());
        assert!(!VerifiedAttr::PanelOverdrive.requires_confirmation());
    }

    #[test]
    fn capability_gate_blocks_missing_and_unconfirmed() {
        let mut caps = CapabilitySet::default();
        caps.panel_overdrive = true;
        assert!(caps.validate(VerifiedAttr::PanelOverdrive, false).is_ok());
        // capability check precedes confirmation check (cannot confirm an absent attr)
        let err = caps
            .validate(VerifiedAttr::ChargeThreshold, true)
            .unwrap_err();
        assert!(matches!(err, SafetyError::MissingCapability(_)));
        let err = caps.validate(VerifiedAttr::DgpuDisable, false).unwrap_err();
        assert!(matches!(err, SafetyError::MissingCapability(_)));
        caps.dgpu_disable = true;
        let err = caps.validate(VerifiedAttr::DgpuDisable, false).unwrap_err();
        assert!(matches!(err, SafetyError::ConfirmationRequired(_)));
        assert!(caps.validate(VerifiedAttr::DgpuDisable, true).is_ok());
    }

    #[test]
    fn model_gate() {
        assert!(is_fx706hf("ASUS TUF Gaming F17 FX706HF_FX706HF"));
        assert!(!is_fx706hf("Random Laptop"));
    }
}
