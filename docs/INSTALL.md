# Install / Uninstall / Configuration backup

## Build requirements
Debian 13 with `build-essential`, `pkg-config`, `libxkbcommon-dev`, `libx11-dev` (present on this machine), Rust 1.80+ (rustup or distro).

## Install
```
scripts/install.sh            # as your user; sudo is requested only for the daemon
# or
sudo scripts/install.sh       # build still runs as your user, installs into your home
```
What it does:
- builds `tuf-gui` + the verified `tuf-crated` daemon as **your user** (no root-owned target/);
- copies `tuf-crate` and `tuf-crate-cli` into **your** `~/.local/bin/`;
- installs the desktop entry into **your** `~/.local/share/applications/`;
- installs `tuf-crated` + `tuf-crated.service` (root), ordered **before display-manager**, and
  restarts it. The GUI finds it automatically; no `--writes` flag is needed for normal controls;
- creates no udev rules, sudo rules or kernel modules.

If you only want the GUI, run the script without root and decline the daemon install; the
app then stays monitoring-only.

## Uninstall
```
scripts/uninstall.sh          # removes the two binaries + desktop entry + audit log
```
Removes only files install.sh created. No system component is modified, so nothing needs restoring.

## Configuration backup & restoration
- Every daemon write is recorded to `/var/lib/tuf-crate/audit.jsonl` (JSONL: timestamp, attribute, value).
- `scripts/backup-config.sh` snapshots the current values of the five verified attributes to `tuf-crate-config-backup-<date>.json`.
- `scripts/restore-config.sh <backup.json>` writes the recorded values back through the same clamped, audited HAL path (requires root for sysfs writes; it will prompt).

## Running
```
tuf-crate                 # GUI; uses tuf-crated automatically (verified controls enabled)
tuf-crate --writes        # direct-sysfs mode for test machines without the daemon (needs root)
tuf-crate --mock           # simulated machine
tuf-crate-cli [--json]    # strictly read-only status (no write code paths)
```

## Verification after install
```
tuf-crate-cli             # must show model FX706HF-verified: true and live telemetry
cargo test --workspace    # 41/41 tests (never touches real hardware)
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
