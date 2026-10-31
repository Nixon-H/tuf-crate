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

/// NVML clocksThrottleReasons bit meanings (legacy bitmask form).
/// Bit 0x1 ("Gpu Idle") is informational, not a throttle condition — excluded,
/// matching the reason-name path.
fn decode_throttle_bits(mask: u64) -> Vec<String> {
    const REASONS: &[(u64, &str)] = &[
        (0x0000_0000_0000_0002, "Applications Clocks Setting"),
        (0x0000_0000_0000_0004, "SW Power Cap"),
        (0x0000_0000_0000_0008, "HW Slowdown"),
        (0x0000_0000_0000_0010, "Sync Boost"),
        (0x0000_0000_0000_0020, "SW Thermal Slowdown"),
        (0x0000_0000_0000_0040, "HW Thermal Slowdown"),
        (0x0000_0000_0000_0080, "HW Power Brake Slowdown"),
        (0x0000_0000_0000_0100, "Display Clock Setting"),
    ];
    REASONS
        .iter()
        .filter(|(bit, _)| mask & bit != 0)
        .map(|(_, name)| (*name).to_string())
        .collect()
}

fn decode_throttle_field(raw: &str) -> Vec<String> {
    let t = raw.trim();
    // Newer drivers emit reason names; older ones a hex bitmask.
    if let Some(hex) = t.strip_prefix("0x") {
        if let Ok(mask) = u64::from_str_radix(hex, 16) {
            return decode_throttle_bits(mask);
        }
    }
    t.split(" | ")
        .map(str::trim)
        .filter(|s| !s.is_empty() && *s != "N/A" && *s != "Not Active" && *s != "Gpu Idle")
        .map(str::to_string)
        .collect()
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
        throttling: decode_throttle_field(f[4]),
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
        // 0x4 = SW Power Cap
        assert_eq!(st.throttling, vec!["SW Power Cap".to_string()]);
    }

    #[test]
    fn decodes_idle_bitmask_to_no_throttle() {
        let st = parse("79, 0, 5.2, P8, 0x0000000000000001").unwrap();
        assert!(st.throttling.is_empty()); // Gpu Idle is not a throttle condition
    }
}
