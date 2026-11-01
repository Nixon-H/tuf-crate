# Unsupported features — and exactly why

Every entry below was **investigated on the actual hardware/firmware** before being classified. None of these is a "didn't get around to it".

| Feature | Why it is unsupported | Evidence |
|---|---|---|
| Dynamic Aura effects beyond Static | The upstream `kbd_rgb_mode` interface accepts Aura mode numbers, but only Static (mode 0) has been live-verified on this firmware. Other modes are deliberately not exposed until their behaviour is verified. | RESEARCH.md §16 |
| (Resolved) Static colour | **No longer unsupported** — static colour works through `kbd_rgb_mode` (WMNB DEVS 0x00100056). Earlier failures were caused by an off-by-one-dword WMNB buffer; see RESEARCH §16 and SAFETY §1d. | RESEARCH.md §16 |
| Custom fan curves | The firmware **acknowledges but does not implement** curve DEVIDs (0x00110024/25/32 `Return Zero`, no curve I/O). Kernel hwmon consequently exposes no `auto_points` on this machine — verified live. | RESEARCH.md §5; live hwmon6 listing |
| Manual fan PWM / `pwm*_enable` writes | The asus hwmon `pwm*_enable` can disable automatic fan control with **no curve behind it** on this EC → fan-stop risk. Rejected by safety policy regardless. | SAFETY.md §2 |
| WMI power limits (SPL/SPPT/FPPT) | PPT DEVIDs (0x001200A0/A3/B0/B1/C1) are AMD-platform interfaces; this firmware's sampled path returns `0xFFFFFFFE` and the machine is Intel i5-11260H + HM570. The running kernel exposes no ppt firmware-attributes on this model. | RESEARCH.md §3; live firmware-attrs = {charge_mode, dgpu_disable, panel_overdrive} |
| NVIDIA Dynamic Boost / thermal target control | Owned by `nvidia-powerd` (already running) and the WMI 0x001200C0/C2 path is not exposed to userspace by any kernel driver. Writing GPU power state from userspace is forbidden. | COMPATIBILITY.md |
| ~40 unnamed WMNB DEVIDs (0x00100021…0x00130041) | Firmware-private constants used by Armoury Crate/vendor services with no public documentation. Semantics unknown → per safety policy: never written. | RESEARCH.md §3 |
| BIOS/EC firmware inspection beyond static ACPI decompile | Flashing or modifying firmware is prohibited; EC RAM access from Linux would require the `ec_sys` debug module (writes not permitted by this project). | SAFETY.md §2 |
| Refresh-rate switching from within tuf-crate | Requires X11/XFCE display configuration that your desktop environment already owns; duplicating it risks conflicting display state. Displayed as guidance only. | COMPATIBILITY.md |

## The honest summary

Armoury Crate on Windows reaches the EC through vendor kernel drivers that own privileged channels (WMI + AsIO). On the FX706HF the *useful* subset of those channels is already exposed by in-kernel drivers — that subset is what tuf-crate provides (profiles, charge limit, brightness, **static keyboard RGB**, panel overdrive, dGPU mode, fan safety modes, CPU power cap, full monitoring). The remaining subset (advanced Aura effect modes, fan curves, PPT) is either **rejected by this model's own firmware** over WMI or reachable only through channels this project's safety policy forbids touching.
