# Architecture — tuf-crate

```
┌────────────────────────────────────────────────────────┐
│ tuf-gui (eframe/egui)      tuf-crate-cli (read-only)   │
│   tabs · polling · confirmation UI                         │
└──────────────┬─────────────────────────────────────────┘
               │ Backend trait (semantic surface only)
┌──────────────┴─────────────────────────────────────────┐
│ tuf-hal                                                │
│  SysfsBackend { root }  ← root prefix = safety seam    │
│  MockBackend            ← simulation for dev/tests     │
│  nvidia::query()        ← read-only nvidia-smi         │
└──────────────┬─────────────────────────────────────────┘
               │ VerifiedAttr (closed enum) + clamp + audit
┌──────────────┴─────────────────────────────────────────┐
│ tuf-core: CapabilitySet · SafetyError · AuditLog       │
└────────────────────────────────────────────────────────┘
               │
   kernel-sanctioned sysfs only:
   /sys/firmware/acpi/platform_profile{,_choices}
   /sys/class/power_supply/BAT1/*  ACAD/online
   /sys/class/leds/asus::kbd_backlight/*
   /sys/devices/platform/asus-nb-wmi/{panel_od,dgpu_disable}
   /sys/class/hwmon/* (fans, coretemp, nvme, acpitz)
   /sys/class/firmware-attributes/asus-armoury/* (read)
```

## Key decisions

1. **Root-prefix seam.** `SysfsBackend::with_root(fixture, …)` is the same code path as `real()` — the entire test suite exercises the real implementation against fake trees; tests never touch hardware.
2. **Write surface = closed enum.** The GUI cannot represent a write outside the five verified attributes; there is no stringly-typed path write anywhere in the workspace.
3. **Monitoring-first defaults.** `real(false)` = monitoring-only; writes require `--writes` (GUI) and are impossible in the CLI by construction.
4. **No daemon.** The GUI polls at 1 Hz in-process. No background service competes with TLP/thermald/NetworkManager (Phase-7 requirement).
5. **Capability-driven UI.** Tabs hide controls the kernel does not expose on this machine (`CapabilitySet::allows`), and the safety layer re-checks every write regardless of UI state.
6. **Audit log.** JSONL at `$XDG_STATE_HOME/tuf-crate/audit.jsonl`: timestamp, attribute, value — the rollback key.

## Data flow (write)

```
UI action → Backend::set_x(value, confirmed)
  → CapabilitySet::validate(attr, confirmed)        # capability + confirmation gates
  → tuf_core::clamp(attr, value, led_max)            # strict range check (errors, no silent clamp)
  → fs::write(<verified path>, value)
  → AuditLog::record(attr, value)
```

## Build

```
cargo test --workspace          # 20 tests, fixture-based
cargo clippy --workspace        # zero warnings policy
cargo build --release -p tuf-gui
```
