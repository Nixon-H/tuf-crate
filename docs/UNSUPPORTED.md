# Unsupported features — and exactly why

Every entry below was **investigated on the actual hardware/firmware** before being classified. None of these is a "didn't get around to it".

| Feature | Why it is unsupported | Evidence |
|---|---|---|
| AI Aura (Windows AI service) | **Closed as out of scope**: Armoury Crate's AI effect needs ASUS's Windows AI service and models; there is no firmware hook. Music, Smart and Adaptive Color are implemented as Linux-native software effects. | docs/SCOPE.md |
| Aura Creator per-key effects | **Closed as physically impossible**: the FX706HF keyboard is single-zone (one colour/effect at a time). | docs/SCOPE.md |
| In-app updater | **Closed as out of scope**: updates are owned by the package manager / git checkout; no network or privilege surface is added. | docs/SCOPE.md |
| (Resolved) Static + dynamic colour | **No longer unsupported** — all 11 firmware effect modes, arbitrary 0–255 RGB mixing, speed 33/67/100 % and brightness work through `kbd_rgb_mode`/`kbd_rgb_state`. | docs/RGB.md |
| Live dGPU power-off (Eco Mode) | `dgpu_disable=1` on the running EC froze the compositor on this machine; it is only safe before the GPU driver binds. Live writes are blocked; restore-only is exposed. | observed 2026-11-02; SAFETY.md |
| Custom fan curves | The firmware **acknowledges but does not implement** curve DEVIDs (0x00110024/25/32 `Return Zero`, no curve I/O). Kernel hwmon consequently exposes no `auto_points` on this machine — verified live. | RESEARCH.md §5; live hwmon6 listing |
| Manual fan PWM / `pwm*_enable` writes | The asus hwmon `pwm*_enable` can disable automatic fan control with **no curve behind it** on this EC → fan-stop risk. Rejected by safety policy regardless. | SAFETY.md §2 |
| WMI power limits (SPL/SPPT/FPPT) | PPT DEVIDs (0x001200A0/A3/B0/B1/C1) are AMD-platform interfaces; this firmware's sampled path returns `0xFFFFFFFE` and the machine is Intel i5-11260H + HM570. The running kernel exposes no ppt firmware-attributes on this model. | RESEARCH.md §3; live firmware-attrs = {charge_mode, dgpu_disable, panel_overdrive} |
| NVIDIA Dynamic Boost / thermal target control | Owned by `nvidia-powerd` (already running) and the WMI 0x001200C0/C2 path is not exposed to userspace by any kernel driver. Writing GPU power state from userspace is forbidden. | COMPATIBILITY.md |
| ~40 unnamed WMNB DEVIDs (0x00100021…0x00130041) | Firmware-private constants used by Armoury Crate/vendor services with no public documentation. Semantics unknown → per safety policy: never written. | RESEARCH.md §3 |
| BIOS/EC firmware inspection beyond static ACPI decompile | Flashing or modifying firmware is prohibited; EC RAM access from Linux would require the `ec_sys` debug module (writes not permitted by this project). | SAFETY.md §2 |
| Refresh-rate switching from within tuf-crate | Requires X11/XFCE display configuration that your desktop environment already owns; duplicating it risks conflicting display state. Displayed as guidance only. | COMPATIBILITY.md |

## The honest summary

Armoury Crate on Windows reaches the EC through vendor kernel drivers that own privileged channels (WMI + AsIO). On the FX706HF the *useful* subset of those channels is already exposed by in-kernel drivers — that subset is what tuf-crate provides (profiles, charge limit, brightness, **static keyboard RGB**, panel overdrive, dGPU mode, fan safety modes, CPU power cap, full monitoring). The remaining subset (fan curves, PPT, AI Aura, per-key Aura Creator, in-app updater) is either **rejected by this model's own firmware**, **physically impossible** on single-zone hardware, or a Windows/package-manager concern — all closed explicitly in `docs/SCOPE.md`.
