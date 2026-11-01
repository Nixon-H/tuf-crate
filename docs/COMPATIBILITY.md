# FX706HF Compatibility Matrix — Armoury Crate feature → Linux reality
Legend: ✅ native · 🟨 kernel-interface (frontend needed) · 🟧 documented-iface, new backend · 🟥 needs further RE · ⛔ unsupported/unsafe · M=monitoring-only

## Performance & operating modes
| Armoury Crate feature | Interface on FX706HF | Class | Notes |
|---|---|---|---|
| Silent / Balanced / Performance (Turbo) modes | `platform_profile` (quiet/balanced/performance) | ✅🟨 | Turbo maps to `performance`; verified live |
| CPU power management | intel_pstate + TLP (external, not fought) | ✅ | governor/EPP read-only in app (TLP owns writes) |
| CPU sustained power cap (PL1) | intel-rapl powercap sysfs | 🟨 | **user-authorized** clamped 25–65 W slider, audited, reversible (SAFETY.md §1c) |
| GPU performance mgmt | nvidia-smi query (clocks/power read-only) | ✅M | power-limit writes forbidden (SAFETY) |
| Hybrid / dGPU offload | `prime-run` + `dgpu_disable` attr | 🟨 | `dgpu_disable` write gated behind confirmation |
| Operating-mode switching | platform_profile write | 🟨 | core feature |

## Fans & thermals
| Feature | Interface | Class | Notes |
|---|---|---|---|
| CPU/GPU temp monitoring | hwmon coretemp + nvidia-smi + acpitz | ✅M | |
| Fan RPM monitoring | `hwmon6 fan1/fan2_input` (cpu_fan/gpu_fan) | ✅M | verified: RPM present |
| Thermal throttling detection | nvidia-smi slowdown flags, cpu MHz vs max, acpitz | ✅M | |
| Fan control / curves | WMI 0x00110024-32 return no-op; `pwm*_enable` dangerous | ⛔ | firmware acks but does not implement (RESEARCH §5) |
| Performance-vs-temperature | platform_profile × temps correlation view | 🟨M | GUI charting |

## Keyboard & RGB
| Feature | Interface | Class | Notes |
|---|---|---|---|
| Backlight brightness 0–3 | `asus::kbd_backlight` | 🟨 | write path verified by kernel driver |
| Static RGB colour | upstream asus-wmi `kbd_rgb_mode` (WMNB DEVS 0x00100056) | ✅ | verified live: firmware acks, GUI/daemon wired |
| Dynamic RGB effects | same interface, 11 Aura modes + speed 33/67/100 % | ✅ | all 11 modes + both speed extremes visually confirmed |
| Music (audio-reactive) | Linux-native: `pw-record` sink monitor → Static frames | ✅ | level meter + frame stream verified with a test tone |
| Lighting profiles / hotkeys | hotkey events via asus-armoury | ⛔→🟨 later | brightness hotkey Fn+F7 handled by kernel |

## Battery
| Feature | Interface | Class | Notes |
|---|---|---|---|
| Charge threshold 1–100 % | `charge_control_end_threshold` | 🟨 | verified live (80 %) |
| Charge modes (standard/custom) | `firmware-attributes/charge_mode` | 🟨 | read+write via kernel attr |
| Health monitoring | charge_full vs full_design, cycle count | ✅M | 69 % health documented |
| Power draw monitoring | sysfs current/power + RAPL | ✅M | |
| AC/battery profile switching | udev-observable; TLP owns policy | ✅M | app displays, does not fight TLP |

## GPU & display
| Feature | Interface | Class | Notes |
|---|---|---|---|
| dGPU status/monitoring | nvidia-smi | ✅M | |
| dGPU offload launcher | `prime-run` wrapper | 🟨 | |
| dGPU disable | asus-nb-wmi `dgpu_disable` / firmware-attrs | 🟨 ⚠ | confirmation-gated, reversible |
| Refresh rate | xrandr 60/144 Hz | 🟨 | |
| Panel overdrive | `panel_od` / firmware-attrs | 🟨 | verified on |
| Dynamic Boost / therm target | WMI 0x001200C0/C2, nvidia-powerd already running | ⛔ | kernel/nvidia-powerd owns it |

## Counts
**✅ verified + implemented:** profiles, charge threshold, charge_mode, kbd brightness, **static keyboard RGB**, panel_od, dgpu_disable, fan FULLSPEED/AUTO, CPU power cap, refresh rate guidance, prime-run launcher, + monitoring set (fans, temps, battery, GPU, throttling, AC state).
**⛔ unsupported/unsafe (5):** Armoury-side rendered effects (Smart/Adaptive/AI), fan curves, manual fan PWM, WMI PPT, Dynamic Boost control.
