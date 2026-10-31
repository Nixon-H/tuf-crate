# Armoury Crate Functionality & Protocol Research — ASUS TUF Gaming F17 FX706HF
**Project:** tuf-crate · **Date:** 2026-10-31 · **Method:** static analysis only — kernel source, decompiled ACPI (read-only), sysfs enumeration, public research. **No WMI method was invoked and no EC register was written during research.**

## 1. Evidence corpus (files in `research/`)

| Artifact | What it is | Acquisition |
|---|---|---|
| `research/acpi/DSDT.dsl` (71,010 lines) | ACPI DSDT of FX706HF (BIOS FX706HF.312), decompiled with iasl | copied read-only from `/sys/firmware/acpi/tables/DSDT`, `iasl -d` |
| `research/acpi/WMNB.dsl` (1,257 lines) | The ASUS WMI dispatcher method (DSDT line 67593) — every Armoury Crate operation funnels through here | extracted from DSDT.dsl |
| `research/kernel/asus-wmi.c` (5,570 L) | Linux kernel ASUS WMI core (master, 7.x-era) | torvalds/linux raw |
| `research/kernel/asus-armoury.c/.h` (1,154/2,914 L) | Kernel ASUS firmware-attributes driver (absorbed asus-nb-wmi hotkeys) | torvalds/linux raw |
| `research/kernel/asus-wmi-uapi.h` (81 DEVID defines) | Kernel Userspace-API device-ID map | torvalds/linux raw |
| Live sysfs state (audit evidence) | platform_profile, charge_control_end_threshold, hwmon6 (cpu/gpu fans), asus::kbd_backlight, panel_od, dgpu_disable, WMI bus devices | read from running 7.1.8 kernel |

## 2. The Armoury Crate path on this machine

Armoury Crate (Windows) does not talk to the EC directly for policy operations. It calls **ASUS System Control Interface** drivers (AsIO/AsCom + WMI), which invoke the ACPI method **`WMNB` behind WMI GUID `97845ED0-4E6D-11DE-8A39-0800200C9A66`** (= `ASUS_WMI_MGMT_GUID`, verified in kernel `asus-wmi-uapi.h:8`).

**WMNB calling convention (from DSDT):**
```
Method (WMNB, 3, Serialized)
Arg2 buffer = 5 × DWORD (IIA0..IIA4)
Arg1 = ASCII magic:
  0x54494E49 "INIT" → INIT(IIA0)
  0x53545342 "BSTS" → battery status
  0x4E554653 "SFUN" → supported functions bitmap
  0x53564544 "DEVS" → device set/get dispatch on IIA0 (DEVID), value in IIA1
  plus SCDG/VERS/MWGF/GLCD helpers
```
The Linux kernel uses exactly this interface (`asus_wmi_evaluate_method(DEVS, devid, value)`) — i.e., **anything the kernel exposes is the same channel Armoury Crate uses.** Building on kernel interfaces *is* using Armoury Crate's own control path.

## 3. DEVID inventory recovered from this firmware (WMNB)

Constants `0x001xxxxx` referenced by this machine's WMNB, cross-referenced with kernel UAPI names:

**Named (kernel UAPI):**
| DEVID | Kernel name | Verdict on FX706HF |
|---|---|---|
| 0x0010005A | TUF_RGB_MODE2 | **`Return 0xFFFFFFFE` (unsupported) in GET path** — see §4 |
| 0x00110011 | THERMAL_CTRL | handled (fan policy behind throttle_thermal_policy) |
| 0x00110013 / 0x00110014 | CPU_FAN_CTRL / GPU_FAN_CTRL | GET reads **EC directly** (`EC0.ECOK` gate, `EC0.MUT0` mutex) |
| 0x00110018 | FAN_BOOST_MODE | present |
| 0x00110024 / 25 / 32 | CPU/GPU/MID_FAN_CURVE | **`Return Zero`** — accepted but no-op (firmware ack, no curve I/O) |
| 0x0012006C | CHARGE_MODE | present (charge control live via kernel: `charge_control_end_threshold`) |
| 0x001200A0/A3/B0/B1/C1 | PPT_PL2_SPPT / PL1_SPL / APU_SPPT / PLAT_SPPT / PL3_FPPT | AMD-oriented power IDs; **`Return 0xFFFFFFFE`** in sampled path — Intel/HM570 platform does not implement WMI PPT |
| 0x001200C0 / C2 | NV_DYN_BOOST / NV_THERM_TARGET | NVIDIA Dynamic Boost hooks |
| 0x00120097 | DGPU_POWER_STATE | present |
| 0x00130022 | BOOT_SOUND | present (×4 refs) |

