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
- CPU sustained power cap 25–65 W — `intel-rapl` (user-authorized; lowers heat; resets on reboot)

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
measured live on the FX706HF: ~4000 RPM (balanced) → ~4400 RPM (performance) within seconds,
auto-ramping to 5500 RPM under sustained load. Custom per-temperature fan curves remain
unsupported by this model's firmware (see `docs/UNSUPPORTED.md`). Settings (profile, power
cap, fan mode) can be persisted across reboots from the GUI ("24/7 persistence").

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
| `docs/WINDOWS-STACK.md` | Windows ASUS component inventory vs Linux |

## Development
```
cargo test --workspace     # 17 tests (5 tuf-core + 12 tuf-hal) — fixture-based, never touches hardware
cargo clippy --workspace   # zero-warning policy
cargo build --release -p tuf-gui
tuf-crate --mock           # develop the GUI against the simulated machine
```

---

## 🎯 RGB protocol recovered — next session (2026-11-01)

The keyboard-lighting command chain was disassembled out of the (memory-liberated) ASUS
hardware service. The complete path Armoury Crate uses:

```
Armoury Crate UI → KEYBDLIGHTEX handler (0x27f9b)
  → worker threads with command pairs (0x50,4) (0x100,4) (0x80,5) (0x180,5) (0x100,6)
  → thread proc 0x73c0
  → WMNB 'DEVS' { IIA0 = 0x00120057, IIA1 = command }
  → \.\ATKACPI\ASLDSRV driver → firmware
```

**The door nobody ever knocked on is DEVID `0x00120057`** — "TUF_RGB_CMD", one of the
"unnamed firmware-private" DEVIDs from the original research. G-Helper and every earlier
attempt used `0x00100056/5A` — wrong doors. And the interface it travels through is **the
exact WMNB interface the Linux kernel's `asus_wmi` driver already speaks** — meaning no raw
EC, no AsIO, no vendor drivers: the RGB control lands squarely inside tuf-crate's existing
safety model.

The Windows ATK wrapper can't confirm it (its own whitelist gives "Generic failure" even
for known-good DEVIDs), so the verdict test runs on Linux — `kernel/tuf_rgb_probe.c`:
read-only by default (DSTS query), with the benign apply-command test gated behind
`probe_devs=1`.

### Next session (on the Linux side)

```bash
cd ~/Projects/tuf-crate && git pull
cd kernel && make -C /lib/modules/$(uname -r)/build M=$PWD modules
sudo insmod tuf_rgb_probe.ko && dmesg | grep tuf_rgb_probe
```

- If DSTS answers with anything but `0xFFFFFFFE` → **channel live, WMI RGB confirmed on
  this model**.
- Then `sudo rmmod tuf_rgb_probe && sudo insmod tuf_rgb_probe.ko probe_devs=1` → the DEVS
  apply test.

If that responds, the last RE hop (how mode/colour/speed pack into the command value)
finishes the protocol, and `kernel/tuf_rgb.c` becomes a real driver — through the
sanctioned interface, with the allowlist design already in place. Full details:
`docs/WINDOWS-PLUGIN-DISCOVERY.md` §6.
