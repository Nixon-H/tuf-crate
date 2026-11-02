#!/usr/bin/env bash
# Automated visual comparison: Windows Armoury Crate reference vs TUF Crate.
#
# Usage: scripts/ui-compare.sh <windows-screenshots-dir> <linux-captures-dir> [out-dir]
# Windows screenshots and generated composites are LOCAL ONLY (proprietary UI);
# the default output lives under the gitignored assets/vendor-reference/.
set -uo pipefail
WIN=${1:?windows screenshot dir}
LIN=${2:?linux capture dir}
OUT=${3:-assets/vendor-reference/ui-compare}
mkdir -p "$OUT"

# name|windows-file|linux-file|fullscreen(0/1)
PAIRS=(
  "home|Screenshot_2026-10-31_055240.png|cap_home.png|0"
  "sysconfig|Screenshot_2026-10-31_055321.png|final_devices_sys.png|0"
  "freememory|Screenshot_2026-10-31_055340.png|final_devices_free.png|0"
  "gpuperformance|Screenshot_2026-10-31_055349.png|final_devices_gpu.png|0"
  "deviceslighting|Screenshot_2026-10-31_055358.png|final_devices_lighting.png|0"
  "splash|Screenshot_2026-10-31_060313.png|cap_splash.png|1"
  "settingsgeneral|Screenshot_2026-10-31_063849.png|final_settings_general.png|0"
  "updatecenter|Screenshot_2026-10-31_063929.png|final_settings_update.png|0"
  "syncdevices|Screenshot_2026-10-31_064317.png|final_aura_sync.png|0"
  "auraeffects|Screenshot_2026-10-31_064323.png|final_aura_effects2.png|0"
  "aiaiura|Screenshot_2026-10-31_064331.png|final_aura_ai.png|0"
  "aurawallpaper|Screenshot_2026-10-31_064402.png|final_wallpaper.png|0"
)

printf 'screen,rmse,ssim\n' > "$OUT/metrics.csv"
printf '| Screen | RMSE (normalised) | SSIM |\n|---|---|---|\n' > "$OUT/metrics.md"

for row in "${PAIRS[@]}"; do
  IFS='|' read -r name win lin full <<<"$row"
  [ -f "$WIN/$win" ] || { echo "missing $WIN/$win"; continue; }
  [ -f "$LIN/$lin" ] || { echo "missing $LIN/$lin"; continue; }
  if [ "$full" = "1" ]; then
    convert "$WIN/$win" -resize 1600x900! "$OUT/${name}_win.png"
    convert "$LIN/$lin" -resize 1600x900! "$OUT/${name}_lin.png"
  else
    convert "$WIN/$win" -crop 1920x1000+0+0 +repage -resize 1600x900! "$OUT/${name}_win.png"
    convert "$LIN/$lin" -crop 1920x1000+0+0 +repage -resize 1600x900! "$OUT/${name}_lin.png"
  fi
  rmse=$(compare -metric RMSE "$OUT/${name}_win.png" "$OUT/${name}_lin.png" null: 2>&1 | sed -E 's/.*\(([0-9.]+)\).*/\1/')
  ssim=$(compare -metric SSIM "$OUT/${name}_win.png" "$OUT/${name}_lin.png" null: 2>&1 | sed -E 's/.*\(([0-9.]+)\).*/\1/')
  compare "$OUT/${name}_win.png" "$OUT/${name}_lin.png" "$OUT/${name}_diff.png" 2>/dev/null || true
  montage "$OUT/${name}_win.png" "$OUT/${name}_lin.png" -tile 2x1 -geometry +4+4 -background '#111' "$OUT/${name}_side.png"
  printf '%s,%s,%s\n' "$name" "$rmse" "$ssim" >> "$OUT/metrics.csv"
  printf '| %s | %s | %s |\n' "$name" "$rmse" "$ssim" >> "$OUT/metrics.md"
done
echo "Written to $OUT (metrics.csv / metrics.md / side-by-side / diff images)"
