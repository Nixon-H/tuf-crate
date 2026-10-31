# Phase 2.5 — supervised Windows capture procedure

Tool: `windows-tracer/tuf_trace.exe` (source: `windows-tracer/tuf_trace.c` — review before
running; it only OBSERVES: debugger breakpoints on ntdll!DeviceIoControl /
NtDeviceIoControlFile, buffers dumped, execution continues unchanged).

## Windows session steps

1. Boot Windows. Copy `tuf_trace.exe` from the repo (`windows-tracer/`) to the Desktop.
2. Open an **elevated** cmd and find the framework service PID:
   ```
   tasklist /fi "imagename eq asus_framework.exe"
   ```
3. Start the tracer (elevated):
   ```
   tuf_trace.exe <PID> %USERPROFILE%\Desktop\trace
   ```
   It prints the target path and "tracing...". Leave this window open.
4. Open Armoury Crate and perform, pausing ~5 s between steps:
   1. Open the Lighting page.
   2. Set Static, pure RED (#FF0000).
   3. Set Static, pure GREEN (#00FF00).
   4. Set Static, pure BLUE (#0000FF).
   5. Change brightness.
   6. Select a different effect (e.g. Breathing).
   7. Return to the original colour.
5. In the tracer window press **Ctrl+C** — it detaches safely (breakpoints disarmed) and
   writes `trace.csv` + `trace.buffers.bin` next to it.
6. Reboot to Linux and copy both files into
   `~/Projects/tuf-crate/research/traces/`.

## Safety notes
- The tracer only reads debuggee memory at breakpoint hits; it never constructs requests.
- Ctrl+C is safe: hooks are disarmed before detach (the ASUS service keeps running).
- If Armoury Crate misbehaves: stop tracing, restart the ArmouryCrateControlInterface
  service (or reboot) — no persistence.

## Analysis (Linux side)
```
python3 windows-tracer/decode_trace.py research/traces/trace.csv
```
Groups by IOCTL, skips repeats, prints colour-change deltas — then the byte map is
correlated with AsIO3.sys handlers and the RGB write path decision is made.
