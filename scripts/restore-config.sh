#!/usr/bin/env bash
# Restore attribute values from a backup created by backup-config.sh.
# Values are validated against the SAME safe ranges the GUI/daemon enforce
# (see crates/tuf-core clamp()) before any sysfs write; root required.
# The live dGPU write is intentionally refused: on this EC it can freeze the
# compositor — schedule Eco from the GUI (applied at next boot) instead.
set -euo pipefail
[ $# -eq 1 ] || { echo "usage: restore-config.sh <backup.json>"; exit 1; }
[ "$(id -u)" -eq 0 ] || { echo "run with sudo"; exit 1; }
jget() { python3 - "$1" "$2" <<'EOF'
import json, sys
d = json.load(open(sys.argv[1]))
v = d.get(sys.argv[2], "")
print("" if v is None else v)
EOF
}
err() { echo "restore: refusing invalid value for $1: '$2'" >&2; exit 1; }
f="$1"

p=$(jget "$f" platform_profile)
if [ -n "$p" ]; then
  case "$p" in
    quiet|balanced|performance) ;;
    *) err "platform_profile" "$p";;
  esac
  echo "$p" > /sys/firmware/acpi/platform_profile && echo "profile ← $p"
fi

c=$(jget "$f" charge_control_end_threshold)
if [ -n "$c" ]; then
  case "$c" in
    ''|*[!0-9]*) err "charge_control_end_threshold" "$c";;
  esac
  [ "$c" -ge 20 ] && [ "$c" -le 100 ] || err "charge_control_end_threshold" "$c"
  echo "$c" > /sys/class/power_supply/BAT1/charge_control_end_threshold && echo "charge limit ← $c"
fi

k=$(jget "$f" kbd_backlight)
if [ -n "$k" ]; then
  case "$k" in
    ''|*[!0-9]*) err "kbd_backlight" "$k";;
  esac
  [ "$k" -ge 0 ] && [ "$k" -le 3 ] || err "kbd_backlight" "$k"
  echo "$k" > '/sys/class/leds/asus::kbd_backlight/brightness' && echo "brightness ← $k"
fi

o=$(jget "$f" panel_od)
if [ -n "$o" ]; then
  case "$o" in
    0|1) ;;
    *) err "panel_od" "$o";;
  esac
  echo "$o" > /sys/devices/platform/asus-nb-wmi/panel_od && echo "panel_od ← $o"
fi

d=$(jget "$f" dgpu_disable)
if [ -n "$d" ]; then
  echo "restore: skipping dgpu_disable='$d' — live dGPU writes can freeze this"
  echo "  compositor. Use the GUI (GPU Performance → Schedule Eco for next boot),"
  echo "  which applies it safely at the next boot."
fi

echo "restore complete."
