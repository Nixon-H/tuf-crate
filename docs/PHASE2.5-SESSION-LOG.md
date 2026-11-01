# Phase 2.5 — Windows capture session log (2026-10-31/11-01)

**Machine:** the target FX706HF booted to Windows 11 (BIOS FX706HF.312), Armoury Crate 6.7.6.0,
AsIO3 driver (Asusgio3) running. All tracer hashes from the procedure verified before use
(`tuf_trace.exe` ea60346b…56c, `run-capture.cmd` 7ac38d2a…c7a). Supervised, observation-only;
no request was ever constructed or modified; every detach went through the safe path.

## Outcome

The original `windows-tracer/tuf_trace.c` was never executed before this session. Running it
surfaced **five independent defects** — any one of which alone produced an empty trace. All five
are fixed in the validated C# tracers (`windows-tracer/session-2026-11-01/`), and the corrected
pipeline is proven end-to-end on a live WOW64 process (`test-dr.ps1`: DR hardware breakpoints
captured real AFD socket IOCTLs, e.g. ioctl 0x12047, with correct stack-arg decoding and CSV
output). The final capture round against `asus_framework.exe` awaits one reboot (see Next steps).

## Defects found in the original tracer (all fixed)

| # | Defect | Effect | Fix |
|---|---|---|---|
| 1 | `GetProcAddress(ntdll, "DeviceIoControl")` — **DeviceIoControl is exported by kernel32, not ntdll** | NULL returned; the computed hook address was kernel-space garbage (`ffff8005d19d0000`); the primary breakpoint was **never armed** (in both shipped captures) | resolve from kernel32 (and keep ntdll!Nt/ZwDeviceIoControlFile) |
| 2 | `NtDeviceIoControlFile` args read from wrong x64 slots (ioctl=r10, inbuf=r8) | even a successful Nt hook would log garbage | 6th arg ioctl @rsp+0x30, 7th inbuf @rsp+0x38, 8th inlen @rsp+0x40 |
| 3 | 32-bit export resolution (added in trace3) used **wrong IMAGE_EXPORT_DIRECTORY offsets** (nFuncs@16 is the ordinal Base!) | 0 hooks on the 32-bit targets | nFuncs@20, nNames@24, pFuncs@28, pNames@32, pOrdinals@36 |
| 4 | `DEBUG_EVENT` union treated as starting at offset 12 — **on x64 the union is 8-aligned at offset 16** (4-byte pad after dwThreadId) | every exception decode read garbage (code=0): int3 hits looked like "foreign breakpoints" → `DBG_CONTINUE` resumed *into the int3* → infinite breakpoint loop | explicit pad field; SizeConst 168 |
| 5 | WOW64 breakpoint hits arrive as **STATUS_WX86_BREAKPOINT (0x4000001E)**, not 0x80000003/4 | DR/int3 hits in 32-bit processes never matched the handlers | accept 0x4000001E alongside the x64 codes |

## Key discoveries about the target system

- **`asus_framework.exe` is a 32-bit (WOW64) process** — confirmed by `IsWow64Process` and by the
  x86 ntdll base 0x773b0000 appearing in the old tracer's own log. The statically-confirmed AsIO3
  client (Phase 2.4) therefore requires 32-bit export-table parsing (in-memory, since a 64-bit
  process cannot `GetProcAddress` into 32-bit modules) and `Wow64Get/SetThreadContext` +
  x86 stdcall stack args (DeviceIoControl: [esp+8/0xC/0x10]; NtDeviceIoControlFile:
  [esp+0x18/0x1C/0x20]).
- **Armoury Crate's own components self-exit on debugger attach** (clean `TARGET EXITED`, exit code
  0): ArmouryCrate (UWP), ArmouryCrate.Service, ArmourySocketServer, ArmouryHtmlDebugServer,
  ArmouryCrate.UserSessionHelper, AuraWallpaperService, ROGLiveService. They must NOT be re-attached
  (they stay broken until their service chain restarts — i.e. until reboot).