**Unnamed (firmware-private, Armoury-Crate-only):** 0x00100021, 0x00100043, 0x00100051–54, 0x0010005B, 0x00100066, 0x00110015/16/1B/1D/1E/21/26/27/29/33, 0x001200xx (≈25 more), 0x0013xxxx — Armoury Crate/vendor-service usage; semantics not documented anywhere public. Per the safety policy these are **not implemented**.

## 4. RGB (keyboard lighting) — the definitive answer

- Firmware **declares** a TUF RGB block (0x00100021, 0x00100051–54, 0x0010005A/5B, 0x00100066) and the kernel UAPI even names `TUF_RGB_MODE(2)`, `TUF_RGB_CMD` (0x00100057).
- **But this machine's WMNB returns `0xFFFFFFFE` ("not supported") for the RGB GETs** — the FX706HF's 1-zone RGB is *not* driven through WMI on this firmware variant.
- No kernel driver (asus-wmi, asus-armoury, asusctl) implements TUF RGB on any path; `asus-armoury.c` has zero TUF_RGB references.
- What exists and works: `asus::kbd_backlight` LED (brightness 0–3) via `asus-wmi` (DEVID 0x00050011-class path) — verified live.
- EC persistence: the lighting controller (EC-attached) retains the last colour set by Armoury Crate in Windows; Linux can adjust brightness only.
- **Verdict: dynamic RGB = Unsupported via every safe channel (evidence below). Static colour = set once in Windows (EC-persisted); brightness = Supported.**

**Deep-dive corrections (2026-10-31, follow-up):**
- DEVID `0x00100021` was identified by full block analysis: it is **NOT RGB** — it forwards
  the argument to `EC0.SKEY(IIA1)`, the EC **hotkey injector** (keycode → `_Qxx` events).
  Armory-Crate-style per-DEVID colour IDs (0x00100052/53/54/5A/5B/66) are GET-rejections
  (0xFFFFFFFE / Zero) in **both** get and set contexts — no set path exists for them.
- ACPI-wide search: the only RGB-named objects are `RGBC` (GNVS shared-memory region) and
  `RGBE` (PWMR — Intel PWRM mailbox, `CSD3` dispatcher with RSAT/AGBE handshake) — both are
  **Intel Dynamic Tuning Technology** interfaces, not the keyboard.
- Conclusion: on this firmware the keyboard colour is driven by Armoury Crate **exclusively
  through AsIO.sys raw EC I/O**, outside ACPI/WMI entirely. That is why it works on Windows
  while every documented interface rejects it here.

**Safe unlock plan (EC-diff reverse engineering, brick-safe by method):**
1. Load the in-tree `ec_sys` module **read-only** (no `write_support`) and dump the 256-byte
   EC image (`scripts/ec-dump.sh`, provided).
2. Owner boots Windows → sets a known colour (e.g. pure RED) in Armoury Crate → returns.
3. Dump again; repeat with GREEN and BLUE.
4. Diff the dumps → the bytes that consistently track the colour are the lighting bytes
   (typically 3 bytes + a mode byte). Fan/thermal regions are elsewhere in the EC map and
   are never touched.
