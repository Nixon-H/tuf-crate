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
cargo test --workspace          # 17 tests (5 tuf-core + 12 tuf-hal), fixture-based
cargo clippy --workspace        # zero warnings policy
cargo build --release -p tuf-gui
```


## UI layer, assets and theming (2026-11-01 redesign)

- `crates/tuf-gui/src/assets.rs` loads **local-only** vendor assets from
  `assets/vendor-reference/` (gitignored): the original Armoury logo (splash), the ROG
  display font, theme preview images and wallpapers. Missing assets fall back gracefully.
- The GUI shell reproduces Armoury's information architecture: icon sidebar with
  Playground/Assistant groups and bottom navigation, page title bar, borderless Home hero +
  usage columns, System Settings sub-tabs, Lighting & Aura Sync tabs, Settings
  General/Update Center/About.
- Theme selection (TUF/ROG/PRIME/ProArt/Light/Follow System) persists in
  `~/.config/tuf-crate/ui.json`; a shared accent token retints custom-painted widgets and
  egui visuals.
- All hardware writes still go through the existing `Backend`/daemon safety funnel; the UI
  additions never touch sysfs/EC directly.
- Visual comparison tooling: `scripts/ui-compare.sh` (RMSE/SSIM + side-by-side/diff),
  results in `docs/UI-COMPARISON.md`; reference inventory in
  `docs/WINDOWS-ASSET-INVENTORY.md`; screen gap matrix in `docs/UI-GAP.md`.
