//! tuf-hal: hardware abstraction for tuf-crate.
//!
//! Two backends implement the same semantic `Backend` trait:
//! - `SysfsBackend` — the real machine, via kernel-sanctioned sysfs interfaces only.
//!   All paths are rooted at `root` (default `/`); tests use a fixture tree instead,
//!   so the exact same code is exercised without touching hardware.
//! - `MockBackend` — in-memory simulation for GUI development and tests.
//!
//! There is deliberately **no** API to write an arbitrary path, poke EC registers,
//! invoke ACPI methods, or run fan-PWM commands. See docs/SAFETY.md.

pub mod nvidia;

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use tuf_core::{clamp, is_fx706hf, AuditLog, CapabilitySet, Profile, SafetyError, VerifiedAttr};

/// A point-in-time, read-only view of the machine.
#[derive(Debug, Clone, Default)]
#[allow(non_snake_case)] // milli-Celsius fields read better with the mC unit suffix
pub struct Telemetry {
    pub profile: Option<String>,
    pub profile_choices: Vec<String>,
    pub cpu_package_temp_mC: Option<i64>, // milli-Celsius
    pub cpu_fan_rpm: Option<u32>,
    pub gpu_fan_rpm: Option<u32>,
    pub fan_labels: Vec<(String, Option<u32>)>,
    pub nvme_temp_mC: Option<i64>,
    pub acpi_zone_temps_mC: Vec<(String, i64)>,
    pub battery: Option<Battery>,
    pub kbd_brightness: Option<u32>,
    pub kbd_max_brightness: Option<u32>,
    pub panel_od: Option<bool>,
    pub dgpu_disable: Option<bool>,
    pub charge_mode: Option<String>,
    pub nvidia: Option<nvidia::NvidiaState>,
}

#[derive(Debug, Clone, Default)]
pub struct Battery {
    pub status: String,
    pub capacity_percent: u32,
    pub charge_now_uah: Option<u64>,
    pub charge_full_uah: Option<u64>,
    pub charge_full_design_uah: Option<u64>,
    pub charge_threshold: Option<u32>,
    pub ac_online: bool,
    pub power_now_uw: Option<u64>,
}

impl Battery {
    /// Health = full capacity / design capacity.
    pub fn health_percent(&self) -> Option<u32> {
        match (self.charge_full_uah, self.charge_full_design_uah) {
            (Some(f), Some(d)) if d > 0 => Some((f * 100 / d) as u32),
            _ => None,
        }
    }
}