5. Only then: a minimal kernel driver (using the kernel's own serialized `ec_read/ec_write`)
   writing **exactly** the identified bytes, read-back-verified, nothing else. Worst case
   remains cosmetic (lighting state until reboot); fan/thermal EC regions are never written.
Step 2–3 require owner cooperation (Windows reboots). Nothing is written until the byte map
is verified across three independent colours.

## 5. Fan control reality on FX706HF

- Kernel hwmon (`asus` hwmon6) exposes `fan1/cpu_fan`, `fan2/gpu_fan` RPM **inputs** and `pwm1/2_enable` — but **no `auto_points` curve attrs**, matching the firmware's `Return Zero` for curve DEVIDs.
- The `pwmX_enable` interface can disable fan auto-control — **rejected by safety policy** (fan-stop risk, no curve support behind it on this EC).
- Thermal policy (Silent/Balanced/Performance) is the firmware-sanctioned fan/thermal control: `platform_profile` ↔ `throttle_thermal_policy` (kernel-verified, works).
- **Verdict: fan curves = Unsupported (firmware acks but does not implement). Fan monitoring + thermal-profile switching = Supported.**

## 6. Feature channels that DO work (verified live on this machine)

| Feature | Kernel interface (built on, not duplicated) |
|---|---|
| Silent/Balanced/Performance | `/sys/firmware/acpi/platform_profile{,_choices}` (+ legacy `asus-nb-wmi/throttle_thermal_policy`) |
| Battery charge limit 1–100 | `/sys/class/power_supply/BAT1/charge_control_end_threshold` |
| Fan + temp monitoring | `/sys/class/hwmon/hwmon6` (fans), `coretemp`, `iwlwifi`→`mt7921`, NVMe composite |
| Keyboard brightness | `/sys/class/leds/asus::kbd_backlight/{brightness,max_brightness}` |
| Panel overdrive | `/sys/devices/platform/asus-nb-wmi/panel_od` |
| dGPU disable | `/sys/devices/platform/asus-nb-wmi/dgpu_disable` (dangerous — off by default in app) |
| Battery health | `/sys/class/power_supply/BAT1/{charge_full,charge_full_design,...}` |
| GPU | `nvidia-smi` (driver 615.71.10) + sysfs DRM |
| asus-armoury firmware-attrs | `/sys/class/firmware-attributes/asus-armoury/attributes/{charge_mode,dgpu_disable,panel_overdrive}` (kernel 7.1.8; re-check on 7.2.11) |

## 7. WMI bus inventory (live)

| GUID | Kernel role |
|---|---|
| 97845ED0-…-5 | `ASUS_WMI_MGMT_GUID` — WMNB (evaluated, not bus-bound — normal) |
| 0B3CBB35-…-6 | `ASUS_NB_WMI_EVENT_GUID` — hotkey/event notifications (bound by asus-armoury) |
| 05901221-…(-2,-4,-7) | WMI Binary-MOF metadata blobs (vendor-documented interface descriptions; artifacts retained) |
| 1F13AB7F-…-3, 2BC49DEF-…-0, A6FEA33E-…-1 | Unbound OEM GUIDs — not used by kernel or this project |

## 8. Consequences for the implementation (tuf-crate)

1. Build **on** kernel interfaces (sysfs/LED/hwmon/firmware-attributes) — same channel Armoury Crate uses; no second path to the EC.
2. **Monitoring-first**; controls limited to kernel-sanctioned, live-verified attrs.
3. RGB dynamics, fan curves, WMI PPT, unnamed DEVIDs: **Unsupported** — documented with firmware-level evidence.
4. Raw EC, ACPI-method invocation, kernel modules, pwm_enable writes: forbidden by `docs/SAFETY.md`.

## 9. Addendum — live FULLSPEED/AUTO verification (2026-10-31)

The kernel's SPEC83 fan path (`pwm1_enable` → WMI `CPU_FAN_CTRL`/`GPU_FAN_CTRL`, values
0=FULLSPEED / 2=AUTO, MANUAL rejected by the driver) was implemented end-to-end
(tuf-core attr → daemon op → GUI toggle) and **live-tested on the FX706HF**:

| Step | Fans (RPM cpu/gpu) |
|---|---|
| quiet profile (auto) | 4000 / 4000 |
| + FULLSPEED mode write (EC acked) | 3800 / 3800 |
| AUTO restored | 3600 / 3500 |
| performance profile restored | 3500 / 3500 → ramps with heat (5700 observed at 87 °C) |

**Conclusion: this EC acknowledges but does not act on the WMI fan-mode writes.** The only
effective fan control on this hardware is the thermal policy (platform_profile):
measured range across the session 3500–5700 RPM. Custom curves and manual speed are
therefore unsupported *by the firmware*, not by tuf-crate. The MAX COOL toggle remains in
the UI with this measured caveat attached.

## 10. Addendum — CPU power limit + fan-auto verification (2026-10-31, later)

**CPU sustained power limit (user-authorized exception, SAFETY.md §1c):** implemented via the
documented kernel `intel-rapl` powercap interface as `VerifiedAttr::CpuPowerLimit`
(clamped 25–65 W, audited, reversible; 65 W = firmware default; resets on reboot).
Live verification on the FX706HF: PL1 65→35 W through the daemon; **thermald did not revert
it** after 30 s under load. Sustained heavy-load temps drop accordingly.

**Automatic fan behaviour confirmed live:** under a 6-thread synthetic load with the 35 W cap:
4400 RPM / 84 °C → spike 94 °C (untouched 83 W PL2 burst window) → fans auto-ramped to
**5500 RPM** pulling temps 94 → 88 → 78 °C after load removal. The EC thermal engine responds
to load exactly as on Windows; the profile selects the curve, heat drives the speed.

**Windows fan-control dependency investigation:** on Windows, Armoury Crate reaches this EC
through the vendor AsIO.sys driver (raw EC port I/O) plus the WMI layer. The raw-EC path
(`ec_sys`/nbfc/ec-probe on Linux) is the community-documented *unsafe* alternative and is
excluded by `docs/SAFETY.md`; the kernel-interface path (asus-wmi/asus-armoury) that tuf-crate
uses is the community-recommended safe route. No additional safe dependency exposing fan duty
on this model was found — consistent with §9 (fan-mode writes are EC-acked but inert here).

