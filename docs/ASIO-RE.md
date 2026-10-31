# Phase 2 — AsIO.sys reverse engineering: interim report

**Status: investigation in progress** · read-only static analysis of files extracted from the
Windows partition · nothing executed or installed from the Windows side · no hardware writes.

## 1. Components located and extracted (`research/windows-drivers/`)

| File | SHA-256 (first 16) | Notes |
|---|---|---|
| `Windows/System32/drivers/AsIO3.sys` | `8accede04c953642` | 54,408 B, x64 native, PDB `AsIO3_64.sys.pdb` (Jenkins build) |
| `LightingService/LightingService.exe` | — | 4.9 MB, the Aura lighting engine |
| `LightingService/LastProfile.xml` | `9562f50e1fc8ee15` | persisted Aura profile state (full device taxonomy, `singlecolour=1`) |

Also present on the partition (not yet extracted): `asus_framework.exe` (ArmouryDevice — the
System Control Interface framework), `LM_Support.exe` + `AuraLayerManager.dll` (AURA add-on).

**Absent on this machine**: AsIO.sys, AsIO2.sys, AsCom.sys, AsusSysIO.sys — this generation
ships **AsIO3 only**.

## 2. Driver analysis (AsIO3.sys)

- Imports: `MmMapIoSpace`, `MmGetPhysicalAddress`, `ZwOpenSection`/`ZwMapViewOfSection`
  (physical-memory section mapping), `ExAcquireFastMutex`, `IoIs32bitProcess` (WoW64 support),
  `ZwSetSecurityObject`.
- **Design confirmed: generic privileged I/O gateway.** Disassembly of `.text` shows 16 `out`
  and 3 `in` instructions where **the port number arrives in DX from the caller's buffer** —
  no port constants in the driver. AsIO3 does not know the hardware; Armoury's user-mode
  services tell it which ports to hit.
- IOCTL family identified (`FILE_DEVICE_UNKNOWN 0x22`): `0x22B807`, `0x22B80A` (×5 sites —
  the read/write workhorses), `0x22BB07`, `0x22BB0A`, `0x22C115`, `0x226C15`, `0x22B015`.
- Complete IOCTL handler reconstruction pending (INIT-section linear disassembly desynced on
  relocations; needs pdata-guided function walking — tooling exists in the venv).

## 3. LightingService analysis

- **Zero references to AsIO** (no `\\.\AsIO3` device string, no AsIO IOCTL constants in the
  binary) — LightingService is the *effect renderer*, not the AsIO client.
- `LastProfile.xml` documents the full Aura device taxonomy (60+ device classes incl.
  `NB_Keyboard`) with `singlecolour=1` state — but no raw colour/command bytes.

## 4. Remaining AsIO callers (next extraction targets)

1. `asus_framework.exe` (ArmouryDevice — System Control Interface framework)
2. `LM_Support.exe` / `AuraLayerManager.dll` (AURA add-on)
3. The Armoury Crate service host (`ArmouryCrateService`) — path on the partition still to be
   enumerated.

The AsIO client that carries the EC port sequence will contain the port constants
(`0x62/0x66` EC command/data, or ITE indexed `0x4E/0x4F`/`0x6C/0x6D`) and the IOCTL
constants identified above — findable by the same byte-pattern scan used here.

## 5. Feasibility assessment (current)

- The protocol is **recoverable**: AsIO3 is a thin gateway; once the client-side sequence is
  found, the port/command/byte-level RGB protocol follows directly.
- A safe Linux implementation then requires a kernel driver that (a) uses the kernel ACPI EC
  locking for any EC access, (b) hard-allowlists the recovered byte sequence, (c) verifies
  read-back. The `tuf_ec`/`tuf_rgb` module skeletons in `kernel/` are the starting point.
- **Not yet established**: the actual command sequence (requires extracting and analysing the
  remaining service binaries), and whether the EC's lighting region is written through the
  ACPI-visible EC space or a banked/indexed space only AsIO reaches.
- Per policy: no hardware probing until the sequence is proven against the safety checklist
  (§8 of the phase-2 brief).

## 6. Decision point

The investigation is viable and partially complete. Continuing requires further offline
binary-analysis sessions (extraction + pattern analysis of `asus_framework.exe`,
`LM_Support.exe`, `AuraLayerManager.dll`, `ArmouryCrateService`). No hardware interaction is
required for that stage. The RGB verdict remains "firmware-blocked through safe channels"
until/unless the AsIO client-side protocol is fully recovered and passes the safety checklist.

## 7. Phase 2.1 — client hunt results (interim)

Extracted and scanned (ASCII + UTF-16 device strings, IOCTL dwords, EC port constants):

| Binary | Size | AsIO refs | Verdict |
|---|---|---|---|
| `LightingService.exe` | 4.9 MB | none | effect renderer — not the client |
| `AsusCCT.dll` (ASUSOptimization dir) | 363 KB | none for AsIO; IOCTL `0x22BB07` ×2 → **AsusColorKernel** (display colour) | display-colour path — dead end for keyboard |
| `ArmourySDK.dll` (ArmouryDevice/dll) | 337 KB | none | SDK layer without hardware access |

