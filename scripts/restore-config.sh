#!/usr/bin/env bash
# Restore attribute values from a backup created by backup-config.sh.
# Writes go through the same kernel interfaces tuf-crate uses; root required.
set -euo pipefail
[ $# -eq 1 ] || { echo "usage: restore-config.sh <backup.json>"; exit 1; }
[ "$(id -u)" -eq 0 ] || { echo "run with sudo"; exit 1; }
jget() { python3 - "$1" "$2" <<'EOF'
import json, sys
d = json.load(open(sys.argv[1]))
print(d.get(sys.argv[2], ""))
EOF
}
f="$1"
p=$(jget "$f" platform_profile);        [ -n "$p" ] && echo "$p" > /sys/firmware/acpi/platform_profile && echo "profile ← $p"
c=$(jget "$f" charge_control_end_threshold); [ -n "$c" ] && echo "$c" > /sys/class/power_supply/BAT1/charge_control_end_threshold && echo "charge limit ← $c"
k=$(jget "$f" kbd_backlight);           [ -n "$k" ] && echo "$k" > '/sys/class/leds/asus::kbd_backlight/brightness' && echo "brightness ← $k"
o=$(jget "$f" panel_od);                [ -n "$o" ] && echo "$o" > /sys/devices/platform/asus-nb-wmi/panel_od && echo "panel_od ← $o"
d=$(jget "$f" dgpu_disable);            [ -n "$d" ] && echo "$d" > /sys/devices/platform/asus-nb-wmi/dgpu_disable && echo "dgpu_disable ← $d"
echo "restore complete."
