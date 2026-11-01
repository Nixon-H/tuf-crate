#!/usr/bin/env bash
# Fetch the official ASUS TUF Gaming F17 product render used as the Home hero.
# The image is ASUS marketing artwork: it is stored LOCALLY ONLY under the
# gitignored assets/vendor-reference/hero-device/ and never redistributed.
set -euo pipefail
DEST="$(cd "$(dirname "$0")/.." && pwd)/assets/vendor-reference/hero-device"
mkdir -p "$DEST"
BASE="https://dlcdnwebimgs.asus.com/files/media/d7fe2016-2358-497d-a685-29630e8be94a/v1/images/desktop/design"
curl -fsSL -A 'Mozilla/5.0' "$BASE/design_pd_01.png" -o "$DEST/FX706HF.png"
curl -fsSL -A 'Mozilla/5.0' "$BASE/design_pd_02.png" -o "$DEST/FX706HF_alt.png"
curl -fsSL -A 'Mozilla/5.0' "$BASE/design_pd_03.png" -o "$DEST/FX706HF_keyboard.png"
sha256sum "$DEST"/*.png
echo "Hero images stored locally in $DEST (not committed)."