**DriverStore located**: `asussci2.inf_amd64_1c054a83d803f0c0` (+one older revision) — the
ASUS System Control Interface package containing the service components: ASUSOptimization,
ASUSSwitch, ASUSSystemAnalysis, ASUSSystemDiagnosis, AsusAppService, plus **AsusCCT.dll,
AsusColorKernel.dll, AsusHotkey.exe, AsusOSD.exe**.

**Key remaining candidate**: `ASUSOptimization/ASUSOptimization.exe` — the primary hardware
service. First extraction attempt produced 0 bytes (ntfscat timeout; large binary) — retry
with a 15-minute window is queued as the immediate next step. If it contains the AsIO client
(expected — it is the service that runs continuous hardware monitoring), the IOCTL buffer
formats follow from the same byte-pattern analysis, and the RGB protocol reconstruction
proceeds to the driver-handler correlation stage.

## 8. Phase 2.1 addendum — access breakthrough + extraction blockers

**FUSE-mount access fixed**: the earlier `ntfs-3g` hangs were a missing `/dev/fuse` device
node in the analysis container (`mknod /dev/fuse c 10 229` fixes it). Full read-only file
access to the Windows partition is now available without ntfscat.

**Extraction results this pass:**
- `ASUSOptimization.exe`: **unreadable through both access paths** — ntfs-3g lists it in the
  directory index but `stat` fails, and ntfscat (MFT-based) errors out. DriverStore binaries
  in this package are likely WOF/Xpress-compressed or have an attribute state that breaks
  ntfs-3g and ntfscat. Same failure for the stale `1c054a83` package (explains the earlier
  0-byte read; that MFT entry is stale).
- `ARMOURY CRATE One Package/ACFL20261031060333/`: only ACSetup installer stubs
  (ACSetup.exe, BIOSInfoLib, ProductInfoLib) — the setup payload, not the app.
- **Prime remaining lead**: `C:\Program Files\WindowsApps\` — Armoury Crate 6.7.6.0 is a
  packaged (UWP-style) app per its Update Center ("UWP App & Core Service"), so the actual
  app + services (and the AsIO client) live under WindowsApps, readable via the ro FUSE
  mount as root. Also: `ARMOURY CRATE Audio SDK`, `ASUS Hotplug Controller` (AsHotplugCtrl.exe),
  `ASUS Monitor Control`, `AURA lighting effect add-on x64` — all freshly enumerable.

**Next session plan (read-only, no execution):**
1. Enumerate `Program Files/WindowsApps/` for the ArmouryCrate package tree; extract
   `ArmouryCrateService`/lighting service binaries.
2. Scan the full set (ASCII+UTF-16 `AsIO` strings, IOCTL dwords 0x22B80x-0x22C115,
   port constants 0x62/0x66/0x6C/0x6D).
3. Correlate the found client with AsIO3's IOCTL handler disassembly (buffer formats).
4. Complete the RGB protocol reconstruction or document the precise remaining gap.

## 9. Phase 2.2 — WindowsApps package analysis (interim)

**Package inventory (`C:\Program Files\WindowsApps\`):**
- `B9ECED6F.ArmouryCrate_6.7.6.0_x64__qmba6cd70vzyy` (app binaries)
- `B9ECED6F.ArmouryCrate_6.7.6.0_neutral_~_qmba6cd70vzyy` (resources)
- `B9ECED6F.ASUSPCAssistant_4.0.49.0_*` (MyASUS)
- `ASUSAmbientHAL64_1.0.0.3_neutral__ecvjzhf6c4w7r`

**AppxManifest findings (x64 package):**
- Application entry: `ArmouryCrate.exe` (UWP frontend)
- **Full-trust process: `Win32Diagnostics\ArmouryCrate.Win32Manager.exe`** — the packaged
  app's privileged component (the only part that can open drivers)
- App services: `com.asus.armourycrate` (+ .inprocess, .toast), SSO provider
- Package subdirs include: ScenarioProfile, ThrottlePlugin, ThrottleMonitor.CommonControl,
  MonitorPlugin, Multiwheel, SystemFeature, UpdateCenter, Win32Diagnostics

**Binary triage:**
| Binary | Size | Type | AsIO evidence |
|---|---|---|---|
| ArmouryCrate.Win32Manager.exe | 100 KB | native (CLR: no) | **none found** — likely an RPC *client* staging the privileged ops |
| ArmouryCrateControlInterface.exe | 478 KB | native service | `CreateFileW` + **IOCTL `0x22BB07` ×1** (AsIO3 family) + RPCRT4/ControlService/named-pipe imports → **hardware-control broker service** |

**Architecture reconstruction (strongly supported hypothesis):**
UWP frontend → app-service/RPC → `ArmouryCrateControlInterface.exe` (native broker; opens
the device with CreateFileW, issues the AsIO3 IOCTL family) → `\\.\AsIO3` → EC.
The 0x22BB07 hit inside the broker is consistent with AsIO3's handler family but its device
path is not a literal string (dynamically built or via config) — the call-site disassembly
and the broker's device-path construction are the remaining RE targets.

**Next stage:** disassemble ArmouryCrateControlInterface.exe around the 0x22BB07 call site
(function walking via its .pdata), extract the exact CreateFileW path construction and the
DeviceIoControl buffer layouts, then correlate each IOCTL with the AsIO3.sys handler for the
complete user→kernel→EC RGB sequence. Win32Manager (100 KB) also pending full analysis.
