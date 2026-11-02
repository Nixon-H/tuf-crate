#!/usr/bin/env bash
# Install tuf-crate for the invoking user, plus the privileged verified daemon.
#
# Safe to run either as the user (`./scripts/install.sh`, sudo is requested only
# for the daemon) or as root (`sudo ./scripts/install.sh`). In the latter case
# the build runs as the original user and the GUI is installed into that user's
# home — never into /root.
set -euo pipefail
cd "$(dirname "$0")/.."

CALLER="${SUDO_USER:-$USER}"
CALLER_HOME="$(getent passwd "$CALLER" | cut -d: -f6)"
[ -n "$CALLER_HOME" ] || { echo "cannot resolve home directory for $CALLER"; exit 1; }

# Locate cargo in the caller's environment (sudo drops ~/.cargo/bin from PATH).
CARGO_BIN=""
# Prefer the caller's rustup shim directly (sudo drops ~/.cargo/bin from PATH).
if [ -x "$CALLER_HOME/.cargo/bin/cargo" ]; then
  CARGO_BIN="$CALLER_HOME/.cargo/bin/cargo"
elif [ "$(id -u)" -eq 0 ] && [ "$CALLER" != "root" ]; then
  CARGO_BIN="$(runuser -u "$CALLER" -- bash -lc 'command -v cargo' 2>/dev/null || true)"
else
  CARGO_BIN="$(command -v cargo 2>/dev/null || true)"
fi
if [ -z "$CARGO_BIN" ] || [ ! -x "$CARGO_BIN" ]; then
  echo "cargo not found for $CALLER."
  echo "Install Rust (https://rustup.rs) as $CALLER, then re-run this script."
  exit 1
fi

build() {
  if [ "$(id -u)" -eq 0 ] && [ "$CALLER" != "root" ]; then
    runuser -u "$CALLER" -- env \
      HOME="$CALLER_HOME" \
      CARGO_HOME="$CALLER_HOME/.cargo" \
      RUSTUP_HOME="$CALLER_HOME/.rustup" \
      PATH="$CALLER_HOME/.cargo/bin:/usr/local/bin:/usr/bin:/bin" \
      "$CARGO_BIN" "$@"
  else
    "$CARGO_BIN" "$@"
  fi
}

# Build as the user so target/ never becomes root-owned.
echo "Building as $CALLER with $CARGO_BIN"
build build --release -p tuf-gui -p tuf-daemon

install_atomic() { # src dst
  local src="$1" dst="$2" tmp
  tmp="${dst}.new.$$"
  install -m 755 "$src" "$tmp"
  mv -f "$tmp" "$dst"
}

mkdir -p "$CALLER_HOME/.local/bin" "$CALLER_HOME/.local/share/applications"
install_atomic target/release/tuf-crate "$CALLER_HOME/.local/bin/tuf-crate"
install_atomic target/release/tuf-crate-cli "$CALLER_HOME/.local/bin/tuf-crate-cli"
if [ "$(id -u)" -eq 0 ]; then
  chown "$CALLER:$CALLER" "$CALLER_HOME/.local/bin/tuf-crate" "$CALLER_HOME/.local/bin/tuf-crate-cli"
fi

# ---- privileged control daemon (verified, clamped, audited operations) ----
SUDO=""
if [ "$(id -u)" -ne 0 ]; then SUDO="sudo"; fi
DAEMON_OK=0
DAEMON_TMP="/usr/local/bin/.tuf-crated.new.$$"
if $SUDO install -m 755 target/release/tuf-crated "$DAEMON_TMP" 2>/dev/null \
   && $SUDO mv -f "$DAEMON_TMP" /usr/local/bin/tuf-crated; then
  DAEMON_OK=1
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
[Install]
WantedBy=multi-user.target
UNIT
  $SUDO groupadd -f tuf-crate 2>/dev/null || true
  $SUDO usermod -aG tuf-crate "$CALLER" 2>/dev/null || true
  $SUDO systemctl daemon-reload
  $SUDO systemctl enable tuf-crated.service >/dev/null 2>&1 || true
  # restart so the newly installed boot-time feature (dGPU/battery) is active
  $SUDO systemctl restart tuf-crated.service
  echo "  tuf-crated daemon: installed, enabled and restarted"
else
  echo "  WARN: daemon not installed (root needed). GUI still works monitoring-only."
  echo "        re-run: sudo $PWD/scripts/install.sh"
fi

cat > "$CALLER_HOME/.local/share/applications/tuf-crate.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=tuf-crate Control Center
Comment=ASUS TUF FX706HF hardware control (verified kernel interfaces)
Exec=$CALLER_HOME/.local/bin/tuf-crate
Icon=preferences-system
Categories=System;HardwareSettings;
Keywords=asus;tuf;fan;battery;
EOF
if [ "$(id -u)" -eq 0 ]; then
  chown "$CALLER:$CALLER" "$CALLER_HOME/.local/share/applications/tuf-crate.desktop"
fi

echo
echo "Installed for $CALLER:"
echo "  $CALLER_HOME/.local/bin/tuf-crate      (GUI)"
echo "  $CALLER_HOME/.local/bin/tuf-crate-cli  (read-only status)"
echo "  $CALLER_HOME/.local/share/applications/tuf-crate.desktop"
[ "$DAEMON_OK" = 1 ] && echo "  /usr/local/bin/tuf-crated + tuf-crated.service"
echo
echo "Close and reopen tuf-crate to load the new build."
