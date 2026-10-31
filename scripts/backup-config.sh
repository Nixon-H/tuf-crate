#!/usr/bin/env bash
# Snapshot the current values of the five verified attributes. Read-only.
# JSON built via python3 so missing/empty/blank values become null correctly.
set -uo pipefail
OUT="tuf-crate-config-backup-$(date +%Y%m%d-%H%M%S).json"
python3 - "$OUT" <<'PYEOF'
import json, sys, pathlib

def read(path):
    p = pathlib.Path(path)
    try:
        v = p.read_text().strip()
        return v if v else None
    except OSError:
        return None

doc = {
    "captured": __import__("datetime").datetime.now().isoformat(timespec="seconds"),
    "platform_profile": read("/sys/firmware/acpi/platform_profile"),
    "charge_control_end_threshold": read("/sys/class/power_supply/BAT1/charge_control_end_threshold"),
    "kbd_backlight": read("/sys/class/leds/asus::kbd_backlight/brightness"),
    "panel_od": read("/sys/devices/platform/asus-nb-wmi/panel_od"),
    "dgpu_disable": read("/sys/devices/platform/asus-nb-wmi/dgpu_disable"),
}
numeric = ("charge_control_end_threshold", "kbd_backlight", "panel_od", "dgpu_disable")
for k in numeric:
    if doc[k] is not None:
        try:
            doc[k] = int(doc[k])
        except ValueError:
            pass
pathlib.Path(sys.argv[1]).write_text(json.dumps(doc, indent=2) + "\n")
print(json.dumps(doc, indent=2))
PYEOF
echo "backup written: $OUT"
