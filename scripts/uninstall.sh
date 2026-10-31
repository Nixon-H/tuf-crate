#!/usr/bin/env bash
# Remove tuf-crate: only the files install.sh created. No system component
# is touched, so no restoration is needed.
set -euo pipefail
rm -fv "$HOME/.local/bin/tuf-crate" "$HOME/.local/bin/tuf-crate-cli"
rm -fv "$HOME/.local/share/applications/tuf-crate.desktop"
if [ "$(id -u)" -eq 0 ]; then S=""; else S="sudo"; fi
$S systemctl disable --now tuf-crated.service 2>/dev/null || true
$S rm -f /etc/systemd/system/tuf-crated.service /usr/local/bin/tuf-crated
$S systemctl daemon-reload 2>/dev/null || true
echo "  tuf-crated daemon removed"
echo "Audit log kept at: ${XDG_STATE_HOME:-$HOME/.local/state}/tuf-crate/audit.jsonl"
echo "tuf-crate removed."
