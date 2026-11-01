# Feature-by-feature comparison: Armoury Crate (Windows) vs tuf-crate (Linux)

| Feature | Armoury Crate | tuf-crate | Channel used by tuf-crate |
|---|---|---|---|
| Silent mode | ✅ | ✅ | platform_profile=quiet |
| Balanced mode | ✅ | ✅ | platform_profile=balanced |
| Performance / Turbo | ✅ | ✅ | platform_profile=performance |
| Manual fan curves | ✅ | ⛔ (firmware no-ops WMI curves; PWM unsafe) | — |
| Fan RPM monitoring | ✅ | ✅ | asus hwmon fan*_input |
| CPU/GPU temp monitoring | ✅ | ✅ | coretemp, nvidia-smi, acpitz, nvme |
| Throttling alerts | ✅ | ✅ | nvidia-smi throttle reasons + temp thresholds |
| Keyboard brightness | ✅ | ✅ | asus::kbd_backlight LED class |
| RGB static colour | ✅ | ✅ (verified WMNB DEVS 0x00100056) | upstream `kbd_rgb_mode` |
| RGB dynamic effects | ✅ | ✅ 11 modes + speed (all visually confirmed) | `kbd_rgb_mode` + `kbd_rgb_state` |
| Music effect | ✅ (Windows engine) | ✅ (Linux-native streaming) | `pw-record` → `set_kbd_rgb` frames |
| Battery charge limit | ✅ (MyASUS) | ✅ (1 % steps, 20–100) | charge_control_end_threshold |
| Battery charge modes | ✅ | ✅ display + kernel attr | firmware-attributes charge_mode |
| Battery health | ✅ | ✅ | power_supply charge_full(_design) |
| Power draw | ✅ | ✅ | power_supply power_now + nvidia-smi |
| dGPU on/off (no MUX) | ✅ | ✅ (double-confirmed) | asus-nb-wmi dgpu_disable |
| dGPU offload app launch | ✅ | ✅ | prime-run |
| GPU telemetry | ✅ | ✅ | nvidia-smi (read-only) |
| Panel overdrive | ✅ | ✅ | panel_od |
| Refresh-rate switch | ✅ | ➖ use desktop display settings | — |
| Power-limit (PPT) editing | ✅ | ⛔ (AMD-only WMI IDs; rejected here) | — |
| Aura Sync / device ecosystem | ✅ | ⛔ | — |
| Firmware updates | ✅ | ⛔ (fwupd/ASUS own this) | — |

Verdict: **every Armoury Crate capability that this hardware actually exposes over kernel-sanctioned interfaces is reproduced.** The gaps are hardware/firmware limitations documented with evidence (UNSUPPORTED.md), not implementation shortcuts.
