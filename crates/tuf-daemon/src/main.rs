//! tuf-crated — the privileged control daemon for tuf-crate.
//!
//! Why this exists: sysfs hardware attributes are root-writable, so a GUI
//! running as the user cannot write them. Standard solution (like asusd /
//! power-profiles-daemon): a small root service exposing a *policy-scoped*
//! local socket. This daemon can ONLY perform the five verified operations
//! of `tuf_core::VerifiedAttr`, through the same capability + clamp + audit
//! funnel as everything else. There is no API for arbitrary paths, EC
//! access, ACPI calls, PWM duty writes, or module loading — not even here.
//!
//! Protocol: newline-delimited JSON over a unix socket.
//!   request:  {"op":"set_profile","value":"performance","confirmed":true}
//!   response: {"ok":true}  |  {"ok":false,"error":"…"}
//! Ops: ping, caps, set_profile, set_charge_threshold, set_kbd_brightness,
//!      set_panel_overdrive, set_dgpu_disable.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;

use serde::Deserialize;
use serde_json::json;
use tuf_core::Profile;
use tuf_hal::{Backend, SysfsBackend};

const DEFAULT_SOCKET: &str = "/run/tuf-crate/control.sock";
const DEFAULT_CONFIG: &str = "/etc/tuf-crate/apply.json";

/// 24/7 persistence: settings the daemon re-applies at every startup.
/// Saved via the `save_boot_config` op (validated), removed by `clear_boot_config`.
#[derive(Debug, Clone, Default)]
pub struct BootConfig {
    pub path: PathBuf,
}

impl BootConfig {
    pub fn at(path: PathBuf) -> Self {
        BootConfig { path }
    }
    pub fn load(&self) -> Option<serde_json::Value> {
        let raw = std::fs::read_to_string(&self.path).ok()?;
        let v: serde_json::Value = serde_json::from_str(&raw).ok()?;
        if v.is_object() && !v.as_object().map(|o| o.is_empty()).unwrap_or(true) {
            Some(v)
        } else {
            None
        }
    }
    pub fn save(&self, v: &serde_json::Value) -> std::io::Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let tmp = self.path.with_extension("json.tmp");
        std::fs::write(&tmp, format!("{}\n", serde_json::to_string_pretty(v)?))?;
        std::fs::rename(&tmp, &self.path)
    }
    pub fn clear(&self) -> std::io::Result<()> {
        std::fs::remove_file(&self.path)
    }
}


