# Hardware Safety Documentation — tuf-crate

**Priority order: hardware safety > feature completeness.** This document is binding for every change to this repository.

## 1. The safety boundary (enforced by construction)

- Hardware writes are expressible **only** as `tuf_core::VerifiedAttr` — a closed enum of five kernel-sanctioned attributes:
  1. `platform_profile` (thermal/fan policy)
  2. `charge_control_end_threshold` (battery, 20–100 %)
  3. `asus::kbd_backlight/brightness` (0..max_brightness)
  4. `panel_od` (0/1)
  5. `dgpu_disable` (0/1, **double-confirmation gated**)
- There is **no API** in the workspace to: write an arbitrary path, read/write EC registers (ECRAM), invoke ACPI methods, run `acpi_call`, load kernel modules, patch kernels, or write `pwm*_enable`/`pwm*` values.
- `SysfsBackend::verified_write()` is the single write funnel: capability check → confirmation check → strict range clamp (out-of-range **errors**, never silently clamps) → write → JSONL audit.
- Writes are **disabled by default**: `SysfsBackend::real(false, …)` is monitoring-only. The GUI passes `--writes` to enable the five verified ops; the CLI has no write paths at all.

## 2. Prohibited operations (never implement)

Blind/undocumented EC register writes · experimental fan-control commands · unverified ACPI method invocation · BIOS/EC firmware modification or flashing · voltage/clock/power-limit forcing (incl. WMI PPT writes) · bypassing thermal protections · disabling ASUS kernel drivers · loading unverified kernel modules · direct userspace control competing with TLP/thermald/nvidia-powerd.

## 3. Model scoping

Capabilities are detected at runtime per-attribute (interface existence), and `CapabilitySet::is_fx706hf` records whether the machine is the verified model. Interfaces absent on the machine are refused even if present in the enum. Behaviour on other ASUS models is per-attribute safe (each attr is a standard kernel interface) but is **untested** — the GUI shows the model and its verification state.

## 4. Why the unsupported features are unsupported (evidence)

See `docs/RESEARCH.md` §4–5 and `docs/UNSUPPORTED.md`. Headlines: this firmware returns `0xFFFFFFFE` (unsupported) for WMI RGB GETs and no-ops for fan-curve DEVIDs; `pwm*_enable` can stop fans; PPT DEVIDs are AMD-oriented and rejected by this Intel platform's firmware.

## 5. Testing posture

The `SysfsBackend` takes a root prefix; the entire test suite runs against fixture sysfs trees (`tempfile`-based). **No test ever touches real hardware.** MockBackend covers GUI development. `cargo test` must pass before any deployment.

## 6. Rollback

Every write path is a sysfs attribute that can be written back (profiles: echo previous choice; charge threshold: echo 100; brightness/panel_od/dgpu_disable likewise). The audit log (`~/.local/state/tuf-crate/audit.jsonl`) records every applied change with timestamps to make reversal deterministic. `scripts/uninstall.sh` removes the application without touching any system component.

## 7. Known interactions

- **TLP**: owns CPU governor/EPP and Wi-Fi powersave. tuf-crate never writes cpufreq — the Performance tab displays, not controls.
- **thermald / nvidia-powerd**: own thermal/GPU power policy. tuf-crate reads telemetry only from them.
- **asus-armoury firmware-attributes**: tuf-crate *displays* them and prefers the dedicated kernel attrs it already uses; it does not write firmware-attributes concurrently with anything else.
