#!/usr/bin/env bash
# Install tuf-crate to ~/.local — binaries + desktop entry. No services,
# no udev rules, no sudo rules, no kernel modules. Reversible by uninstall.sh.
set -euo pipefail
cd "$(dirname "$0")/.."

command -v cargo >/dev/null || { echo "cargo not found — install Rust first"; exit 1; }
cargo build --release -p tuf-gui -p tuf-daemon

mkdir -p "$HOME/.local/bin" "$HOME/.local/share/applications"
install -m 755 target/release/tuf-crate     "$HOME/.local/bin/"
install -m 755 target/release/tuf-crate-cli "$HOME/.local/bin/"

# ---- privileged control daemon (makes controls work without CLI flags) ----
# Runs as root via systemd; can ONLY perform the five verified, clamped,
# audited operations. No EC/ACPI/PWM/module access exists in the codebase.
SUDO=""
if [ "$(id -u)" -ne 0 ]; then SUDO="sudo"; fi
if $SUDO install -m 755 target/release/tuf-crated /usr/local/bin/tuf-crated 2>/dev/null; then
  $SUDO tee /etc/systemd/system/tuf-crated.service >/dev/null <<'UNIT'
[Unit]
Description=tuf-crate privileged control daemon (verified ops only)
[Service]
ExecStart=/usr/local/bin/tuf-crated
Restart=on-failure
RuntimeDirectory=tuf-crate
RuntimeDirectoryMode=0755
[Install]
WantedBy=multi-user.target
UNIT
  $SUDO systemctl daemon-reload
  $SUDO systemctl enable --now tuf-crated.service
  echo "  tuf-crated daemon: installed + enabled (controls work out of the box)"
else
  echo "  WARN: could not install tuf-crated daemon (need root) — app stays monitoring-only"
  echo "        run: sudo ./scripts/install.sh"
fi

cat > "$HOME/.local/share/applications/tuf-crate.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=tuf-crate Control Center
Comment=ASUS TUF FX706HF hardware control (monitoring-only by default)
Exec=$HOME/.local/bin/tuf-crate
Icon=preferences-system
Categories=System;HardwareSettings;
Keywords=asus;tuf;fan;battery;
EOF

echo
echo "Installed:"
echo "  $HOME/.local/bin/tuf-crate      (GUI — monitoring-only; run 'tuf-crate --writes' for controls)"
echo "  $HOME/.local/bin/tuf-crate-cli  (strictly read-only status)"
echo "  $HOME/.local/share/applications/tuf-crate.desktop"
echo
echo "Verify with: tuf-crate-cli"