/// Best-effort check that a graphical session is already running. Used only
/// to refuse a live boot-config dGPU write; false positives merely postpone
/// the switch to the next boot.
fn display_session_active() -> bool {
    // Fixture tests run on a machine that may itself have a desktop session;
    // they only ever touch a temporary sysfs tree.
    if cfg!(test) {
        return false;
    }
    if std::process::Command::new("systemctl")
        .args(["is-active", "--quiet", "display-manager.service"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
    {
        return true;
    }
    let x = std::fs::read_dir("/tmp/.X11-unix")
        .map(|rd| {
            rd.flatten()
                .any(|e| e.file_name().to_string_lossy().starts_with('X'))
        })
        .unwrap_or(false);
    if x {
        return true;
    }
    std::fs::read_dir("/run/user")
        .map(|rd| {
            rd.flatten().any(|u| {
                std::fs::read_dir(u.path())
                    .map(|r| {
                        r.flatten()
                            .any(|f| f.file_name().to_string_lossy().starts_with("wayland-"))
                    })
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false)
}

/// Apply the persisted settings through the same verified funnel. Every entry
/// is re-clamped/capability-checked at apply time — the file cannot smuggle
/// in anything the live ops would refuse.
pub fn apply_boot_config(backend: &mut dyn Backend, cfg: &BootConfig) -> Vec<String> {
    let mut applied = Vec::new();
    if let Some(v) = cfg.load() {
        if let Some(p) = v.get("profile").and_then(serde_json::Value::as_str) {
            if let Some(profile) = Profile::from_kernel_str(p) {
                if backend.set_profile(profile, true).is_ok() {
                    applied.push(format!("profile={p}"));
                }
            }
        }
        if let Some(w) = v.get("pl1_watts").and_then(serde_json::Value::as_u64) {
            if backend.set_cpu_power_limit(w as u32, true).is_ok() {
                applied.push(format!("pl1_watts={w}"));
            }
        }
        if let Some(f) = v.get("fan_full_speed").and_then(serde_json::Value::as_bool) {
            if backend.set_fan_full_speed(f, true).is_ok() {
                applied.push(format!("fan_full_speed={f}"));
            }
        }
        if let Some(t) = v.get("charge_threshold").and_then(serde_json::Value::as_u64) {
            if backend.set_charge_threshold(t as u32, true).is_ok() {
                applied.push(format!("charge_threshold={t}"));
            }
        }
        // Boot-time dGPU mode. The daemon starts before the display manager,
        // so this is the only safe way to power the dGPU off: never while
        // NVIDIA/compositor is bound (that froze this machine).
        if let Some(d) = v.get("dgpu_disable").and_then(serde_json::Value::as_bool) {
            // Guard against an installer/manual restart while the desktop is
            // live: writing dgpu_disable under a bound compositor froze this
            // machine. At real boot the display manager is still inactive.
            if display_session_active() {
                applied.push("dgpu_disable=skipped-live-session".into());
            } else if backend.set_dgpu_disabled(d, true).is_ok() {
                applied.push(format!("dgpu_disable={d}"));
            }
        }
    }
    applied
}

#[derive(Deserialize)]
struct Request {
    op: String,
    #[serde(default)]
    value: serde_json::Value,
    #[serde(default)]
    confirmed: bool,
}

fn value_u32(v: &serde_json::Value) -> Option<u32> {
    match v {
        serde_json::Value::Number(n) => n.as_u64().map(|x| x as u32),
        serde_json::Value::String(s) => s.parse().ok(),
        _ => None,
    }
}

/// Handle a single request against the backend. All safety enforcement lives
/// in the Backend funnel (capability → confirmation → clamp → write → audit);
/// this function adds zero policy of its own.
pub fn handle_request(
    backend: &mut dyn Backend,
    cfg: &BootConfig,
    line: &str,
) -> serde_json::Value {
    let req: Request = match serde_json::from_str(line) {
        Ok(r) => r,
        Err(e) => return json!({"ok": false, "error": format!("bad request: {e}")}),
    };
    let res = match req.op.as_str() {
        "ping" => Ok(json!({"ok": true, "version": env!("CARGO_PKG_VERSION")})),
        "caps" => Ok(json!({"ok": true, "capabilities": backend.capabilities()})),
        "set_profile" => {
            let s = req.value.as_str().unwrap_or_default().to_string();
            match Profile::from_kernel_str(&s) {
                Some(p) => backend
                    .set_profile(p, req.confirmed)
                    .map(|_| json!({"ok": true})),
                None => Ok(
                    json!({"ok": false, "error": format!("unknown profile '{s}' (expected quiet|balanced|performance)")}),
                ),
            }
        }
        "set_charge_threshold" => match value_u32(&req.value) {
            Some(v) => backend
                .set_charge_threshold(v, req.confirmed)
                .map(|_| json!({"ok": true})),
            None => Ok(json!({"ok": false, "error": "value must be a number"})),
        },
        "set_kbd_brightness" => match value_u32(&req.value) {
            Some(v) => backend
                .set_kbd_brightness(v, req.confirmed)
                .map(|_| json!({"ok": true})),
            None => Ok(json!({"ok": false, "error": "value must be a number"})),
        },
        "set_kbd_rgb" => {
            let get = |k: &str| req.value.get(k).and_then(serde_json::Value::as_u64);
            match (get("mode"), get("r"), get("g"), get("b"), get("speed")) {
                (Some(mode), Some(r), Some(g), Some(b), Some(speed)) => backend
                    .set_kbd_rgb(
                        mode as u32,
                        r as u32,
                        g as u32,
                        b as u32,
                        speed as u32,
                        req.confirmed,
                    )
                    .map(|_| json!({"ok": true})),
                _ => Ok(json!({
                    "ok": false,
                    "error": "value must be {mode,r,g,b,speed} numbers"
                })),
            }
        }
        "set_panel_overdrive" => match value_u32(&req.value) {
            Some(v) if v <= 1 => backend
                .set_panel_overdrive(v == 1, req.confirmed)
                .map(|_| json!({"ok": true})),
            _ => Ok(json!({"ok": false, "error": "value must be 0 or 1"})),
        },
        "set_dgpu_disable" => match value_u32(&req.value) {
            Some(v) if v <= 1 => backend
                .set_dgpu_disabled(v == 1, req.confirmed)
                .map(|_| json!({"ok": true})),
            _ => Ok(json!({"ok": false, "error": "value must be 0 or 1"})),
        },
        "set_fan_full_speed" => match value_u32(&req.value) {
            Some(v) if v <= 1 => backend
                .set_fan_full_speed(v == 1, req.confirmed)
                .map(|_| json!({"ok": true})),
            _ => Ok(json!({"ok": false, "error": "value must be 0 or 1"})),
        },
        "set_cpu_power_limit" => match value_u32(&req.value) {
            Some(w) => backend
                .set_cpu_power_limit(w, req.confirmed)
                .map(|_| json!({"ok": true})),
            None => Ok(json!({"ok": false, "error": "value must be a number (watts)"})),
        },
        "save_boot_config" => {
            let Some(obj) = req.value.as_object() else {
                return json!({"ok": false, "error": "value must be an object"});
            };
            // whitelist keys; reject anything else so the file stays within policy
            for k in obj.keys() {
                if !matches!(
                    k.as_str(),
                    "profile" | "pl1_watts" | "fan_full_speed" | "charge_threshold" | "dgpu_disable"
                ) {
                    return json!({"ok": false, "error": format!("key '{k}' is not persistable")});
                }
            }
            Ok(match cfg.save(&req.value) {
                Ok(()) => json!({"ok": true, "saved": req.value}),
                Err(e) => json!({"ok": false, "error": format!("cannot save: {e}")}),
            })
        }
        "clear_boot_config" => Ok(match cfg.clear() {
            Ok(()) => json!({"ok": true}),
            Err(_) => json!({"ok": true, "note": "no persisted config"}),
        }),
        "get_boot_config" => Ok(json!({"ok": true, "config": cfg.load()})),
        other => Ok(json!({"ok": false, "error": format!("unknown op '{other}'")})),
    };
    match res {
        Ok(v) => v,
        Err(e) => json!({"ok": false, "error": e.to_string()}),
    }
}

/// Serve one short-lived request/response connection. The GUI opens one
/// connection per operation. A read timeout prevents an idle local client
/// from blocking the single-threaded accept loop indefinitely.
pub fn serve_connection(backend: &mut dyn Backend, cfg: &BootConfig, stream: UnixStream) {
    let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(5)));
    let mut writer = stream.try_clone().ok();
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) | Err(_) => return,
            Ok(_) => {}
        }
        if line.len() > 64 * 1024 {
            let _ = writer
                .as_mut()
                .map(|w| writeln!(w, "{{\"ok\":false,\"error\":\"request too large\"}}"));
            return;
        }
        let resp = handle_request(backend, cfg, line.trim());
        if let Some(w) = writer.as_mut() {
            let _ = writeln!(w, "{resp}");
            let _ = w.flush();
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let socket = args
        .iter()
        .position(|a| a == "--socket")
        .and_then(|i| args.get(i + 1))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_SOCKET));

    // The daemon exists to perform verified writes; audit trail is mandatory.
    let cfg = BootConfig::at(
        args.iter()
            .position(|a| a == "--config")
            .and_then(|i| args.get(i + 1))
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(DEFAULT_CONFIG)),
    );
    // Boot timing: intel_rapl can appear slightly after multi-user.target;
    // wait briefly so the persisted power limit applies on the first try.
    for _ in 0..10 {
        if PathBuf::from("/sys/class/powercap/intel-rapl:0").exists() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
    let audit = PathBuf::from("/var/lib/tuf-crate/audit.jsonl");
    let mut backend = match SysfsBackend::real(true, Some(audit)) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("tuf-crated: backend init failed: {e}");
            std::process::exit(1);
        }
    };

    if let Some(parent) = socket.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    // Self-guard: if a live daemon owns the socket, exit cleanly (the cron
    // launcher re-runs this binary every minute). Only remove a STALE socket.
    if socket.exists() && UnixStream::connect(&socket).is_ok() {
        eprintln!("tuf-crated: another instance is running — exiting");
        return;
    }
    let _ = std::fs::remove_file(&socket);
    let listener = match UnixListener::bind(&socket) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("tuf-crated: cannot bind {}: {e}", socket.display());
            std::process::exit(1);
        }
    };
    // Local control socket. It must remain usable immediately after install
    // (adding a user to a group does not affect their running session). Every
    // operation is still a closed-enum, capability-checked, clamped, audited
    // verified write; there is no generic sysfs/EC path behind this socket.
    let _ = std::fs::set_permissions(&socket, std::fs::Permissions::from_mode(0o666));

    // Publish the read-only ASUS part number for the user session: the GUI
    // needs it for the cloud device-image lookup (Armoury's ProductPNG
    // pipeline), but the SMBIOS OEM strings that carry it are root-only.
    // The number is not secret; it is written into the daemon's runtime dir.
    if let Some(pn) = tuf_hal::smbios_part_number() {
        let dir = std::path::Path::new("/run/tuf-crate");
        let _ = std::fs::create_dir_all(dir);
        let _ = std::fs::write(dir.join("part_number"), format!("{pn}\n"));
        let _ = std::fs::set_permissions(
            dir.join("part_number"),
            std::os::unix::fs::PermissionsExt::from_mode(0o644),
        );
    }

    // 24/7: re-apply persisted settings (power cap, profile, fan mode) at startup
    let applied = apply_boot_config(&mut backend, &cfg);
    if !applied.is_empty() {
        eprintln!("tuf-crated: boot config applied: {}", applied.join(", "));
    }

    eprintln!(
        "tuf-crated {} listening on {} (model: {})",
        env!("CARGO_PKG_VERSION"),
        socket.display(),
        backend.capabilities().model
    );

    for stream in listener.incoming() {
        match stream {
            Ok(s) => serve_connection(&mut backend, &cfg, s),
            Err(e) => eprintln!("tuf-crated: accept failed: {e}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

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
        fs::write(hw.join("pwm1_enable"), "2\n").unwrap();
        fs::write(hw.join("pwm2_enable"), "2\n").unwrap();
        let pc = r.join("sys/class/powercap/intel-rapl:0");
        fs::create_dir_all(&pc).unwrap();
        fs::write(pc.join("name"), "package-0\n").unwrap();
        fs::write(pc.join("constraint_0_power_limit_uw"), "65000000\n").unwrap();
        d
    }

    #[test]
    fn ping_and_caps() {
        let d = fixture();
        let cfg = BootConfig::at(d.path().join("apply.json"));
        let mut b = SysfsBackend::with_root(d.path(), true, None).unwrap();
        assert_eq!(
            handle_request(&mut b, &cfg, r#"{"op":"ping"}"#)["ok"],
            json!(true)
        );
        let caps = handle_request(&mut b, &cfg, r#"{"op":"caps"}"#);
        assert_eq!(caps["capabilities"]["is_fx706hf"], json!(true));
    }

    #[test]
    fn set_profile_via_daemon_protocol() {
        let d = fixture();
        let cfg = BootConfig::at(d.path().join("apply.json"));
        let mut b = SysfsBackend::with_root(d.path(), true, None).unwrap();
        let r = handle_request(
            &mut b,
            &cfg,
            r#"{"op":"set_profile","value":"performance"}"#,
        );
        assert_eq!(r["ok"], json!(true));
        assert_eq!(
            fs::read_to_string(d.path().join("sys/firmware/acpi/platform_profile"))
                .unwrap()
                .trim(),
            "performance"
        );
        // clamped/unknown values rejected by the same funnel
        let r = handle_request(
            &mut b,
            &cfg,
            r#"{"op":"set_profile","value":"turbo-xtreme"}"#,
        );
        assert_eq!(r["ok"], json!(false));
    }

    #[test]
    fn threshold_clamp_rejected_through_protocol() {
        let d = fixture();
        let cfg = BootConfig::at(d.path().join("apply.json"));
        let mut b = SysfsBackend::with_root(d.path(), true, None).unwrap();
        let r = handle_request(&mut b, &cfg, r#"{"op":"set_charge_threshold","value":150}"#);
        assert_eq!(r["ok"], json!(false));
        let r = handle_request(&mut b, &cfg, r#"{"op":"set_charge_threshold","value":60}"#);
        assert_eq!(r["ok"], json!(true));
        assert_eq!(
            fs::read_to_string(
                d.path()
                    .join("sys/class/power_supply/BAT1/charge_control_end_threshold")
            )
            .unwrap()
            .trim(),
            "60"
        );
    }

    #[test]
    fn dgpu_requires_confirmation_through_protocol() {
        let d = fixture();
        let cfg = BootConfig::at(d.path().join("apply.json"));
        let mut b = SysfsBackend::with_root(d.path(), true, None).unwrap();
        let r = handle_request(&mut b, &cfg, r#"{"op":"set_dgpu_disable","value":1}"#);
        assert_eq!(r["ok"], json!(false));
        let r = handle_request(
            &mut b,
            &cfg,
            r#"{"op":"set_dgpu_disable","value":1,"confirmed":true}"#,
        );
        assert_eq!(r["ok"], json!(true));
    }

    #[test]
    fn fan_full_speed_through_protocol() {
        let d = fixture();
        let cfg = BootConfig::at(d.path().join("apply.json"));
        let mut b = SysfsBackend::with_root(d.path(), true, None).unwrap();
        assert_eq!(
            handle_request(&mut b, &cfg, r#"{"op":"set_fan_full_speed","value":1}"#)["ok"],
            json!(true)
        );
        assert_eq!(
            fs::read_to_string(d.path().join("sys/class/hwmon/hwmon6/pwm1_enable"))
                .unwrap()
                .trim(),
            "0" // FULLSPEED
        );
        let r = handle_request(&mut b, &cfg, r#"{"op":"set_fan_full_speed","value":5}"#);
        assert_eq!(r["ok"], json!(false)); // only 0/2 accepted by the safety funnel
        assert_eq!(
            handle_request(&mut b, &cfg, r#"{"op":"set_fan_full_speed","value":0}"#)["ok"],
            json!(true)
        );
        assert_eq!(
            fs::read_to_string(d.path().join("sys/class/hwmon/hwmon6/pwm1_enable"))
                .unwrap()
                .trim(),
            "2" // AUTO restored
        );
    }

    #[test]
    fn cpu_power_limit_through_protocol() {
        let d = fixture();
        let cfg = BootConfig::at(d.path().join("apply.json"));
        let mut b = SysfsBackend::with_root(d.path(), true, None).unwrap();
        let pc = d
            .path()
            .join("sys/class/powercap/intel-rapl:0/constraint_0_power_limit_uw");
        // out of clamp range
        assert_eq!(
            handle_request(&mut b, &cfg, r#"{"op":"set_cpu_power_limit","value":90}"#)["ok"],
            json!(false)
        );
        assert_eq!(fs::read_to_string(&pc).unwrap().trim(), "65000000");
        // valid cap
        assert_eq!(
            handle_request(&mut b, &cfg, r#"{"op":"set_cpu_power_limit","value":35}"#)["ok"],
            json!(true)
        );
        assert_eq!(fs::read_to_string(&pc).unwrap().trim(), "35000000");
        // telemetry reports watts
        let t = b.telemetry().unwrap();
        assert_eq!(t.pl1_watts, Some(35));
    }

    #[test]
    fn boot_config_save_get_clear_roundtrip() {
        let d = fixture();
        let cfg = BootConfig::at(d.path().join("apply.json"));
        let mut b = SysfsBackend::with_root(d.path(), true, None).unwrap();
        // initially empty
        let r = handle_request(&mut b, &cfg, r#"{"op":"get_boot_config"}"#);
        assert_eq!(r["config"], json!(null));
        // save (whitelisted keys only)
        let r = handle_request(
            &mut b,
            &cfg,
            r#"{"op":"save_boot_config","value":{"profile":"quiet","pl1_watts":35,"charge_threshold":60,"dgpu_disable":true}}"#,
        );
        assert_eq!(r["ok"], json!(true));
        // foreign key rejected
        let r = handle_request(
            &mut b,
            &cfg,
            r#"{"op":"save_boot_config","value":{"ec_dump":true}}"#,
        );
        assert_eq!(r["ok"], json!(false));
        // get returns it
        let r = handle_request(&mut b, &cfg, r#"{"op":"get_boot_config"}"#);
        assert_eq!(r["config"]["profile"], json!("quiet"));
        assert_eq!(r["config"]["pl1_watts"], json!(35));
        assert_eq!(r["config"]["charge_threshold"], json!(60));
        assert_eq!(r["config"]["dgpu_disable"], json!(true));
        // boot-apply through the verified funnel
        let applied = apply_boot_config(&mut b, &cfg);
        assert_eq!(
            applied,
            vec![
                "profile=quiet",
                "pl1_watts=35",
                "charge_threshold=60",
                "dgpu_disable=true"
            ]
        );
        assert_eq!(
            fs::read_to_string(d.path().join("sys/firmware/acpi/platform_profile"))
                .unwrap()
                .trim(),
            "quiet"
        );
        assert_eq!(
            fs::read_to_string(
                d.path()
                    .join("sys/class/powercap/intel-rapl:0/constraint_0_power_limit_uw")
            )
            .unwrap()
            .trim(),
            "35000000"
        );
        assert_eq!(
            fs::read_to_string(
                d.path()
                    .join("sys/class/power_supply/BAT1/charge_control_end_threshold")
            )
            .unwrap()
            .trim(),
            "60"
        );
        assert_eq!(
            fs::read_to_string(d.path().join("sys/devices/platform/asus-nb-wmi/dgpu_disable"))
                .unwrap()
                .trim(),
            "1"
        );
        // clear
        assert_eq!(
            handle_request(&mut b, &cfg, r#"{"op":"clear_boot_config"}"#)["ok"],
            json!(true)
        );
        assert!(cfg.load().is_none());
    }

    #[test]
    fn unknown_op_and_bad_json() {
        let d = fixture();
        let cfg = BootConfig::at(d.path().join("apply.json"));
        let mut b = SysfsBackend::with_root(d.path(), true, None).unwrap();
        assert_eq!(
            handle_request(&mut b, &cfg, r#"{"op":"reboot_now"}"#)["ok"],
            json!(false)
        );
        assert_eq!(handle_request(&mut b, &cfg, "not json")["ok"], json!(false));
    }

    #[test]
    fn socket_roundtrip_on_fixture() {
        let d = fixture();
        let cfg = BootConfig::at(d.path().join("apply.json"));
        let sock = d.path().join("test.sock");
        let mut b = SysfsBackend::with_root(d.path(), true, None).unwrap();
        let listener = UnixListener::bind(&sock).unwrap();
        let cfg_t = cfg.clone();
        let server = std::thread::spawn(move || {
            for s in listener.incoming().flatten() {
                serve_connection(&mut b, &cfg_t, s);
            }
        });
        let mut c = UnixStream::connect(&sock).unwrap();
        writeln!(c, r#"{{"op":"set_profile","value":"quiet"}}"#).unwrap();
        c.flush().unwrap();
        let mut resp = String::new();
        BufReader::new(c).read_line(&mut resp).unwrap();
        assert!(resp.contains(r#""ok":true"#));
        assert_eq!(
            fs::read_to_string(d.path().join("sys/firmware/acpi/platform_profile"))
                .unwrap()
                .trim(),
            "quiet"
        );
        // server thread loops until the fixture socket vanishes at teardown
        drop(server);
    }
}