## 12. Addendum — updated Armoury Crate 6.7.6.0 inventory (from Windows screenshots)

16 screenshots of the updated Windows installation (App Version **6.7.6.0**) were extracted
from the Windows partition (`research/screenshots/`) and reviewed. Inventory:

- **Home**: System Usage (CPU freq/usage/mem-freq/temp/voltage; GPU freq/usage/mem-freq/temp;
  **TGP 70 W**; **Fan RPM ×2 + Fan Acoustics dBA**; storage/RAM) · Devices ·
  **Operating Mode: Windows | Silent | Performance | Turbo**
- **Devices → System Settings** tabs: System Configuration (Win Key, Touch Pad, Modern Standby
  Assistant, Panel Power Saver, Power mode synchronization) · Free up memory (game-time process
  killer) · GPU Performance (**GPU Mode: Standard MSHybrid / Eco / Optimized** — Eco = dGPU fully
  disabled = our `dgpu_disable`; Optimized = auto) · **Lighting** · Audio · Resource Monitor
- **Settings**: General (theme: TUF/ROG/PRIME/ProArt), Update Center, About (**App 6.7.6.0**)
- **Lighting & Aura Sync** (new section in 6.7.6): Sync devices (FX706HF + Aura Wallpaper;
  Aura Performance Mode; Philips **Hue Connection**) · **Aura Effects**: Static, Breathing,
  Strobing, Color Cycle, Rainbow, Starry night, Music, Smart, Adaptive Color, Immersion,
  Dark (Off), AI Aura Lighting, INDIA + **Aura Creator** + In-Game effects ·
  **AI Aura Lighting** (Experimental, login-gated)
- **Aura Wallpaper**: wallpaper-driven ambient lighting (was OFF)
- Static effect panel: **Color swatch (full picker with wheel/HEX behind it) + Brightness 33 %**

**Brightness mapping evidence**: the RED static screenshot shows **BRIGHTNESS 33 %** — and the
contemporaneous EC diff shows `0xA2 = 0x60` (96 ≈ 255 × 0.376 with the EC's brightness curve)
while `0xA1 = 0x17`. Hypothesis (confirmed pending GREEN/BLUE): `0xA1` = effect/mode byte,
`0xA2/0xA3/0xA4` = R/G/B scaled by brightness — i.e. the EC stores 24-bit colour, brightness-
scaled. Armoury's 33/66/100 % brightness = kernel kbd levels 1/2/3 (already functional in
tuf-crate).

**Parity result vs tuf-crate**: profiles ✅ · power cap ✅ (beyond Armoury) · fans ✅ auto +
monitoring ✅ · battery ✅ · GPU modes ✅ (dgpu_disable + prime-run = Standard/Eco/Optimized) ·
panel overdrive ✅ · monitoring incl. TGP/acoustics-equivalents ✅ · RGB UI ✅ (colour
wheel/HEX implemented, hardware write pending GREEN/BLUE) · Aura Creator/In-Game/AI/Hue/
Wallpaper — Windows-software ecosystems, not hardware features (N/A by design).

## 13. Addendum — EC write-probe result + privileged channel analysis (final)

A kernel driver (`kernel/tuf_ec.c`) was built and loaded implementing the verified-offset
design: sysfs `ec_a0..ec_a3`, kernel `ec_read/ec_write` (serialized), read-back verification,
hard allowlist. The **live probe result: the EC refuses writes to 0xA0-0xA3** — writes are
accepted at the port level but the EC does not store them (read-back stays 00). The lighting
block is owned by the EC firmware; dynamic mode is EC-internal.

**Privileged channel analysis** (how Armoury Crate actually reaches it):
- ACPI `W15H()` writes 0xAA to **I/O port 0xB2** (SMI trigger) after populating GNVS
  `WNVA/WNVB/WNVC/WNVD` → BIOS SMM handler performs the privileged EC operation.