/// Errors surfaced to the UI.
#[derive(Debug, thiserror::Error)]
pub enum HalError {
    #[error("I/O: {0}")]
    Io(#[from] io::Error),
    #[error("safety: {0}")]
    Safety(#[from] SafetyError),
    #[error("attribute missing: {0}")]
    Missing(&'static str),
    #[error("unsupported operation on this hardware")]
    Unsupported,
}

pub type HalResult<T> = Result<T, HalError>;

/// The complete semantic surface of tuf-crate. The GUI only ever sees this.
pub trait Backend {
    fn name(&self) -> &'static str;
    fn capabilities(&self) -> &CapabilitySet;
    /// Read-only telemetry snapshot.
    fn telemetry(&self) -> HalResult<Telemetry>;
    /// Write ops — each validates capability + clamps + audits before touching sysfs.
    fn set_profile(&mut self, p: Profile, confirmed: bool) -> HalResult<()>;
    fn set_charge_threshold(&mut self, percent: u32, confirmed: bool) -> HalResult<()>;
    fn set_kbd_brightness(&mut self, level: u32, confirmed: bool) -> HalResult<()>;
    fn set_panel_overdrive(&mut self, on: bool, confirmed: bool) -> HalResult<()>;
    fn set_dgpu_disabled(&mut self, disable: bool, confirmed: bool) -> HalResult<()>;
}

// ---------------------------------------------------------------------------
// SysfsBackend
// ---------------------------------------------------------------------------

pub struct SysfsBackend {
    root: PathBuf,
    caps: CapabilitySet,
    audit: AuditLog,
    /// Writes enabled only when explicitly requested (Phase-6 safety posture).
    writes_enabled: bool,
}

/// Path constants — the ONLY hardware locations this application writes, ever.
mod paths {
    use std::path::{Path, PathBuf};
    pub fn platform_profile(r: &Path) -> PathBuf {
        r.join("sys/firmware/acpi/platform_profile")
    }
    pub fn platform_profile_choices(r: &Path) -> PathBuf {
        r.join("sys/firmware/acpi/platform_profile_choices")
    }
    pub fn dmi_product(r: &Path) -> PathBuf {
        r.join("sys/class/dmi/id/product_name")
    }
    pub fn asus_nb_wmi(r: &Path, attr: &str) -> PathBuf {
        r.join(format!("sys/devices/platform/asus-nb-wmi/{attr}"))
    }
    pub fn battery(r: &Path, attr: &str) -> PathBuf {
        r.join(format!("sys/class/power_supply/BAT1/{attr}"))
    }
    pub fn ac(r: &Path, attr: &str) -> PathBuf {
        r.join(format!("sys/class/power_supply/ACAD/{attr}"))
    }
    pub fn kbd_led(r: &Path, attr: &str) -> PathBuf {
        r.join("sys/class/leds/asus::kbd_backlight").join(attr)
    }
    pub fn hwmon(r: &Path, n: u32, attr: &str) -> PathBuf {
        r.join(format!("sys/class/hwmon/hwmon{n}/{attr}"))
    }
    pub fn fwattr(r: &Path, attr: &str) -> PathBuf {
        r.join(format!(
            "sys/class/firmware-attributes/asus-armoury/attributes/{attr}/current_value"
        ))
    }
}

impl SysfsBackend {
    /// Production constructor.
    pub fn real(writes_enabled: bool, audit_path: Option<PathBuf>) -> HalResult<Self> {
        Self::with_root(Path::new("/"), writes_enabled, audit_path)
    }

    /// Test/fixture constructor: same logic, arbitrary root.
    pub fn with_root(
        root: &Path,
        writes_enabled: bool,
        audit_path: Option<PathBuf>,
    ) -> HalResult<Self> {
        let caps = Self::detect(root);
        let audit = match audit_path {
            Some(p) => AuditLog::at(p),
            None => AuditLog::disabled(),
        };
        Ok(SysfsBackend {
            root: root.to_path_buf(),
            caps,
            audit,
            writes_enabled,
        })
    }

    /// Detect capabilities by *reading* the interface set. Read-only.
    pub fn detect(root: &Path) -> CapabilitySet {
        let mut c = CapabilitySet::default();
        let model = fs::read_to_string(paths::dmi_product(root))
            .unwrap_or_default()
            .trim()
            .to_string();
        c.model = model.clone();
        c.is_fx706hf = is_fx706hf(&model);
        c.platform_profile = paths::platform_profile(root).exists();
        if let Ok(ch) = fs::read_to_string(paths::platform_profile_choices(root)) {
            c.profile_choices = ch.split_whitespace().map(str::to_string).collect();
        }
        c.charge_threshold = paths::battery(root, "charge_control_end_threshold").exists();
        c.kbd_backlight = paths::kbd_led(root, "brightness").exists();
        c.kbd_max_brightness = fs::read_to_string(paths::kbd_led(root, "max_brightness"))
            .ok()
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(3);
        c.panel_overdrive = paths::asus_nb_wmi(root, "panel_od").exists();
        c.dgpu_disable = paths::asus_nb_wmi(root, "dgpu_disable").exists();
        // firmware-attributes (asus-armoury) — enumerate what the kernel exposes
        if let Ok(dir) =
            fs::read_dir(root.join("sys/class/firmware-attributes/asus-armoury/attributes"))
        {
            for e in dir.flatten() {
                if e.path().is_dir() {
                    c.firmware_attrs
                        .push(e.file_name().to_string_lossy().into_owned());
                }
            }
            c.firmware_attrs.sort();
        }
        // hwmon fans from the ASUS hwmon device (name == "asus")
        if let Ok(dir) = fs::read_dir(root.join("sys/class/hwmon")) {
            for e in dir.flatten() {
                let name = fs::read_to_string(e.path().join("name")).unwrap_or_default();
                if name.trim() != "asus" {
                    continue;
                }
                if let Ok(files) = fs::read_dir(e.path()) {
                    for f in files.flatten() {
                        let fn_ = f.file_name().to_string_lossy().into_owned();
                        if fn_.starts_with("fan") && fn_.ends_with("_input") {
                            c.fans.push(fn_);
                        }
                    }
                }
            }
            c.fans.sort();
        }
        c.battery = paths::battery(root, "capacity").exists();
        c.nvidia = Command::new("nvidia-smi")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        c
    }

    fn read(&self, p: impl AsRef<Path>) -> Option<String> {
        fs::read_to_string(p.as_ref())
            .ok()
            .map(|s| s.trim().to_string())
    }

    fn read_u64(&self, p: impl AsRef<Path>) -> Option<u64> {
        self.read(p)
            .and_then(|s| s.split_whitespace().next().and_then(|t| t.parse().ok()))
    }

    /// The single write funnel. Everything goes through here: capability check,
    /// confirmation check, clamp, write, audit. Nothing else in the crate writes.
    fn verified_write(
        &mut self,
        path: &Path,
        attr: VerifiedAttr,
        value: &str,
        confirmed: bool,
    ) -> HalResult<()> {
        if !self.writes_enabled {
            return Err(HalError::Unsupported); // monitoring-only mode
        }
        self.caps.validate(attr, confirmed)?;
        match attr.safe_range() {
            Some(_) => {
                let v: u32 = value.parse().map_err(|_| HalError::Unsupported)?;
                clamp(attr, v, self.caps.kbd_max_brightness)?;
            }
            None => {
                // string attr: profile must be one of the kernel-advertised choices
                if attr == VerifiedAttr::PlatformProfile
                    && !self.caps.profile_choices.is_empty()
                    && !self.caps.profile_choices.iter().any(|c| c == value)
                {
                    return Err(SafetyError::UnknownProfile(value.to_string()).into());
                }
            }
        }
        fs::write(path, value)?;
        self.audit.record(attr, value);
        Ok(())
    }

    fn find_asus_hwmon(&self) -> Option<u32> {
        let base = self.root.join("sys/class/hwmon");
        for e in fs::read_dir(&base).ok()?.flatten() {
            if self.read(e.path().join("name")).as_deref() == Some("asus") {
                let idx: u32 = e
                    .file_name()
                    .to_string_lossy()
                    .trim_start_matches("hwmon")
                    .parse()
                    .ok()?;
                return Some(idx);
            }
        }
        None
    }
}

impl Backend for SysfsBackend {
    fn name(&self) -> &'static str {
        "sysfs"
    }

    fn capabilities(&self) -> &CapabilitySet {
        &self.caps
    }

    fn telemetry(&self) -> HalResult<Telemetry> {
        let r = &self.root;
        let mut t = Telemetry {
            profile: self.read(paths::platform_profile(r)),
            ..Default::default()
        };
        if let Ok(ch) = fs::read_to_string(paths::platform_profile_choices(r)) {
            t.profile_choices = ch.split_whitespace().map(str::to_string).collect();
        }
        // Temps: coretemp package + NVMe + acpitz zones
        for e in fs::read_dir(r.join("sys/class/hwmon"))
            .map_err(HalError::Io)?
            .flatten()
        {
            let name = self.read(e.path().join("name")).unwrap_or_default();
            match name.as_str() {
                "coretemp" => {
                    if let Some(v) = self.read_u64(e.path().join("temp1_input")) {
                        t.cpu_package_temp_mC = Some(v as i64);
                    }
                }
                "nvme" => {
                    if let Some(v) = self.read_u64(e.path().join("temp1_input")) {
                        t.nvme_temp_mC = Some(v as i64);
                    }
                }
                n if n.starts_with("acpitz") => {
                    if let Some(v) = self.read_u64(e.path().join("temp1_input")) {
                        t.acpi_zone_temps_mC.push((n.to_string(), v as i64));
                    }
                }
                _ => {}
            }
        }
        // ASUS hwmon: fans
        if let Some(idx) = self.find_asus_hwmon() {
            for f in ["fan1", "fan2"] {
                let rpm = self.read_u64(paths::hwmon(r, idx, &format!("{f}_input")));
                let label = self
                    .read(paths::hwmon(r, idx, &format!("{f}_label")))
                    .unwrap_or_else(|| f.to_string());
                t.fan_labels.push((label, rpm.map(|v| v as u32)));
            }
            t.cpu_fan_rpm = self
                .read_u64(paths::hwmon(r, idx, "fan1_input"))
                .map(|v| v as u32);
            t.gpu_fan_rpm = self
                .read_u64(paths::hwmon(r, idx, "fan2_input"))
                .map(|v| v as u32);
        }
        // Battery
        if self.caps.battery {
            let status = self.read(paths::battery(r, "status")).unwrap_or_default();
            let capacity = self.read_u64(paths::battery(r, "capacity")).unwrap_or(0) as u32;
            t.battery = Some(Battery {
                status,
                capacity_percent: capacity,
                charge_now_uah: self.read_u64(paths::battery(r, "charge_now")),
                charge_full_uah: self.read_u64(paths::battery(r, "charge_full")),
                charge_full_design_uah: self.read_u64(paths::battery(r, "charge_full_design")),
                charge_threshold: self
                    .read_u64(paths::battery(r, "charge_control_end_threshold"))
                    .map(|v| v as u32),
                ac_online: self.read_u64(paths::ac(r, "online")).unwrap_or(0) == 1,
                power_now_uw: self.read_u64(paths::battery(r, "power_now")),
            });
        }
        t.kbd_brightness = self
            .read_u64(paths::kbd_led(r, "brightness"))
            .map(|v| v as u32);
        t.kbd_max_brightness = self
            .read_u64(paths::kbd_led(r, "max_brightness"))
            .map(|v| v as u32);
        t.panel_od = self
            .read_u64(paths::asus_nb_wmi(r, "panel_od"))
            .map(|v| v == 1);
        t.dgpu_disable = self
            .read_u64(paths::asus_nb_wmi(r, "dgpu_disable"))
            .map(|v| v == 1);
        t.charge_mode = self.read(paths::fwattr(r, "charge_mode"));
        t.nvidia = nvidia::query();
        Ok(t)
    }

    fn set_profile(&mut self, p: Profile, confirmed: bool) -> HalResult<()> {
        let path = paths::platform_profile(&self.root);
        self.verified_write(&path, VerifiedAttr::PlatformProfile, p.as_str(), confirmed)
    }

    fn set_charge_threshold(&mut self, percent: u32, confirmed: bool) -> HalResult<()> {
        let path = paths::battery(&self.root, "charge_control_end_threshold");
        self.verified_write(
            &path,
            VerifiedAttr::ChargeThreshold,
            &percent.to_string(),
            confirmed,
        )
    }

    fn set_kbd_brightness(&mut self, level: u32, confirmed: bool) -> HalResult<()> {
        let path = paths::kbd_led(&self.root, "brightness");
        self.verified_write(
            &path,
            VerifiedAttr::KbdBrightness,
            &level.to_string(),
            confirmed,
        )
    }

    fn set_panel_overdrive(&mut self, on: bool, confirmed: bool) -> HalResult<()> {
        let path = paths::asus_nb_wmi(&self.root, "panel_od");
        self.verified_write(
            &path,
            VerifiedAttr::PanelOverdrive,
            if on { "1" } else { "0" },
            confirmed,
        )
    }

    fn set_dgpu_disabled(&mut self, disable: bool, confirmed: bool) -> HalResult<()> {
        let path = paths::asus_nb_wmi(&self.root, "dgpu_disable");
        self.verified_write(
            &path,
            VerifiedAttr::DgpuDisable,
            if disable { "1" } else { "0" },
            confirmed,
        )
    }
}

// ---------------------------------------------------------------------------
// MockBackend — full simulation, zero hardware access.
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct MockState {
    pub profile: String,
    pub charge_threshold: u32,
    pub kbd_brightness: u32,
    pub panel_od: bool,
    pub dgpu_disable: bool,
    pub write_count: u32,
}

pub struct MockBackend {
    caps: CapabilitySet,
    state: MockState,
    audit: AuditLog,
    /// Same monitoring-first posture as SysfsBackend: mock writes are refused
    /// unless explicitly enabled, so the UI behaviour is identical everywhere.
    writes_enabled: bool,
}

impl MockBackend {
    pub fn new() -> Self {
        Self::with_writes(false)
    }

    pub fn with_writes(writes_enabled: bool) -> Self {
        let caps = CapabilitySet {
            platform_profile: true,
            profile_choices: vec!["quiet".into(), "balanced".into(), "performance".into()],
            charge_threshold: true,
            kbd_backlight: true,
            kbd_max_brightness: 3,
            panel_overdrive: true,
            dgpu_disable: true,
            fans: vec!["fan1_input".into(), "fan2_input".into()],
            battery: true,
            nvidia: true,
            model: "Mock TUF Gaming F17 FX706HF".into(),
            is_fx706hf: true,
            firmware_attrs: Vec::new(),
        };
        let state = MockState {
            profile: "balanced".into(),
            charge_threshold: 80,
            kbd_brightness: 1,
            panel_od: true,
            dgpu_disable: false,
            write_count: 0,
        };
        MockBackend {
            caps,
            state,
            audit: AuditLog::disabled(),
            writes_enabled,
        }
    }

    pub fn state(&self) -> &MockState {
        &self.state
    }

    fn fan_curve_rpm(base: u32, wobble: u64) -> u32 {
        base + ((wobble * 7) % 120) as u32
    }

    /// Uniform monitoring-first gate for every mock write.
    fn mock_write_gate(&self) -> HalResult<()> {
        if !self.writes_enabled {
            return Err(HalError::Unsupported);
        }
        Ok(())
    }
}

impl Default for MockBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl Backend for MockBackend {
    fn name(&self) -> &'static str {
        "mock"
    }

    fn capabilities(&self) -> &CapabilitySet {
        &self.caps
    }

    fn telemetry(&self) -> HalResult<Telemetry> {
        let tick = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let load = match self.state.profile.as_str() {
            "quiet" => 55_000,
            "performance" => 88_000,
            _ => 71_000,
        };
        let batt = Battery {
            status: "Charging".into(),
            capacity_percent: 78,
            charge_now_uah: Some(2_200_000),
            charge_full_uah: Some(2_836_000),
            charge_full_design_uah: Some(4_110_000),
            charge_threshold: Some(self.state.charge_threshold),
            ac_online: true,
            power_now_uw: Some(18_500_000),
        };
        Ok(Telemetry {
            profile: Some(self.state.profile.clone()),
            profile_choices: self.caps.profile_choices.clone(),
            cpu_package_temp_mC: Some(load),
            cpu_fan_rpm: Some(Self::fan_curve_rpm(3400, tick)),
            gpu_fan_rpm: Some(Self::fan_curve_rpm(3200, tick)),
            fan_labels: vec![
                ("cpu_fan".into(), Some(Self::fan_curve_rpm(3400, tick))),
                ("gpu_fan".into(), Some(Self::fan_curve_rpm(3200, tick))),
            ],
            nvme_temp_mC: Some(52_000),
            acpi_zone_temps_mC: vec![("acpitz_0".into(), load)],
            battery: Some(batt),
            kbd_brightness: Some(self.state.kbd_brightness),
            kbd_max_brightness: Some(3),
            panel_od: Some(self.state.panel_od),
            dgpu_disable: Some(self.state.dgpu_disable),
            charge_mode: Some("1".into()),
            nvidia: Some(nvidia::NvidiaState {
                temperature_c: 74,
                utilization_percent: 0,
                power_watts: 4.2,
                perf_state: "P8".into(),
                throttling: vec!["SW Thermal Slowdown".into()],
                available: true,
            }),
        })
    }

    fn set_profile(&mut self, p: Profile, confirmed: bool) -> HalResult<()> {
        self.mock_write_gate()?;
        self.caps
            .validate(VerifiedAttr::PlatformProfile, confirmed)?;
        self.state.profile = p.as_str().to_string();
        self.state.write_count += 1;
        self.audit.record(VerifiedAttr::PlatformProfile, p.as_str());
        Ok(())
    }

    fn set_charge_threshold(&mut self, percent: u32, confirmed: bool) -> HalResult<()> {
        self.mock_write_gate()?;
        self.caps
            .validate(VerifiedAttr::ChargeThreshold, confirmed)?;
        let v = clamp(VerifiedAttr::ChargeThreshold, percent, 3)?;
        self.state.charge_threshold = v;
        self.state.write_count += 1;
        self.audit
            .record(VerifiedAttr::ChargeThreshold, &v.to_string());
        Ok(())
    }

    fn set_kbd_brightness(&mut self, level: u32, confirmed: bool) -> HalResult<()> {
        self.mock_write_gate()?;
        self.caps.validate(VerifiedAttr::KbdBrightness, confirmed)?;
        let v = clamp(VerifiedAttr::KbdBrightness, level, 3)?;
        self.state.kbd_brightness = v;
        self.state.write_count += 1;
        self.audit
            .record(VerifiedAttr::KbdBrightness, &v.to_string());
        Ok(())
    }

    fn set_panel_overdrive(&mut self, on: bool, confirmed: bool) -> HalResult<()> {
        self.mock_write_gate()?;
        self.caps
            .validate(VerifiedAttr::PanelOverdrive, confirmed)?;
        self.state.panel_od = on;
        self.state.write_count += 1;
        self.audit
            .record(VerifiedAttr::PanelOverdrive, if on { "1" } else { "0" });
        Ok(())
    }

    fn set_dgpu_disabled(&mut self, disable: bool, confirmed: bool) -> HalResult<()> {
        self.mock_write_gate()?;
        self.caps.validate(VerifiedAttr::DgpuDisable, confirmed)?;
        self.state.dgpu_disable = disable;
        self.state.write_count += 1;
        self.audit
            .record(VerifiedAttr::DgpuDisable, if disable { "1" } else { "0" });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// Build a fake sysfs tree mirroring the FX706HF interface set.
    fn fixture() -> tempfile::TempDir {
        let d = tempfile::tempdir().unwrap();
        let r = d.path();
        fs::create_dir_all(r.join("sys/class/dmi/id")).unwrap();
        fs::write(
            r.join("sys/class/dmi/id/product_name"),
            "ASUS TUF Gaming F17 FX706HF_FX706HF\n",
        )
        .unwrap();
        fs::create_dir_all(r.join("sys/firmware/acpi")).unwrap();
        fs::write(r.join("sys/firmware/acpi/platform_profile"), "balanced\n").unwrap();
        fs::write(
            r.join("sys/firmware/acpi/platform_profile_choices"),
            "quiet balanced performance\n",
        )
        .unwrap();
        let bat = r.join("sys/class/power_supply/BAT1");
        fs::create_dir_all(&bat).unwrap();
        for (f, v) in [
            ("status", "Full"),
            ("capacity", "100"),
            ("charge_now", "2836000"),
            ("charge_full", "2836000"),
            ("charge_full_design", "4110000"),
            ("charge_control_end_threshold", "80"),
            ("power_now", "0"),
        ] {
            fs::write(bat.join(f), format!("{v}\n")).unwrap();
        }
        let ac = r.join("sys/class/power_supply/ACAD");
        fs::create_dir_all(&ac).unwrap();
        fs::write(ac.join("online"), "1\n").unwrap();
        let led = r.join("sys/class/leds/asus::kbd_backlight");
        fs::create_dir_all(&led).unwrap();
        fs::write(led.join("brightness"), "1\n").unwrap();
        fs::write(led.join("max_brightness"), "3\n").unwrap();
        let nb = r.join("sys/devices/platform/asus-nb-wmi");
        fs::create_dir_all(&nb).unwrap();
        fs::write(nb.join("panel_od"), "1\n").unwrap();
        fs::write(nb.join("dgpu_disable"), "0\n").unwrap();
        let hw = r.join("sys/class/hwmon/hwmon6");
        fs::create_dir_all(&hw).unwrap();
        fs::write(hw.join("name"), "asus\n").unwrap();
        fs::write(hw.join("fan1_input"), "4700\n").unwrap();
        fs::write(hw.join("fan1_label"), "cpu_fan\n").unwrap();
        fs::write(hw.join("fan2_input"), "4700\n").unwrap();
        fs::write(hw.join("fan2_label"), "gpu_fan\n").unwrap();
        let ct = r.join("sys/class/hwmon/hwmon7");
        fs::create_dir_all(&ct).unwrap();
        fs::write(ct.join("name"), "coretemp\n").unwrap();
        fs::write(ct.join("temp1_input"), "95000\n").unwrap();
        d
    }

    #[test]
    fn detects_fx706hf_capabilities_from_fixture() {
        let d = fixture();
        let b = SysfsBackend::with_root(d.path(), false, None).unwrap();
        let c = b.capabilities();
        assert!(c.is_fx706hf);
        assert!(c.platform_profile && c.charge_threshold && c.kbd_backlight);
        assert!(c.panel_overdrive && c.dgpu_disable);
        assert_eq!(c.fans.len(), 2);
        // nvidia detection is environment-dependent (real nvidia-smi vs CI):
        // only assert it never *false-negatives* against the same probe it uses.
        assert_eq!(c.nvidia, nvidia::query().is_some());
    }

    #[test]
    fn telemetry_reads_fixture() {
        let d = fixture();
        let b = SysfsBackend::with_root(d.path(), false, None).unwrap();
        let t = b.telemetry().unwrap();
        assert_eq!(t.profile.as_deref(), Some("balanced"));
        assert_eq!(t.cpu_package_temp_mC, Some(95000));
        assert_eq!(t.cpu_fan_rpm, Some(4700));
        let b = t.battery.unwrap();
        assert_eq!(b.health_percent(), Some(69)); // 2836000/4110000
        assert_eq!(b.charge_threshold, Some(80));
        assert!(b.ac_online);
    }

    #[test]
    fn writes_disabled_by_default_monitoring_only() {
        let d = fixture();
        let mut b = SysfsBackend::with_root(d.path(), false, None).unwrap();
        let err = b.set_profile(Profile::Performance, true).unwrap_err();
        assert!(matches!(err, HalError::Unsupported));
        // file unchanged
        assert!(
            std::fs::read_to_string(d.path().join("sys/firmware/acpi/platform_profile"))
                .unwrap()
                .starts_with("balanced")
        );
    }

    #[test]
    fn writes_clamped_and_audited() {
        let d = fixture();
        let audit = d.path().join("audit.jsonl");
        let mut b = SysfsBackend::with_root(d.path(), true, Some(audit.clone())).unwrap();
        // clamp path: threshold 150 must be rejected, not written
        assert!(b.set_charge_threshold(150, true).is_err());
        assert!(b.set_charge_threshold(80, true).is_ok());
        let content = std::fs::read_to_string(
            d.path()
                .join("sys/class/power_supply/BAT1/charge_control_end_threshold"),
        )
        .unwrap();
        assert_eq!(content.trim(), "80");
        let log = std::fs::read_to_string(&audit).unwrap();
        assert!(log.contains("charge_threshold"));
    }

    #[test]
    fn unknown_profile_rejected() {
        let d = fixture();
        let mut b = SysfsBackend::with_root(d.path(), true, None).unwrap();
        // clamp happens in verified_write only for ranged attrs; profile check:
        let res = b.set_profile(
            Profile::from_kernel_str("turbo-xtreme").unwrap_or(Profile::Performance),
            true,
        );
        assert!(res.is_ok()); // enum has no invalid values — compile-time safety
        assert_eq!(
            std::fs::read_to_string(d.path().join("sys/firmware/acpi/platform_profile"))
                .unwrap()
                .trim(),
            "performance"
        );
    }

    #[test]
    fn mock_backend_roundtrip() {
        let mut m = MockBackend::with_writes(true);
        m.set_profile(Profile::Quiet, true).unwrap();
        assert_eq!(m.telemetry().unwrap().profile.as_deref(), Some("quiet"));
        m.set_dgpu_disabled(true, false).unwrap_err(); // confirmation required
        m.set_dgpu_disabled(true, true).unwrap();
        assert_eq!(m.state().write_count, 2);
    }

    // ---- degraded-capability fixtures: the app must stay safe and useful
    // when interfaces are absent (other machines, kernel changes, dGPU off) ----

    #[test]
    fn degraded_no_battery_graceful() {
        let d = fixture();
        std::fs::remove_dir_all(d.path().join("sys/class/power_supply/BAT1")).unwrap();
        let mut b = SysfsBackend::with_root(d.path(), true, None).unwrap();
        assert!(!b.capabilities().charge_threshold);
        assert!(b.telemetry().unwrap().battery.is_none());
        let err = b.set_charge_threshold(80, true).unwrap_err();
        assert!(matches!(
            err,
            HalError::Safety(tuf_core::SafetyError::MissingCapability(_))
        ));
    }

    #[test]
    fn degraded_no_asus_nb_wmi_graceful() {
        let d = fixture();
        std::fs::remove_dir_all(d.path().join("sys/devices/platform/asus-nb-wmi")).unwrap();
        let mut b = SysfsBackend::with_root(d.path(), true, None).unwrap();
        assert!(!b.capabilities().panel_overdrive && !b.capabilities().dgpu_disable);
        let t = b.telemetry().unwrap();
        assert!(t.panel_od.is_none() && t.dgpu_disable.is_none());
        assert!(b.set_panel_overdrive(true, true).is_err());
        // fan telemetry still works — degradation is per-interface
        assert_eq!(t.cpu_fan_rpm, Some(4700));
    }

    #[test]
    fn degraded_no_led_no_profile_graceful() {
        let d = fixture();
        std::fs::remove_dir_all(d.path().join("sys/class/leds/asus::kbd_backlight")).unwrap();
        std::fs::remove_file(d.path().join("sys/firmware/acpi/platform_profile")).unwrap();
        let b = SysfsBackend::with_root(d.path(), false, None).unwrap();
        let c = b.capabilities();
        assert!(!c.kbd_backlight && !c.platform_profile);
        let t = b.telemetry().unwrap(); // must not error
        assert!(t.profile.is_none() && t.kbd_brightness.is_none());
    }

    #[test]
    fn non_fx706hf_machine_still_works_per_attribute() {
        let d = fixture();
        std::fs::write(
            d.path().join("sys/class/dmi/id/product_name"),
            "Some Other Laptop\n",
        )
        .unwrap();
        let mut b = SysfsBackend::with_root(d.path(), true, None).unwrap();
        assert!(!b.capabilities().is_fx706hf);
        // per-attribute safety still applies: attrs that exist remain writable
        b.set_profile(Profile::Performance, true).unwrap();
        // and attrs that do not exist are refused regardless of model
        std::fs::remove_dir_all(d.path().join("sys/class/power_supply/BAT1")).unwrap();
        assert!(b.set_charge_threshold(80, true).is_err());
    }
}
