#!/usr/bin/env python3
"""Decode a tuf_trace CSV+buffers capture: group by IOCTL, diff colour changes."""
import csv, sys

csv_path = sys.argv[1] if len(sys.argv) > 1 else "trace.csv"
buf_path = csv_path.replace(".csv", ".buffers.bin")
buffers = open(buf_path, "rb").read() if len(sys.argv) > 1 else b""

rows = list(csv.DictReader(open(csv_path)))
FAMILY = {0x22B80A: "ASIO3-write", 0x22B807: "ASIO3-read", 0x22BB07: "ASIO3b-write",
          0x22BB0A: "ASIO3b-read", 0x22C115: "ASIO3-c", 0x226C15: "ASIO3-d", 0x22B015: "ASIO3-e"}

fam_rows = [r for r in rows if int(r["ioctl"], 16 if r["ioctl"].startswith("0x") else 10) in FAMILY]
print(f"total IOCTLs logged: {len(rows)} | AsIO3-family: {len(fam_rows)}")

prev = None
for r in fam_rows:
    off = int(r["buf_off"])
    inlen = int(r["in_len"])
    content = buffers[off:off+inlen].hex() if inlen else ""
    ioctl = int(r["ioctl"], 16 if r["ioctl"].startswith("0x") else 10)
    label = FAMILY.get(ioctl, hex(ioctl))
    if prev is not None and content == prev:
        continue  # skip consecutive identical
    print(f"ts={r['ts']} {label} handle={r['handle']} len={inlen} data={content}")
    prev = content
