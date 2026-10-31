#!/usr/bin/env bash
# Snapshot the current values of the five verified attributes.
# Read-only. Root needed for some reads; falls back gracefully.
set -uo pipefail
OUT="tuf-crate-config-backup-$(date +%Y%m%d-%H%M%S).json"
val() { cat "$1" 2>/dev/null | tr -d ' \n' || echo null; }
{
  echo "{"
  echo "  \"captured\": \"$(date -Is)\","
  echo "  \"platform_profile\": \"$(val /sys/firmware/acpi/platform_profile)\","
  echo "  \"charge_control_end_threshold\": $(val /sys/class/power_supply/BAT1/charge_control_end_threshold || echo null),"
  echo "  \"kbd_backlight\": $(val '/sys/class/leds/asus::kbd_backlight/brightness' || echo null),"
  echo "  \"panel_od\": $(val /sys/devices/platform/asus-nb-wmi/panel_od || echo null),"
  echo "  \"dgpu_disable\": $(val /sys/devices/platform/asus-nb-wmi/dgpu_disable || echo null)"
  echo "}"
} > "$OUT"
cat "$OUT"
echo "backup written: $OUT"
