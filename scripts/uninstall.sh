#!/usr/bin/env bash
# Remove tuf-crate: only the files install.sh created. No system component
# is touched, so no restoration is needed.
set -euo pipefail
rm -fv "$HOME/.local/bin/tuf-crate" "$HOME/.local/bin/tuf-crate-cli"
rm -fv "$HOME/.local/share/applications/tuf-crate.desktop"
echo "Audit log kept at: ${XDG_STATE_HOME:-$HOME/.local/state}/tuf-crate/audit.jsonl"
echo "tuf-crate removed."
