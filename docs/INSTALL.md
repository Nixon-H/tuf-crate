# Install / Uninstall / Configuration backup

## Clone
```
git clone https://github.com/Nixon-H/tuf-crate.git
cd tuf-crate
```

## Build requirements
Rust 1.80+ (rustup or the distribution package), `git`, a C toolchain, `pkg-config`,
X11/OpenGL development headers, and `libxkbcommon`:
- Debian/Ubuntu: `build-essential pkg-config libxkbcommon-dev libx11-dev libxi-dev libgl1-mesa-dev`
- Fedora: `gcc pkgconf-pkg-config libxkbcommon-devel libX11-devel libXi-devel mesa-libGL-devel`
- Arch: `base-devel pkgconf libxkbcommon libx11 libxi mesa`

Runtime extras (optional): `xrandr`, `xfconf` (XFCE wallpaper), PipeWire + WirePlumber
(`wpctl`), ImageMagick (`import`), `curl`, `nvidia-smi`, `df`.

## Install
```
sudo ./scripts/install.sh     # build still runs as your user; GUI into your home
```
or, for a GUI-only install without root:
```
./scripts/install.sh          # daemon step is skipped (monitoring-only)
```
Non-systemd distributions (OpenRC/runit/s6/sysvinit): see
[`INSTALL-UNIVERSAL.md`](INSTALL-UNIVERSAL.md) for the manual path.
What it does:
- builds `tuf-gui` + the verified `tuf-crated` daemon as **your user** (no root-owned target/);
- copies `tuf-crate` and `tuf-crate-cli` into **your** `~/.local/bin/`;
- installs the desktop entry into **your** `~/.local/share/applications/`;
- installs `tuf-crated` + `tuf-crated.service` (root), ordered **before display-manager**, and
  restarts it. The GUI finds it automatically; no `--writes` flag is needed for normal controls;
- creates no udev rules, sudo rules or kernel modules.

If you only want the GUI, run the script without root: it installs the binaries, assets
and desktop entry, skips the privileged daemon (with a warning), and the app stays
monitoring-only. `--no-daemon` in [`INSTALL-UNIVERSAL.md`](INSTALL-UNIVERSAL.md) is the
explicit equivalent for non-systemd hosts.

## Run
```
tuf-crate                     # GUI (finds the daemon automatically)
tuf-crate-cli                 # read-only status
tuf-crate --mock              # simulated machine for UI development
```

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
cargo test --workspace    # 41/42 tests (never touches real hardware)
```


## Local Armoury reference assets (optional, not redistributed)

The recreated UI can use the owner's local Armoury Crate assets for exact branding,
typography, theme previews and the splash logo:

```
assets/vendor-reference/          # original Armoury resources (copyright ASUS, reference only)
├── branding/AC_Logo.png
├── fonts/ROGFonts-Regular.ttf
├── themes/ac_settings_img_*.png
├── wallpapers/                   # any locally extracted vendor wallpapers
└── manifest/assets-manifest.json # SHA-256, sizes, dimensions
```

These are extracted read-only from the Windows installation (see
`archive/docs/WINDOWS-ASSET-INVENTORY.md`). Without them the application still runs: it falls back
to drawn icons/vector art and the default font; the splash uses a text fallback.

Redistributable wallpapers created for this project live in `assets/wallpapers/` and ARE
committed.
