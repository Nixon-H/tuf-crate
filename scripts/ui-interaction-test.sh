#!/usr/bin/env bash
# Headless interaction/animation test for the Armoury-style UI.
# Runs entirely under Xvfb; never touches the owner's live desktop.
set -uo pipefail
OUT=${1:-/tmp/tuf-ui-test}
mkdir -p "$OUT"
export DISPLAY=:99
export LIBGL_ALWAYS_SOFTWARE=1
Xvfb :99 -screen 0 1920x1080x24 >/tmp/xvfb-ui.log 2>&1 &
XPID=$!
sleep 1.5
TUF_CRATE_SKIP_SPLASH=1 /home/nixon/.local/bin/tuf-crate >/tmp/gui-ui.log 2>&1 &
APID=$!
sleep 5
fail=0
alive() { if ! kill -0 "$APID" 2>/dev/null; then echo "FAIL: app exited after $1"; fail=1; return 1; fi; }
shot() { import -window root "$OUT/$1.png"; }
click() { xdotool mousemove "$1" "$2" click 1; sleep 0.4; alive "$3" || true; shot "$3"; }

# sidebar pages
click 110 117 devices
click 110 181 aura
click 110 217 wallpaper
click 110 281 scenario
click 110 317 macros
click 110 353 gamevisual
click 110 417 content
click 110 881 featurelib
click 110 915 usercenter
click 110 951 settings
click 110 81 home

# Devices sub-tabs
click 110 117 devices
click 300 88 dev_sysconfig
click 425 88 dev_freememory
click 535 88 dev_gpu
click 615 88 dev_lighting
click 675 88 dev_audio
click 770 88 dev_resource

# Aura Sync tabs
click 110 181 aura
click 280 88 aura_sync
click 350 88 aura_effects
click 455 88 aura_ai

# Settings tabs + themes
click 110 951 settings
click 350 88 settings_update
click 440 88 settings_about
click 250 88 settings_general
click 545 280 theme_rog
sleep 0.3
if [ -f "$HOME/.config/tuf-crate/ui.json" ] && grep -q '"Rog"' "$HOME/.config/tuf-crate/ui.json"; then
  echo "PASS: theme persisted (ROG)"
else
  echo "FAIL: theme not persisted"; fail=1
fi
click 330 280 theme_tuf
click 110 951 settings
click 1150 280 theme_light
sleep 0.3
if [ -f "$HOME/.config/tuf-crate/ui.json" ] && grep -q '"Light"' "$HOME/.config/tuf-crate/ui.json"; then
  echo "PASS: light theme applied/persisted"
else
  echo "FAIL: light theme not persisted"; fail=1
fi
click 330 280 theme_tuf

# RGB write interaction (one static apply) — navigate back to Aura Effects first
before=$(grep -c '"attr":"kbd_rgb"' /var/lib/tuf-crate/audit.jsonl 2>/dev/null || echo 0)
click 110 181 aura_for_effect
click 350 88 aura_effects_for_click
click 280 330 effect_static
sleep 0.6
after=$(grep -c '"attr":"kbd_rgb"' /var/lib/tuf-crate/audit.jsonl 2>/dev/null || echo 0)
if [ "$after" -gt "$before" ]; then echo "PASS: effect click produced audited RGB write ($before -> $after)"; else echo "FAIL: no audited RGB write"; fail=1; fi

# Theme accents (state only) and responsive window sizes
xdotool windowsize "$(xdotool search --name tuf-crate | head -1)" 1600 900
sleep 0.5; shot resized_1600x900; alive resize_1600 || true
xdotool windowsize "$(xdotool search --name tuf-crate | head -1)" 1366 768
sleep 0.5; shot resized_1366x768; alive resize_1366 || true

kill "$APID" 2>/dev/null; kill "$XPID" 2>/dev/null

# Splash animation frames (separate run)
Xvfb :99 -screen 0 1920x1080x24 >/tmp/xvfb-ui.log 2>&1 &
XPID=$!
sleep 1.2
HOME=/home/nixon /home/nixon/.local/bin/tuf-crate >/tmp/gui-splash.log 2>&1 &
APID=$!
sleep 0.45; import -window root "$OUT/splash_a.png"
sleep 0.55; import -window root "$OUT/splash_b.png"
sleep 0.55; import -window root "$OUT/splash_c.png"
kill "$APID" 2>/dev/null; kill "$XPID" 2>/dev/null
d1=$(compare -metric RMSE "$OUT/splash_a.png" "$OUT/splash_b.png" null: 2>&1 | sed -E 's/.*\(([0-9.]+)\).*/\1/')
d2=$(compare -metric RMSE "$OUT/splash_b.png" "$OUT/splash_c.png" null: 2>&1 | sed -E 's/.*\(([0-9.]+)\).*/\1/')
echo "splash frame deltas: $d1 / $d2"

echo "interaction test complete (failures=$fail), screenshots in $OUT"
exit "$fail"
