#!/usr/bin/env bash
# READ-ONLY Embedded Controller dump for RGB reverse engineering.
# Loads the in-tree ec_sys module WITHOUT write_support — this cannot write
# anything, to the EC or anywhere else. Output: 256-byte EC image + checksum.
set -euo pipefail
[ "$(id -u)" = 0 ] || { echo "run with sudo"; exit 1; }
modprobe ec_sys 2>/dev/null || true
[ -r /sys/kernel/debug/ec/ec0/io ] || { echo "ec_sys unavailable on this kernel"; exit 1; }
OUT="${1:-/tmp/ec-dump-$(date +%Y%m%d-%H%M%S).bin}"
cp /sys/kernel/debug/ec/ec0/io "$OUT"
echo "EC dump saved: $OUT"
sha256sum "$OUT"
