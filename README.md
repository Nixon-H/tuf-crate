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

## How controls work (privileged daemon)
`scripts/install.sh` installs a root systemd service (**tuf-crated**) that exposes exactly the
five verified operations over a local socket — the same privileged-daemon pattern as asusd and
power-profiles-daemon. The GUI detects it automatically: **controls work out of the box**, no
CLI flags. Every daemon write is closed-enum, clamped, capability-gated and audited
(`/var/lib/tuf-crate/audit.jsonl`). Without the daemon the app stays monitoring-only.

**Fans**: switching Silent/Balanced/Performance drives the firmware's own thermal policy —
measured live on the FX706HF: ~4000 RPM (balanced) → ~4400 RPM (performance) within seconds.
Custom per-temperature fan curves remain unsupported by this model's firmware (see
`docs/UNSUPPORTED.md`).

## Status (verified on the target machine)
- 17/17 tests · clippy 0 warnings · `cargo audit` 0 vulnerabilities (324 deps)
- Read-only hardware verification on FX706HF: model detected, profiles, fans, battery health/threshold, NVIDIA telemetry, firmware-attributes
- Full UI walkthrough (all 7 tabs + every interactive flow) on an isolated virtual display — see `docs/UI-VERIFICATION.md`
- Requirement-by-requirement compliance with the original Armoury-Crate-recreation goal: `docs/REQUIREMENTS-COMPLIANCE.md`

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
| `docs/REQUIREMENTS-COMPLIANCE.md` | original goal → artifact mapping |
| `docs/DEPENDENCY-AUDIT.md` | cargo audit results |
| `docs/UI-VERIFICATION.md` | UI walkthrough + bug-audit record |

## Development
```
cargo test --workspace     # 17 tests (5 tuf-core + 12 tuf-hal) — fixture-based, never touches hardware
cargo clippy --workspace   # zero-warning policy
cargo build --release -p tuf-gui
tuf-crate --mock           # develop the GUI against the simulated machine
```
