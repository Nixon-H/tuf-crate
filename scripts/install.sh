#!/usr/bin/env bash
# Install tuf-crate to ~/.local — binaries + desktop entry. No services,
# no udev rules, no sudo rules, no kernel modules. Reversible by uninstall.sh.
set -euo pipefail
cd "$(dirname "$0")/.."

command -v cargo >/dev/null || { echo "cargo not found — install Rust first"; exit 1; }
cargo build --release -p tuf-gui

mkdir -p "$HOME/.local/bin" "$HOME/.local/share/applications"
install -m 755 target/release/tuf-crate     "$HOME/.local/bin/"
install -m 755 target/release/tuf-crate-cli "$HOME/.local/bin/"

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