- The DriverStore sci2 services tolerate user-mode debugging fine: ASUSOptimization,
  ASUSOptimizationStartupTask, ArmouryCrateControlInterface, AsusAppService, AsusSwitch,
  AsusSystemAnalysis, AsusSystemDiagnosis, AsusCertService, GameSDK (this one also WOW64).
- The x86 `kernel32!DeviceIoControl` hot-patch prologue (`8B FF 55 8B EC`) resolves identically in
  every WOW64 target this boot (kernel32 0x75540000, ntdll 0x773b0000) — offset math within the
  same boot session is safe.

## Capture rounds

1. **Round 1** — `tuf_trace.exe` (original C tool) on asus_framework main PID 16376, full 3-minute
   capture during lighting operations: clean attach/detach, **0 records** (defects 1+2).
2. **Round 2** — multi-process probe (`probe-all.ps1`): all ASUS processes are `OpenProcess`-able
   when elevated; tracer runs on 15 processes; the 7 Armoury components anti-debug-exit; survivors
   logged 0 records during lighting operations (defects 1+2 again).
3. **Round 3** — corrected x64 C# tracer (`trace2.ps1`) + WOW64 support (`trace3.ps1`): 16 x64
   targets hooked and verified armed; asus_framework discovered 32-bit (skipped); survivors again
   0 records — even `all`-mode ASUSOptimization produces **zero** DeviceIoControl calls through
   ntdll/kernel32 entry points (its hardware access is via WMI/ALPC in other processes, or direct
   syscalls).
4. **Round 4** — `trace3` with 32-bit hooks armed on all three asus_framework instances:
   correctly-armed int3 (`x86 kernel32!DeviceIoControl @ 7555c870`, orig 8B) — processes
   self-terminated ~9 s later. Root cause identified post-mortem as **defects 4+5**: the handler
   never recognized the int3 it had armed, resumed into it, and the resulting breakpoint storm
   killed the processes (not ASUS anti-tamper).
5. **Validation** — `test-dr.ps1`: debug-register (DR0/DR1/DR2) hooks on a live 32-bit PowerShell
   captured real AFD socket IOCTLs. DR hits decode as `code=0x4000001e addr=0x77429080 first=1`;
   stack args correct; CSV rows correct. **The DR path writes zero bytes into the target** — no
   int3, nothing for code-integrity scans to find — and needs no step-over (the trap is a fault
   before the instruction; `DBG_CONTINUE` executes it and the DR re-arms itself).

## Next steps

1. **Reboot Windows** (restores the Armoury service chain and asus_framework, killed in round 4).
2. Run `trace4.ps1` from an elevated console: DR hooks (no code writes) on asus_framework +
   corrected int3 `all`-mode on the three x64 services.
3. Perform the 7 lighting operations (Static R→G→B, brightness, Breathing, restore).
4. Stop via `stop.flag`; run the Linux-side `decode_trace.py` on the per-target CSVs; correlate the
   AsIO3-family buffers (0x22B80A/0x22BB0A write format from AsIO3.sys disassembly) with the
   colour changes to recover the RGB byte sequence.

## Artifacts in this directory

- `trace2.ps1` — corrected x64 tracer (kernel32/ntdll hooks, fixed Nt stack args, fixed
  DEBUG_EVENT decoding, multi-target, flag-file stop).
- `trace3.ps1` — + in-memory 32-bit PE export resolution and WOW64 context/stack handling.
- `trace4.ps1` — + DR hardware-breakpoint mode for WOW64 targets (zero code writes), WX86 code
  handling, new-thread DR propagation, DR cleanup on detach.
- `test-dr.ps1` — self-contained validation harness (spawn 32-bit PowerShell → DR hooks → UDP
  traffic → assert captured AFD IOCTLs).
