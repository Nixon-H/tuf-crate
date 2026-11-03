# ASUS Linux tooling research → native implementation mapping

Research date: 2026-11-02. Sources were cloned read-only into `/tmp` and inspected;
nothing from these projects is bundled, executed or wrapped by tuf-crate.

Projects reviewed:

- asusctl / asusd — https://gitlab.com/asus-linux/asusctl
- supergfxctl / supergfxd — https://gitlab.com/asus-linux/supergfxctl
- ghelper-for-linux — https://github.com/Ichihiroy/ghelper-for-linux

## What each project actually uses

### asusctl

| Feature | Mechanism found in source | Present on this FX706HF / kernel 7.2.11? |
|---|---|---|
| Quiet/Balanced/Performance | `/sys/firmware/acpi/platform_profile` via `rog-profiles` | **Yes** |
| Fan curves | hwmon node named **`asus_custom_fan_curve`** (`find_fan_curve_node()`); writes `pwmN_auto_pointN_*`, then `pwmN_enable` | **No** — this kernel exposes only the `asus` hwmon with `fanN_input`/`pwmN_enable`; no `asus_custom_fan_curve` node and no `auto_points` |
| CPU/GPU PPT (SPL/SPPT/FPPT) | `asus-armoury` firmware attributes `ppt_pl1_spl`, `ppt_pl2_sppt`, `ppt_pl3_fppt`, `ppt_platform_sppt` | **No** — this model's asus-armoury exposes only `charge_mode`, `dgpu_disable`, `panel_overdrive`, `pending_reboot` |
| Battery threshold | `charge_control_end_threshold` | **Yes** |
| Keyboard RGB | `asus::kbd_backlight` `kbd_rgb_mode`/`kbd_rgb_state` | **Yes** |
| Anime Matrix, eGPU, MUX | model-specific ASUS kernel/hardware | **No hardware / not exposed** |

asusctl's own README notes that fan curves only work on supported models (G14/G15 and
some TUF like FA507) and that many features are developed alongside ASUS kernel patches.
The FX706HF is not one of the fan-curve models.

### supergfxctl

- Reads/writes exactly `/sys/devices/platform/asus-nb-wmi/dgpu_disable`
  (`special_asus.rs::ASUS_DGPU_DISABLE_PATH`).
- Optional MUX: `/sys/devices/platform/asus-nb-wmi/gpu_mux_mode` — **absent** on this
  laptop (no MUX).
- It does **not** write `dgpu_disable` while the graphical session is live. For
  Hybrid↔Integrated it ends the user sessions through logind, writes the mode, and the
  user logs back in. The README states: switching to/from Hybrid requires **logout**
  (no reboot); MUX mode always requires a reboot.
- `supergfxd` also has a boot-time safety check: if `dgpu_disable=1` but the MUX is on
  dGPU, it turns `dgpu_disable` off before continuing.

**Conclusion for the freeze:** Windows/Armoury and supergfxctl both arrange a
session/reboot boundary; there is no safe “power the dGPU off while the compositor owns
it” write. The observed Linux freeze is the expected result of doing that.

### Fan control: definitive kernel evidence on this FX706HF

`dmesg` from this machine (BIOS FX706HF.312, kernel 7.2.11):

```
asus_wmi: fan_curve_get_factory_default (0x00110024) failed: -61
asus_wmi: fan_curve_get_factory_default (0x00110025) failed: -61
asus_wmi: fan_curve_get_factory_default (0x00110032) failed: -19
asus_armoury: No matching power limits found for this system
```

Those are the CPU/GPU/MID firmware curve DEVIDs the kernel driver queries at probe time.
Because they fail, the kernel does not register `asus_custom_fan_curve`, and it does not
expose `pwm*_auto_point*` or `fan_boost_mode`. asusctl and the Avalonia
`utajum/g-helper-linux` both look for exactly that hwmon node; without it their fan-curve
UI reports unsupported. The firmware also rejects the `ppt_*` attributes
("No matching power limits").

A raw EC fan write would bypass every kernel safety check and the EC register map is not
documented for this board — that is forbidden by `docs/SAFETY.md`. **There is no safe
software path to 100 % fans on this model.** The effective controls are the firmware
thermal policy (Silent/Balanced/Performance) and load; RPM is EC-reported and now graphed.

### ghelper-for-linux

Current scope is much smaller than G-Helper on Windows:

- battery charge cap (20–100 %) at `charge_control_end_threshold`, persisted by a systemd
  service + udev rule;
- battery/thermal/memory monitoring.

Fan curves, platform profiles, GPU modes and keyboard backlight are listed as *future
ideas*, not implemented. tuf-crate already implements all of those except fan curves/PPT
(which are firmware-blocked here).

## Native integration decisions in tuf-crate

| Capability | tuf-crate implementation | Status |
|---|---|---|
| Profiles | direct `platform_profile` through the verified daemon | working |
| Battery limit persistence | daemon boot config now supports `charge_threshold` (applies before login) | working after daemon reinstall |
| GPU Standard restore | live `dgpu_disable=0` (safe direction) | working |
| GPU Eco | **boot-time schedule**: daemon writes `dgpu_disable=1` before `display-manager.service`, then user reboots | implemented; needs updated `tuf-crated` installed |
| CPU sustained cap | Intel RAPL `constraint_0_power_limit_uw` (65 W default) | working |
| Custom fan curves | not possible: no `asus_custom_fan_curve` node; WMI curve IDs no-op | firmware-limited |
| PPT SPL/SPPT/FPPT | not exposed by asus-armoury on this model | firmware-limited |
| MUX / dGPU-primary | no hardware | not present |
| Keyboard RGB/brightness | direct LED class | working |

No asusctl/supergfxctl/ghelper binary, library or DBus API is called. The sysfs and
firmware-attribute interfaces were implemented natively in `tuf-hal` / `tuf-daemon`.

## Safe Eco switching procedure (implemented)

1. User chooses “Schedule Eco for next boot” in System Configuration → GPU Performance.
2. The daemon merges `{"dgpu_disable": true}` into `/var/lib/tuf-crate/boot.json`.
3. On the next boot `tuf-crated` starts after local-fs and **before** the display manager,
   applies the flag while no NVIDIA client exists, and logs it.
4. The user boots into an iGPU-only desktop. “Restore Standard” remains available live and
   also clears the boot flag so the next boot returns to hybrid.

This avoids the compositor freeze entirely. A logout-assisted switch (supergfxctl-style)
is possible in principle, but it must terminate the session before writing; the GUI cannot
do that safely from inside itself.
