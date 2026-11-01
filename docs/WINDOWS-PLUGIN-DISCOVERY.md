# Windows-side discovery: AsIO3 gateway DLL + lighting architecture (2026-11-01)

Follow-up to `PHASE2.5-SESSION-LOG.md`. After the conclusive user-mode capture failure, the
Procmon + static-analysis pass identified the real command chain and a fresh, fully
statically-analyzable attack surface. All artifacts are committed under
`research/windows-plugins/`.

## 1. Procmon is structurally blind to the RGB traffic (proven)

A 10-minute system-wide Procmon capture (3.1 GB, 6,669,901 events, taken while the owner
performed the full colour sequence) contains **15,069 DeviceIoControl events total — none
from any ASUS process, none on any AsIO device**. Reason: Procmon instruments the
file-system/registry/network stacks only; DeviceIoControl to raw devices (\\.\AsIO3, ACPI,
etc.) never traverses them. The IRP-level traffic of non-filesystem devices is invisible to
Procmon. (Parsed with `procmon-parser` via the offsets API; full log: `procmon-capture-summary.txt`.)

Combined with the user-mode result (direct syscalls bypass ntdll/kernel32 hooks), **no
passive observation tool that runs in user mode — or a filesystem filter — can see the RGB
writes.** Only kernel-debugger-level observation (WinDbg on AsIO3.sys) or driver
reconstruction from static analysis remain.

## 2. The real AsIO3 client chain (identified)

```
Armoury Crate UI (UWP)
  └─ ArmouryCrate.Service.exe            [anti-debug: self-exits on debugger attach]
       └─ SystemDevicePlugin\AsIO3_64.dll   [WINIO-STYLE RAW HARDWARE GATEWAY — plain on disk!]
            └─ \\.\AsIO3 IOCTLs (own code family, e.g. 0x22BB07 workhorse, 0xA040xxxx set)
                 └─ AsIO3.sys (Asusgio3)  → raw EC port I / indexed I / physmem in kernel
```

