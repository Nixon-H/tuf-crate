//! tuf-crate-cli — strictly READ-ONLY command-line view of the tuf-crate
//! telemetry and capability set. This binary contains no write paths at all.

use tuf_hal::{Backend, SysfsBackend};

fn main() {
    let backend = match SysfsBackend::real(false, None) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("tuf-crate-cli: cannot initialise: {e}");
            std::process::exit(1);
        }
    };
    let caps = backend.capabilities();
    let t = match backend.telemetry() {
        Ok(t) => t,
        Err(e) => {
            eprintln!("tuf-crate-cli: telemetry failed: {e}");
            std::process::exit(2);
        }
    };

    let json = std::env::args().any(|a| a == "--json");
    if json {
        let doc = serde_json::json!({
            "capabilities": caps,
            "telemetry": {
                "profile": t.profile,
                "cpu_temp_mC": t.cpu_package_temp_mC,
                "nvme_temp_mC": t.nvme_temp_mC,
                "fans": t.fan_labels,
                "battery": t.battery.as_ref().map(|b| serde_json::json!({
                    "status": b.status,
                    "capacity_percent": b.capacity_percent,
                    "health_percent": b.health_percent(),
                    "charge_threshold": b.charge_threshold,
                    "ac_online": b.ac_online,
                })),
                "panel_overdrive": t.panel_od,
                "dgpu_disable": t.dgpu_disable,
                "nvidia": t.nvidia,
            },
        });
        println!("{}", serde_json::to_string_pretty(&doc).unwrap());
        return;
    }

    println!("tuf-crate-cli — read-only status");
    println!(
        "model: {} (FX706HF-verified: {})",
        caps.model, caps.is_fx706hf
    );
    println!(
        "profile: {} (choices: {})",
        t.profile.as_deref().unwrap_or("—"),
        caps.profile_choices.join(", ")
    );
    println!(
        "cpu: {}  nvme: {}",
        fmt_temp(t.cpu_package_temp_mC),
        fmt_temp(t.nvme_temp_mC)
    );
    for (l, r) in &t.fan_labels {
        println!(
            "fan {l}: {} RPM",
            r.map(|v| v.to_string()).unwrap_or_else(|| "—".into())
        );
    }
    if let Some(b) = &t.battery {
        println!(
            "battery: {} % · health {} % · limit {:?} % · AC {}",
            b.capacity_percent,
            b.health_percent()
                .map(|h| h.to_string())
                .unwrap_or_else(|| "?".into()),
            b.charge_threshold,
            if b.ac_online { "on" } else { "off" }
        );
    }
    if let Some(g) = &t.nvidia {
        println!(
            "nvidia: {} °C · {} % · {:.1} W · {} · throttle: {:?}",
            g.temperature_c, g.utilization_percent, g.power_watts, g.perf_state, g.throttling
        );
    }
    println!(
        "kbd backlight: {:?}/{:?}",
        t.kbd_brightness, t.kbd_max_brightness
    );
    println!(
        "panel overdrive: {:?} · dgpu_disable: {:?}",
        t.panel_od, t.dgpu_disable
    );
    println!(
        "firmware-attrs: {}",
        if caps.firmware_attrs.is_empty() {
            "—".into()
        } else {
            caps.firmware_attrs.join(", ")
        }
    );
}

fn fmt_temp(mc: Option<i64>) -> String {
    mc.map(|v| format!("{:.0} °C", v as f32 / 1000.0))
        .unwrap_or_else(|| "—".into())
}
