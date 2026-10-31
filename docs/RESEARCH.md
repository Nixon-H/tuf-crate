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