- `AsIO3_64.dll` (503 KB, x64 native, **not** WOF-compressed — lives in
  `C:\Program Files\ASUS\Armoury Crate Service\SystemDevicePlugin\`) exports the classic
  WinIO primitive set: `ASIO_InPortB/OutPortB`, `SetPortVal`, `GetPortVal`,
  `Read/Write_IndexIo_Byte` (the EC *indexed* register space — outside ACPI EC RAM, exactly
  where the lighting bytes were proven to live), `MapPhysToLin`, `Read/Write_PCI_*`,
  `Read/Write_MSR`, `InstallNTDriver/StartNTDriver` (it can install its own driver build),
  `LoadATSZIoDriver/CloseATSZIoDriver`.
- **55 DeviceIoControl call sites**, one per primitive (full site→function map produced).
  Two `0x22BB07` constants inside internal functions; a family of `0xA040xxxx` control codes.
- This explains every observation at once: the Service is anti-debug-protected; its gateway
  DLL uses raw-device IOCTLs (invisible to Procmon) and the WinIO primitives issue single
  device-open + primitive calls (no ntdll I/O traffic pattern to hook per operation).
- `CPUControlLibrary.dll` P/Invokes `AsIO3_64` (UTF-16 DllImport); `FanControlLibrary.dll`
  uses its own path. `Load.conf` loads `ArmouryCrate.SystemDevicePlugin.dll`.

## 3. Lighting architecture (from AuraPlugin configs)

`AuraPlugin\Aura.ini` / `DetachableKBEC.ini` / `Basic.ini` document the model: keyboard
lighting = **firmware effect indices + parameters** ("Static - FW effect index 0",
`Effect1: ColorR/G/B, Speed, Direction, ...`, `ECApply=3/4`). The EC lighting controller
renders effects itself — consistent with RESEARCH §13-14 (EC refuses direct RAM writes; the
controller owns the block). AuraPlugin (5.4 MB .NET, committed) is a coordinator: hidden
window IPC ("ASUS ROG Aura Listener Sync Proc Invisible Window") to
`LightingService.exe`; the privileged EC write happens in the DriverStore hardware service
(ASUSOptimization) — still WOF-compressed.

## 4. Next-session plan (static analysis only, no runtime tracing needed)

1. Disassemble `AsIO3_64.dll`'s 55 DeviceIoControl sites → full primitive→IOCTL→buffer-format
   map (small, unprotected DLL; offline work, ideal for the Linux side).
2. Correlate `Write_IndexIo_Byte` / `SetPortVal` call sequences with the EC indexed space to
   locate the effect-apply protocol (the caller side: WOF-decompress ASUSOptimization, or
   decompile the .NET plugins).
3. If the sequence is recovered, it still must pass `docs/SAFETY.md` before any Linux
   implementation is considered (hard allowlist, kernel EC serialization, read-back
   verification — as designed in `kernel/tuf_rgb.c`).

## Artifacts (research/windows-plugins/)

| File | Size | What |
|---|---|---|
| `AsIO3_64.dll` | 503,192 B | the gateway DLL (SHA-256 08846424…713370) |
| `ArmouryCrate.SystemDevicePlugin.dll` | 138,144 B | .NET plugin host |
| `FanControlLibrary.dll` | 68,000 B | fan control library |
| `ArmouryCrate.AuraPlugin.dll` | 5,424,544 B | lighting coordinator (.NET) |
| `Load.conf` | 35 B | plugin load list |
| `procmon-capture-summary.txt` | — | parsed capture summary |

## 5. Service image liberated + RGB channel verdicts (2026-11-01, later)

The ASUS hardware service was **decompressed from memory** (its on-disk DriverStore image is
WOF-compressed; a passive VirtualQueryEx walk of the running process captured the loaded image
without any thread suspension — `AsusOptimization-memimage.bin`, SHA-256 f3d76563…f0fc4d, PE
+ .pdata intact, 2055 functions).

Definitive RGB channel verdicts on FX706HF.312:

| Channel | Verdict | Evidence |
|---|---|---|
| WMI one-zone RGB (DEVID 0x00100066) | **CLOSED** | DSDT static (WMNB returns 0xFFFFFFFE), kernel DEVS live test, and live Windows ATK `DSTS(0x00100066)` → 0xFFFFFFFE. ASUSOptimization's own support-check code was located (lea/mov 0x00100066 + `mov ecx, 0x53545344` DSTS + result bit 0x10 test) — on this model it fails there too |
| USB HID RGB (0x0B05/0x8854) | **N/A on this model** | ASUSOptimization supports it, but no VID_0B05 USB device exists on FX706HF (internal keyboard is EC-attached) |
| EC via AsIO/AsusSAIO driver | **the actual path** | ASUSOptimization contains its own EC command engine (`SaveECCommandError`, `ECCommandError debug`, `ServiceLoopThread, KEYBDLIGHTEX`, `Set/Get/CheckKeyboardLightStatus`) with keyboard-light handlers; its 6 DeviceIoControl sites build IOCTLs at runtime (CTL_CODE constructed — no constants to grep) |

Windows ATK note: `DSTS(0x00050011)` — a DEVID known-good on Linux — also returns 0xFFFFFFFE
through `AsusAtkWmi_WMNB`, so the Windows ATK WMI wrapper is not a faithful firmware passthrough
(likely needs the WMNB INIT handshake). Linux-side `asus_wmi_evaluate_method` remains the
authoritative test channel.

### Next session (single focused task)

Disassemble `AsusOptimization-memimage.bin`'s EC command engine (functions around the six
DeviceIoControl sites 0x2e51/0xfd40/0x4f962/0x6d30/0xf9fd; keyboard-light functions reachable
from the `KEYBDLIGHTEX` service-loop handler) to extract: (a) the driver device it opens
(AsIO3 / AsusSAIO) and the runtime-built IOCTL codes, (b) the EC command sequences for the
keyboard-lighting effect-apply (mode/colour/speed register writes). The outputs feed
`kernel/tuf_rgb.c` (already allowlist-designed) after the SAFETY.md checklist.

## 6. PROTOCOL RECOVERED: WMNB DEVID 0x00120057 "TUF_RGB_CMD" (2026-11-01, disassembly)

The KEYBDLIGHTEX handler (func 0x27f9b) in the liberated ASUSOptimization image was traced to
the end of the chain:

```
service loop (KEYBDLIGHTEX feature)
  └─ handler 0x27f9b: dispatches keyboard-light sub-features
       ├─ registry state writes (0x449b0 → RegSetValueExW, single-letter value names A/F/W/S/D/Q/P)
       ├─ worker threads via 0x7330(params {cmd, sub}) — observed pairs:
       │     (0x50,4) (0x100,4) (0x80,5) (0x180,5) (0x100,6)
       └─ thread proc 0x73c0:
            if cmd ∈ {0x80, 0x180} →  wrapper 0x6fa0(DEVID=0x00120057, value=cmd)
            else                   →  sub-code → service global, same wrapper
                 wrapper 0x6fa0:  builds WMNB request {IIA0=DEVID, IIA1=value}, magic='DEVS',
                                  8-byte in / 4-byte out → 0x6d30 → DeviceIoControl(\.\ATKACPI\ASLDSRV)
```

**The RGB command channel is WMNB 'DEVS' with DEVID 0x00120057 — the same WMNB interface the
in-kernel asus-wmi driver uses.** 0x00120057 is one of the "unnamed 0x001200xx" firmware-private
DEVIDs from RESEARCH §3 — now identified as TUF_RGB_CMD. Every earlier RGB attempt (G-Helper's
0x00100056/5A protocol, the one-zone 0x00100066) knocked on different doors.

Windows ATK note: the `AsusAtkWmi_WMNB.DSTS` wrapper rejects 0x00120057 with "Generic failure"
(its own DEVID whitelist — same wrapper mangles the known-good 0x00050011), so the authoritative
test is the Linux kernel interface. Probe module committed: `kernel/tuf_rgb_probe.c`
(read-only DSTS by default; DEVS apply-code gated behind `probe_devs=1`).

Remaining RE (colour encoding): the handler's 15 registry writes + the worker-thread parameter
pairs cover mode/apply; the colour-byte encoding is one hop further (the wrapper passes only a
u32 value — the colour/mode/speed packing into that u32 follows from the handler's buffer
construction around the thread-spawn sites).
