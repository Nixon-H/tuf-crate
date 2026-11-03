# Windows ASUS software stack — full inventory vs Linux (FX706HF)

Investigated per request: everything ASUS-related on Windows, not only Armoury Crate —
what each component does, what channel it uses, and whether tuf-crate covers it.

| Windows component | What it actually does | Channel | Linux equivalent / status |
|---|---|---|---|
| **ASUS System Control Interface** (driver bundle: AsIO.sys, AsCom.sys, AsusSysIO, SSIF) | The privilege layer: AsIO performs **raw EC port I/O + WMI** on behalf of Armoury Crate/MyASUS; AsCom bridges user services to kernel | proprietary kernel drivers | **Excluded by safety policy** (raw EC = the documented-unsafe path). Kernel `asus-wmi`/`asus-armoury` provide the sanctioned subset |
| **Armoury Crate / Armoury Crate Service + Agent** | UI + policy: profiles, fan curves (where supported), RGB, display overdrive | WMI via System Control Interface | ✅ **tuf-crate** (profiles, power cap, monitoring); RGB/curves firmware-blocked on this model |
| **ASUS Optimization (ASUSOptimization.exe)** | Thermal/fan policy enforcement, hardware monitor polling, Fn-key routing | WMI + EC | ✅ kernel `asus-armoury` hotkeys + EC thermal engine (fans auto-ramp — verified live) |
| **ASUS Switch / ASUSSwitch** | GPU mode switching (Optimus/Ultimate on MUX models) | WMI + NVIDIA APIs | FX706HF has no MUX; PRIME offload covers it (`prime-run`) |
| **ASUS Battery Health Charging** | 60/80/100 % charge thresholds | WMI DEVID 0x00120057/6C | ✅ native (`charge_control_end_threshold`, firmware-attributes `charge_mode`) |
| **MyASUS** | Diagnostics, updates, support, battery modes | System Control Interface | ➖ fwupd covers firmware updates; diagnostics via smartctl/sensors; support features N/A |
| **Aura Sync / Armoury Crate Core (lighting service)** | RGB effects for keyboard + peripherals | **AsIO.sys raw EC I/O** (no ACPI/WMI path exists on this firmware — RESEARCH §4/§11) | 🟥 unlock in progress: read-only EC-diff RE to locate the lighting bytes, then a minimal verified driver; EC retains last Windows-set colour meanwhile |
| **ASUS Update / ASUSFW update mechanism** | BIOS/EC firmware flashing | Windows flasher / EZ Flash | ⛔ out of scope (SAFETY.md); use ASUS官方 tools + verify version first |
| **Realtek/MediaTek vendor drivers** | Wi-Fi/BT/audio firmware loading | Windows driver stack | ✅ linux-firmware (mediatek/realtek packages installed and current) |

**Bottom line:** every Windows ASUS capability that this hardware exposes through *any*
channel tuf-crate can safely use is covered (profiles, power, battery, monitoring, hotkeys,
overdrive, offload). The only Windows-only channels are raw-EC access (unsafe, excluded by
policy — and live-proven unnecessary: the profiles already drive the fans) and RGB effect
 authoring (firmware-rejected on this model regardless of channel).

Persistence note: Windows keeps settings in the ASUS services; on Linux tuf-crate's
`tuf-crated` daemon re-applies your saved settings at every boot
(`/etc/tuf-crate/apply.json`, managed from the GUI's "24/7 persistence" section).
