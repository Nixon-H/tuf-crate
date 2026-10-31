# tuf-crate — Armoury-Crate-equivalent control center for ASUS TUF FX706HF (Linux)

A native Linux application recreating the **hardware-management capabilities** of ASUS
Armoury Crate for the TUF Gaming F17 FX706HF, built exclusively on kernel-sanctioned
interfaces, with hardware safety as the primary design constraint.

```
tuf-crate      # GUI: dashboard, performance profiles, fans/thermals, battery,
               #      lighting, display & GPU — capability-driven, monitoring-first
tuf-crate-cli  # strictly read-only status (JSON or text)
```

## What it controls (all clamped, capability-gated, audited)
- Silent / Balanced / Performance (Turbo) — `platform_profile`
- Battery charge limit 20–100 % — `charge_control_end_threshold`
- Keyboard backlight — `asus::kbd_backlight`
- Panel overdrive — `panel_od`
- dGPU disable — `dgpu_disable` (double-confirmed)

## What it monitors
CPU package / NVMe / ACPI-zone temps · cpu_fan & gpu_fan RPM · GPU temp/util/power/
perf-state/throttle-reasons · battery capacity/health/threshold/AC · charge mode ·
firmware-attributes inventory · detected capability set.

## What it refuses (with firmware-level evidence)
Dynamic RGB, static-colour-from-Linux, fan curves, manual PWM, WMI power limits,
Dynamic Boost control, unnamed vendor DEVIDs — see `docs/UNSUPPORTED.md`.

## Documentation
| Doc | Content |
|---|---|
| `docs/RESEARCH.md` | Armoury Crate protocol research (WMNB/DEVID inventory from this machine's DSDT) |
| `docs/COMPATIBILITY.md` | Feature × interface matrix |
| `docs/SAFETY.md` | Binding safety policy and enforcement |
| `docs/ARCHITECTURE.md` | Design, write funnel, test seam |
| `docs/COMPARISON.md` | Feature-by-feature Armoury Crate comparison |
| `docs/UNSUPPORTED.md` | Unsupported features and the evidence for each |
| `docs/INSTALL.md` | Install/uninstall/backup/restore |

## Development
```
cargo test --workspace     # 13 tests (5 tuf-core + 8 tuf-hal) — fixture-based, never touches hardware
cargo clippy --workspace   # zero-warning policy
cargo build --release -p tuf-gui
tuf-crate --mock           # develop the GUI against the simulated machine
```