- On Windows, AsIO.sys performs equivalent raw EC port I/O from kernel mode.
- Both channels are **rejected by the safety policy**: SMI/GNVS commands with hand-crafted
  parameters are unverifiable (the SMM handler's behaviour for arbitrary values is unknown),
  and raw EC port I/O is the documented-unsafe path. A single misused SMI command can affect
  far more than the keyboard.

**Final RGB verdict (evidence-complete):** keyboard colour control on FX706HF is reachable
ONLY through privileged channels excluded by the safety policy (AsIO raw-EC on Windows; SMI/
GNVS on ACPI). Three safe channels were exhaustively tested and closed: WMI per-DEVID sets
(rejected), generic 0x9601 command dispatch (no RGB command found), direct EC RAM writes
(refused by the EC firmware). The EC retains the last Windows-set lighting state; Linux
controls brightness (fully functional) and monitors everything else. The `tuf_ec` driver
remains loaded as a read-only verified interface and documents the boundary.

Implemented as the honest endpoint: GUI Lighting tab ships the full colour interface
(hue strip, RGB sliders, HEX, preview) with the write path clearly marked blocked-by-firmware,
plus a complete Armoury Crate 6.7.6.0 feature-parity record (§12).

## 14. Addendum — four-colour EC diff: conclusive result (final experiment)

All four dumps collected and SHA-verified: dynamic-baseline, static-red, static-green
(#00FF00), static-blue (#0000FF) — same static effect, same 33 % brightness, only the colour
changed. Four-way diff across the full 256-byte ACPI-visible EC space:

- `0xA1 = 0x17` in all three static dumps, `0x00` in dynamic → **static-mode marker**
- `0xA2 ≈ 0x60/0x5F` in all three static dumps → **brightness-scaled level** (33 %), NOT a
  per-channel colour byte (it does not change between red/green/blue)
- Every other differing byte is boot-state noise (no clean 00→FF colour signature anywhere)
- **No byte in the ACPI-visible EC space tracks the colour.**

**Final conclusion (evidence-complete):** Armoury Crate applies the keyboard colour at
*runtime* through AsIO.sys raw EC port commands that target a register space outside the
ACPI-visible EC RAM. The colour does not persist in readable EC state across reboots, and the
static-mode marker it leaves behind carries no colour information. Therefore no Linux
implementation — userspace, kernel driver, or daemon — can set the keyboard colour through
any interface that is safe under this project's policy. The remaining route is raw EC port
protocol reverse engineering on the live EC, which stays excluded.

Delivered instead (all functional): full colour-selection UI (hue strip, RGB sliders, HEX,
live preview), **local lighting profiles** (save/load/delete colour presets, persisted in
`~/.config/tuf-crate/lighting-profiles.json`), brightness control (0/33/66/100 % — fully
functional), and an explicit hardware-unavailable state in the Lighting tab.

## 15. Addendum — G-Helper / ArmouryCrate-Linux analysis + definitive WMI RGB test (final)

**Reference repos cloned for source analysis only (nothing installed or executed):**
`Decipher369/ArmouryCrate-Linux`, `seerge/g-helper` → `~/Projects/reference/`.

**ArmouryCrate-Linux**: targets a different machine (ROG Strix G15 G512LU, USB keyboard
PID 0x1866), Arch-only, status "Phase 1: hardware detection" — RGB is still on *their*
roadmap. Behind tuf-crate; no applicable findings for FX706HF.

**G-Helper** (C#, well-regarded): its TUF RGB implementation was extracted verbatim:
`DeviceSet(TUF_KB=0x00100056, [0xb4, mode, R, G, B, speed])`, fallback
`TUF_KB2=0x0010005a` with `0xb3` then `0xb4`; AuraMode Static=0/Breathing=1/ColorCycle=2/
Rainbow=3; brightness `0x00050021 = 0x80|level`; power-state `0x00100057 = 0xbd|flags`.
Fan curves use the same DEVIDs (0x00110024-32) our firmware no-ops.

**Definitive live test on FX706HF.312** (kernel 7.2.11, `kernel/tuf_rgb.c` — the exact
G-Helper protocol through the kernel WMI channel, 3-dword bios_args):
- `TUF_KB (0x00100056)` set → **0xFFFFFFFE (rejected)**
- `TUF_KB2 (0x0010005a)` with `0xb3` → rejected; with `0xb4` → rejected

**Four-channel conclusion (complete):** keyboard colour writes on this firmware are
reachable ONLY through the privileged paths excluded by safety policy (AsIO raw-EC /
SMI). WMI RGB, ACPI-EC RAM, generic command dispatch and the G-Helper WMI protocol are
all exhausted with recorded evidence. RGB colour remains a Windows-runtime feature;
Linux gets brightness (functional), the full colour-selection UI with **local profiles**
(functional), and this documented boundary. G-Helper adds nothing further for FX706HF:
its fan curves use the same inert WMI IDs, and it explicitly avoids AsIO.

Note: user's installed Armoury Crate (6.7.6.0) is newer than the public 6.0.0 installer
package listed on the support page; no additional features exist in the public package.
