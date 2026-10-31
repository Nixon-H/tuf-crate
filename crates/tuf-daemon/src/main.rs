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
pub fn handle_request(backend: &mut dyn Backend, line: &str) -> serde_json::Value {
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
        other => Ok(json!({"ok": false, "error": format!("unknown op '{other}'")})),
    };
    match res {
        Ok(v) => v,
        Err(e) => json!({"ok": false, "error": e.to_string()}),
    }
}

/// Serve one connection until EOF (the GUI keeps a single connection open).
pub fn serve_connection(backend: &mut dyn Backend, stream: UnixStream) {
    let mut writer = stream.try_clone().ok();
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) | Err(_) => return,
            Ok(_) => {}
        }
        let resp = handle_request(backend, line.trim());
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
    // Local single-user machine policy: the socket accepts connections from
    // local users, but every operation is a closed-enum, clamped, audited
    // verified write. See docs/SAFETY.md §"privileged daemon".
    let _ = std::fs::set_permissions(&socket, std::fs::Permissions::from_mode(0o666));

    eprintln!(
        "tuf-crated {} listening on {} (model: {})",
        env!("CARGO_PKG_VERSION"),
        socket.display(),
        backend.capabilities().model
    );

    for stream in listener.incoming() {
        match stream {
            Ok(s) => serve_connection(&mut backend, s),
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
        d
    }

    #[test]
    fn ping_and_caps() {
        let d = fixture();
        let mut b = SysfsBackend::with_root(d.path(), true, None).unwrap();
        assert_eq!(
            handle_request(&mut b, r#"{"op":"ping"}"#)["ok"],
            json!(true)
        );
        let caps = handle_request(&mut b, r#"{"op":"caps"}"#);
        assert_eq!(caps["capabilities"]["is_fx706hf"], json!(true));
    }

    #[test]
    fn set_profile_via_daemon_protocol() {
        let d = fixture();
        let mut b = SysfsBackend::with_root(d.path(), true, None).unwrap();
        let r = handle_request(&mut b, r#"{"op":"set_profile","value":"performance"}"#);
        assert_eq!(r["ok"], json!(true));
        assert_eq!(
            fs::read_to_string(d.path().join("sys/firmware/acpi/platform_profile"))
                .unwrap()
                .trim(),
            "performance"
        );
        // clamped/unknown values rejected by the same funnel
        let r = handle_request(&mut b, r#"{"op":"set_profile","value":"turbo-xtreme"}"#);
        assert_eq!(r["ok"], json!(false));
    }

    #[test]
    fn threshold_clamp_rejected_through_protocol() {
        let d = fixture();
        let mut b = SysfsBackend::with_root(d.path(), true, None).unwrap();
        let r = handle_request(&mut b, r#"{"op":"set_charge_threshold","value":150}"#);
        assert_eq!(r["ok"], json!(false));
        let r = handle_request(&mut b, r#"{"op":"set_charge_threshold","value":60}"#);
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
        let mut b = SysfsBackend::with_root(d.path(), true, None).unwrap();
        let r = handle_request(&mut b, r#"{"op":"set_dgpu_disable","value":1}"#);
        assert_eq!(r["ok"], json!(false));
        let r = handle_request(
            &mut b,
            r#"{"op":"set_dgpu_disable","value":1,"confirmed":true}"#,
        );
        assert_eq!(r["ok"], json!(true));
    }

    #[test]
    fn unknown_op_and_bad_json() {
        let d = fixture();
        let mut b = SysfsBackend::with_root(d.path(), true, None).unwrap();
        assert_eq!(
            handle_request(&mut b, r#"{"op":"reboot_now"}"#)["ok"],
            json!(false)
        );
        assert_eq!(handle_request(&mut b, "not json")["ok"], json!(false));
    }

    #[test]
    fn socket_roundtrip_on_fixture() {
        let d = fixture();
        let sock = d.path().join("test.sock");
        let mut b = SysfsBackend::with_root(d.path(), true, None).unwrap();
        let listener = UnixListener::bind(&sock).unwrap();
        let server = std::thread::spawn(move || {
            for s in listener.incoming().flatten() {
                serve_connection(&mut b, s);
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
