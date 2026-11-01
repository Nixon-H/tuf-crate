# Install / Uninstall / Configuration backup

## Build requirements
Debian 13 with `build-essential`, `pkg-config`, `libxkbcommon-dev`, `libx11-dev` (present on this machine), Rust 1.80+ (rustup or distro).

## Install
```
scripts/install.sh            # builds release, installs to ~/.local/{bin,share}
```
What it does (and nothing more):
- `cargo build --release -p tuf-gui`
- copies `target/release/{tuf-crate,tuf-crate-cli}` → `~/.local/bin/`
- installs a desktop entry `~/.local/share/applications/tuf-crate.desktop` (launches GUI in monitoring-only mode; add `--writes` manually if you want controls enabled)
- does **not** create services, udev rules, sudo rules, or kernel modules

## Uninstall
```
scripts/uninstall.sh          # removes the two binaries + desktop entry + audit log
```
Removes only files install.sh created. No system component is modified, so nothing needs restoring.

## Configuration backup & restoration
- Every write is recorded to `~/.local/state/tuf-crate/audit.jsonl` (JSONL: timestamp, attribute, value).
- `scripts/backup-config.sh` snapshots the current values of the five verified attributes to `tuf-crate-config-backup-<date>.json`.
- `scripts/restore-config.sh <backup.json>` writes the recorded values back through the same clamped, audited HAL path (requires root for sysfs writes; it will prompt).

## Running
```
tuf-crate                 # GUI, monitoring-only (safe default)
tuf-crate --writes        # GUI with the five verified controls enabled
tuf-crate --mock          # simulated machine (development/demo)
tuf-crate-cli [--json]    # strictly read-only status (no write code paths)
```

## Verification after install
```
tuf-crate-cli             # must show model FX706HF-verified: true and live telemetry
cargo test --workspace    # 17/17 tests (never touches hardware)
```


## Local Armoury reference assets (optional, not redistributed)

The recreated UI can use the owner's local Armoury Crate assets for exact branding,
typography, theme previews and the splash logo:

```
assets/vendor-reference/          # gitignored, proprietary ASUS assets
├── branding/AC_Logo.png
├── fonts/ROGFonts-Regular.ttf
├── themes/ac_settings_img_*.png
├── wallpapers/                   # any locally extracted vendor wallpapers
└── manifest/assets-manifest.json # SHA-256, sizes, dimensions
```

These are extracted read-only from the Windows installation (see
`docs/WINDOWS-ASSET-INVENTORY.md`). Without them the application still runs: it falls back
to drawn icons/vector art and the default font; the splash uses a text fallback.

Redistributable wallpapers created for this project live in `assets/wallpapers/` and ARE
committed.
