//! Read-only NVIDIA telemetry via nvidia-smi. No driver parameter writes —
//! power/clock changes are forbidden by docs/SAFETY.md (nvidia-powerd owns them).

use serde::Serialize;
use std::process::Command;

#[derive(Debug, Clone, Default, Serialize)]
pub struct NvidiaState {
    pub temperature_c: i32,
    pub utilization_percent: u32,
    pub power_watts: f32,
    pub perf_state: String,
    pub throttling: Vec<String>,
    pub available: bool,
}

/// Query nvidia-smi. Returns None when the GPU/driver is unavailable
/// (e.g. dGPU powered down in offload mode) — that is normal, not an error.
pub fn query() -> Option<NvidiaState> {
    let out = Command::new("nvidia-smi")
        .args([
            "--query-gpu=temperature.gpu,utilization.gpu,power.draw,pstate,clocks_event_reasons.active",
            "--format=csv,noheader,nounits",
        ])
        .output()
        .ok()?;
    if !out.status.success() {
        // Older drivers use clocks_throttle_reasons.active — retry with legacy field.
        let out = Command::new("nvidia-smi")
            .args([
                "--query-gpu=temperature.gpu,utilization.gpu,power.draw,pstate,clocks_throttle_reasons.active",
                "--format=csv,noheader,nounits",
            ])
            .output()
            .ok()?;
        if !out.status.success() {
            return None;
        }
        return parse(&String::from_utf8_lossy(&out.stdout));
    }
    parse(&String::from_utf8_lossy(&out.stdout))
}

fn parse(s: &str) -> Option<NvidiaState> {
    let line = s.lines().next()?;
    let f: Vec<&str> = line.split(", ").map(str::trim).collect();
    if f.len() < 5 {
        return None;
    }
    let st = NvidiaState {
        temperature_c: f[0].parse().unwrap_or(0),
        utilization_percent: f[1].parse().unwrap_or(0),
        power_watts: f[2].parse().unwrap_or(0.0),
        perf_state: f[3]
            .trim_start_matches('[')
            .trim_end_matches(']')
            .to_string(),
        throttling: f[4]
            .split(" | ")
            .map(|t| t.trim().trim_start_matches("0x").to_string())
            .filter(|t| !t.is_empty() && t != "N/A" && t != "Not Active" && t != "Gpu Idle")
            .collect(),
        available: true,
    };
    Some(st)
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn parses_nvidia_smi_line() {
        let st = parse("77, 0, 4.28, P8, 0x0000000000000004").unwrap();
        assert_eq!(st.temperature_c, 77);
        assert_eq!(st.utilization_percent, 0);
        assert!((st.power_watts - 4.28).abs() < 0.01);
        assert_eq!(st.perf_state, "P8");
        assert_eq!(st.throttling.len(), 1); // the active-reason bitmask line
    }
}
