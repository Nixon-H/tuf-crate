#!/usr/bin/env bash
# Fetch the official ASUS TUF Gaming F17 product renders used as the Home hero.
# Sources are the same ASUS CDN URLs the runtime device-image pipeline uses
# (see docs/HERO-IMAGE-PIPELINE.md). The images are ASUS-copyrighted artwork;
# this repository commits them as reference material at the owner's request.
# Prefer the built-in runtime fetch (part number -> odinapi) when a part number
# is available; this script is the manual fallback.
set -euo pipefail
DEST="$(cd "$(dirname "$0")/.." && pwd)/assets/vendor-reference/hero-device"
mkdir -p "$DEST"
G1="https://dlcdnwebimgs.asus.com/gain/572e7a4d-9581-4f36-9465-eb53fc8e561d"
G2="https://dlcdnwebimgs.asus.com/gain/2d490885-3512-463d-893d-01ba1c3f7d98"
fetch() { # url dest
  curl -fsSL --max-time 30 --max-filesize 20971520 -A 'Mozilla/5.0' "$1" -o "$2" \
    || { echo "fetch failed: $1" >&2; rm -f "$2"; return 1; }
}
fetch "$G1" "$DEST/FX706HF.png"
fetch "$G2" "$DEST/FX706HF_alt.png"
sha256sum "$DEST/FX706HF.png" "$DEST/FX706HF_alt.png"
echo "Hero images stored in $DEST (ASUS-copyrighted reference material)."
