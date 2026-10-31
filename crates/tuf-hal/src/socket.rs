//! SocketBackend — talks to the privileged `tuf-crated` daemon for writes
//! while keeping telemetry on direct (read-only) sysfs. This is what makes
//! hardware controls work for a normal user session, exactly like the
//! privileged-daemon pattern used by asusd / power-profiles-daemon.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;

use serde_json::json;

use crate::{Backend, HalError, HalResult, SysfsBackend, Telemetry};
use tuf_core::{CapabilitySet, Profile};

pub const DEFAULT_SOCKET: &str = "/run/tuf-crate/control.sock";

pub struct SocketBackend {
    sys: SysfsBackend,
    path: PathBuf,
}

impl SocketBackend {
    /// Build from an existing (monitoring-only) sysfs backend for reads.
    pub fn new(sys: SysfsBackend, path: PathBuf) -> Self {
        SocketBackend { sys, path }
    }

    pub fn socket_path(&self) -> &PathBuf {
        &self.path
    }

    /// One request/response exchange with the daemon.
    fn request(&self, op: &str, value: serde_json::Value, confirmed: bool) -> HalResult<()> {
        let mut stream = UnixStream::connect(&self.path)
            .map_err(|e| HalError::Daemon(format!("control daemon unreachable: {e}")))?;
        let req = json!({"op": op, "value": value, "confirmed": confirmed});
        writeln!(stream, "{req}")
            .map_err(|e| HalError::Daemon(format!("daemon write failed: {e}")))?;
        let mut line = String::new();
        BufReader::new(stream)
            .read_line(&mut line)
            .map_err(|e| HalError::Daemon(format!("daemon read failed: {e}")))?;
        let resp: serde_json::Value = serde_json::from_str(line.trim())
            .map_err(|e| HalError::Daemon(format!("daemon sent bad response: {e}")))?;
        match resp.get("ok").and_then(serde_json::Value::as_bool) {
            Some(true) => Ok(()),
            _ => {
                let err = resp
                    .get("error")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("unknown daemon error")
                    .to_string();
                Err(HalError::Daemon(err))
            }
        }
    }
}

impl Backend for SocketBackend {
    fn name(&self) -> &'static str {
        "daemon"
    }

    fn capabilities(&self) -> &CapabilitySet {
        self.sys.capabilities()
    }

    fn telemetry(&self) -> HalResult<Telemetry> {
        self.sys.telemetry()
    }

    fn set_profile(&mut self, p: Profile, confirmed: bool) -> HalResult<()> {
        self.request("set_profile", json!(p.as_str()), confirmed)
    }

    fn set_charge_threshold(&mut self, percent: u32, confirmed: bool) -> HalResult<()> {
        self.request("set_charge_threshold", json!(percent), confirmed)
    }

    fn set_kbd_brightness(&mut self, level: u32, confirmed: bool) -> HalResult<()> {
        self.request("set_kbd_brightness", json!(level), confirmed)
    }

    fn set_panel_overdrive(&mut self, on: bool, confirmed: bool) -> HalResult<()> {
        self.request(
            "set_panel_overdrive",
            json!(if on { 1 } else { 0 }),
            confirmed,
        )
    }

    fn set_dgpu_disabled(&mut self, disable: bool, confirmed: bool) -> HalResult<()> {
        self.request(
            "set_dgpu_disable",
            json!(if disable { 1 } else { 0 }),
            confirmed,
        )
    }
}
