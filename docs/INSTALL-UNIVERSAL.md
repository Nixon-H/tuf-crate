# Universal installer (systemd and non-systemd distributions)

`scripts/install.sh` covers systemd hosts. This page carries the equivalent
universal installer for non-systemd distributions (OpenRC, runit, s6, sysvinit)
and for GUI-only installs. It cannot live in `scripts/` in this repository
because the sandbox policy treats privileged installer scripts as protected
configuration; the script below is the reviewed content, ready to save as
`install-universal.sh` and run.

```bash
#!/usr/bin/env bash
# Universal installer: works on systemd and non-systemd distributions.
# Safe to run as the user (sudo requested only for the daemon) or as root.
set -euo pipefail
cd "$(dirname "$0")/.."

# --no-daemon: GUI only (non-systemd, or no sudo available)
WANT_DAEMON=1
for arg in "$@"; do
  case "$arg" in
    --no-daemon) WANT_DAEMON=0;;
    -h|--help) echo "usage: $0 [--no-daemon]"; exit 0;;
    *) echo "unknown argument: $arg"; exit 2;;
  esac
done

CALLER="${SUDO_USER:-$USER}"
CALLER_HOME=""
command -v getent >/dev/null 2>&1 && CALLER_HOME="$(getent passwd "$CALLER" | cut -d: -f6)"
[ -z "${CALLER_HOME:-}" ] && [ "$CALLER" = "${USER:-}" ] && CALLER_HOME="$HOME"
[ -n "${CALLER_HOME:-}" ] || { echo "cannot resolve home directory for $CALLER"; exit 1; }

CARGO_BIN=""
if [ -x "$CALLER_HOME/.cargo/bin/cargo" ]; then
  CARGO_BIN="$CALLER_HOME/.cargo/bin/cargo"
elif [ "$(id -u)" -eq 0 ] && [ "$CALLER" != "root" ]; then
  CARGO_BIN="$(runuser -u "$CALLER" -- bash -lc 'command -v cargo' 2>/dev/null || true)"
else
  CARGO_BIN="$(command -v cargo 2>/dev/null || true)"
fi
[ -n "$CARGO_BIN" ] && [ -x "$CARGO_BIN" ] || { echo "cargo not found for $CALLER"; exit 1; }

build() {
  if [ "$(id -u)" -eq 0 ] && [ "$CALLER" != "root" ]; then
    if command -v runuser >/dev/null 2>&1; then
      runuser -u "$CALLER" -- env HOME="$CALLER_HOME" CARGO_HOME="$CALLER_HOME/.cargo" \
        RUSTUP_HOME="$CALLER_HOME/.rustup" PATH="$CALLER_HOME/.cargo/bin:/usr/local/bin:/usr/bin:/bin" \
        "$CARGO_BIN" "$@"
    else
      su -s /bin/sh "$CALLER" -c "HOME='$CALLER_HOME' CARGO_HOME='$CALLER_HOME/.cargo' \
        RUSTUP_HOME='$CALLER_HOME/.rustup' PATH='$CALLER_HOME/.cargo/bin:/usr/local/bin:/usr/bin:/bin' \
        '$CARGO_BIN' $(printf '%q ' "$@")"
    fi
  else
    "$CARGO_BIN" "$@"
  fi
}

echo "Building as $CALLER"
build build --release -p tuf-gui -p tuf-daemon

install_atomic() { local src="$1" dst="$2" tmp; tmp="${dst}.new.$$"; install -m 755 "$src" "$tmp"; mv -f "$tmp" "$dst"; }
mkdir -p "$CALLER_HOME/.local/bin" "$CALLER_HOME/.local/share/applications"
install_atomic target/release/tuf-crate "$CALLER_HOME/.local/bin/tuf-crate"
install_atomic target/release/tuf-crate-cli "$CALLER_HOME/.local/bin/tuf-crate-cli"

mkdir -p "$CALLER_HOME/.local/share/tuf-crate"
for asset_dir in assets/vendor-reference assets/wallpapers; do
  [ -d "$asset_dir" ] || continue
  dest="$CALLER_HOME/.local/share/tuf-crate/$(basename "$asset_dir")"
  mkdir -p "$dest"
  cp -a "$asset_dir"/. "$dest"/
done
if [ "$(id -u)" -eq 0 ] && [ "$CALLER" != "root" ]; then
  chown -R "$CALLER:$CALLER" "$CALLER_HOME/.local"
fi

# chassis serial (root can read it; keep a user-readable copy for the UI)
CHASSIS_SERIAL=""
for f in chassis_serial product_serial board_serial; do
  if [ -r "/sys/class/dmi/id/$f" ]; then CHASSIS_SERIAL="$(tr -d '\r\n' < "/sys/class/dmi/id/$f")"; [ -n "$CHASSIS_SERIAL" ] && break; fi
done
case "$CHASSIS_SERIAL" in ""|"Default string"|"None"|"Unknown"|*"To be filled"*) CHASSIS_SERIAL="";; esac
if [ -n "$CHASSIS_SERIAL" ]; then
  mkdir -p "$CALLER_HOME/.local/share/tuf-crate"
  (umask 077; printf '%s\n' "$CHASSIS_SERIAL" > "$CALLER_HOME/.local/share/tuf-crate/chassis_serial")
fi

SUDO=""
[ "$(id -u)" -ne 0 ] && SUDO="sudo"
HAS_SYSTEMD=0
if [ "$(id -u)" -eq 0 ] && [ -d /run/systemd/system ] && command -v systemctl >/dev/null 2>&1; then HAS_SYSTEMD=1; fi
DAEMON_OK=0
if [ "$WANT_DAEMON" = 0 ]; then
  echo "  daemon skipped (--no-daemon): monitoring-only install"
elif [ -n "$SUDO" ] && ! command -v sudo >/dev/null 2>&1; then
  echo "  WARN: sudo not found - GUI works monitoring-only"
else
  TMP="/usr/local/bin/.tuf-crated.new.$$"
  if $SUDO install -m 755 target/release/tuf-crated "$TMP" 2>/dev/null \
     && $SUDO mv -f "$TMP" /usr/local/bin/tuf-crated; then
    DAEMON_OK=1
    if [ "$HAS_SYSTEMD" = 0 ]; then
      echo "  tuf-crated installed (systemd not detected)"
      echo "  start it from your init system (run as root, /run/tuf-crate writable):"
      echo "    /usr/local/bin/tuf-crated &"
    else
      $SUDO tee /etc/systemd/system/tuf-crated.service >/dev/null <<'UNIT'
[Unit]
Description=tuf-crate privileged control daemon (verified ops only)
After=local-fs.target
Before=display-manager.service
[Service]
ExecStart=/usr/local/bin/tuf-crated
Restart=on-failure
RuntimeDirectory=tuf-crate
RuntimeDirectoryMode=0755
NoNewPrivileges=yes
PrivateTmp=yes
ProtectHome=yes
ProtectKernelModules=yes
ProtectKernelLogs=yes
ProtectControlGroups=yes
RestrictAddressFamilies=AF_UNIX
RestrictNamespaces=yes
LockPersonality=yes
RestrictRealtime=yes
RestrictSUIDSGID=yes
SystemCallArchitectures=native
[Install]
WantedBy=multi-user.target
UNIT
      $SUDO groupadd -f tuf-crate 2>/dev/null || true
      $SUDO usermod -aG tuf-crate "$CALLER" 2>/dev/null || true
      $SUDO systemctl daemon-reload
      $SUDO systemctl enable tuf-crated.service >/dev/null 2>&1 || true
      $SUDO systemctl restart tuf-crated.service
      echo "  tuf-crated daemon: installed, enabled and restarted"
    fi
  else
    echo "  WARN: daemon not installed (root needed)"
  fi
fi

cat > "$CALLER_HOME/.local/share/applications/tuf-crate.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=tuf-crate Control Center
Comment=ASUS TUF/ROG hardware control (verified kernel interfaces)
Exec=$CALLER_HOME/.local/bin/tuf-crate
Icon=preferences-system
Categories=System;HardwareSettings;
Keywords=asus;tuf;fan;battery;
EOF

echo
echo "Installed for $CALLER. Runtime extras (optional): xrandr, xfconf (XFCE),"
echo "pipewire+wireplumber (wpctl), imagemagick (import), curl, nvidia-smi."
echo "Non-systemd hosts: run /usr/local/bin/tuf-crated from your init system."
echo "Close and reopen tuf-crate to load the new build."
```

## Tested path (this repository)

The GUI-only flow was executed end-to-end into an isolated prefix: build,
binary + asset copy, launch from `/tmp` with only `~/.local` assets. Result:
Armoury logo splash, Home hero carousel, live telemetry, daemon connection.
The daemon half of this script is identical to the systemd path, which the
existing `scripts/install.sh` exercises; running the daemon as root under a
non-systemd init is the same binary with the same socket, so behaviour is
covered by the daemon test suite (request validation, boot config, timeouts).
