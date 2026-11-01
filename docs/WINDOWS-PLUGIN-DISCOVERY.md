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

> **CORRECTION (same day, final): this identification was wrong.** `0x00120057` is
> `ASUS_WMI_DEVID_RSOC` (battery relative state of charge), not RGB. The disassembled
> KEYBDLIGHTEX worker was the battery/charge-limit path; its command codes and sub-codes do
> not carry colour. The actual keyboard-RGB channel is the upstream **`0x00100056`** with
> the kernel's `bios_args` layout (`arg0 = DEVID`, `DEVS` passed as the WMI method id).
> With that layout the firmware accepts the colour and the GUI/daemon now work. See
> `docs/RESEARCH.md` §16 and `docs/SAFETY.md` §1d. The text below is kept as the historical
> reasoning trail.

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

## 7. Live probe results — DEVS 0x00120057 state semantics (2026-11-01)

`kernel/tuf_rgb_probe.c` (sysfs `/sys/devices/platform/tuf-rgb-probe/{value,dsts}`) was used
for supervised empirical mapping through the kernel WMI channel:

| DEVS value | DSTS after | Interpretation |
|---|---|---|
| — (initial) | 0x00890000 | base state |
| 0x50 | 0x00890000 | bit 0x80 clear |
| 0x100 | 0x00890000 | bit 0x80 clear |
| 0x180 | 0x00890800 | **bit 0x80 → status bit 0x0800** |
| 0x80 | 0x00890800 | **bit 0x80 → status bit 0x0800** |
| 0x00FF0000 / 0x000000FF / 0x00FF0080 | 0x00890800 (unchanged) | accepted (retval=1) but no state change — not the colour carrier |

**Confirmed:** value bit 7 (`0x80`) is an enable/apply flag reflected as status bit 11.
Colour candidates in byte positions 0/1/2 were accepted but did not alter the state — the
colour must ride the handler's *sub-code* path (observed service pairs {cmd, sub}: the sub
is stored to a service global, not passed to WMI in the same call).

**Component split discovered:** `ArmouryCrate.AuraPlugin.dll` references the G-Helper-style
DEVIDs `0x00100056`/`0x0010005A` (the paths this firmware rejects), while the live keyboard
path (`0x00120057`) lives in **ASUSOptimization's KEYBDLIGHTEX handler**. The colour u32
packing therefore follows from that handler's buffer construction (worker proc 0x73c0 →
wrapper 0x6fa0) — the remaining disassembly hop, now with the full sysfs probe available to
verify any candidate encoding in one keystroke.

## 8. Handler disassembly — colour is NOT in the worker command values (2026-11-01)

Disassembly of the KEYBDLIGHTEX thread proc (memimage 0x73C0) settles the worker path:

```
0x73D2: mov r8d, [rcx]        ; r8d = cmd (becomes the WMI value)
0x73D5: lea eax,[r8-0x80]     ; cmd ∈ {0x80, 0x180} ?
0x73EB: mov eax,[rcx+4]       ; sub
0x73EE: mov [global],eax      ; sub → service global (NOT into WMI)
0x73FD/0x747A: mov edx,0x120057 ; DEVID
0x7402/0x747F: call wrapper   ; value = cmd only
```

**The worker DEVS calls carry only the command code (0x50/0x80/0x100/0x180) — no colour.**
Empirical writes of colour candidates (bits 0-7/8-15/16-23/24-31) were accepted (retval=1)
but inert, consistent with this.

**All eight wrapper call sites and their DEVIDs:**
| Site | DEVID | Value pattern |
|---|---|---|
| 0x7402 / 0x747F | 0x00120057 | cmd codes 0x50/0x80/0x100/0x180 |
| 0x51904 | **0x00120080** | 1, 0xB, 0xC (mode commands; `cmp edi,1 / [edi-0xb]≤1`) |
| 0x502AB | **0x00120070** | persisted setting, DSTS flag bit 0x10000 gates the set |
| 0x515DE | **0x0012008F** | value context not yet resolved (code region needs realignment) |
| 0x51662 | computed | r8d=0x1e |
| 0x431E6 | 0x00040018 | r8d=ebx |

The colour bytes are therefore written by the **handler itself** (registry-state path,
0x449B0 RegSetValueExW with single-letter names A/F/W/S/D/Q/P) — the remaining hop is
tracing how those registry values reach a DEVS value or a separate lighting DEVID.
The sysfs probe (devid+value) is ready to verify each candidate instantly once identified.

**Probe infrastructure note:** after several load/unload cycles the leaked platform device
(the module intentionally keeps the probe device registered) can prevent the driver from
rebinding its sysfs attrs; a fresh boot or a unique device name per build resolves it.
