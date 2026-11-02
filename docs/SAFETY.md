# Hardware Safety Documentation — tuf-crate

**Priority order: hardware safety > feature completeness.** This document is binding for every change to this repository.

## 1. The safety boundary (enforced by construction)

- Hardware writes are expressible **only** as `tuf_core::VerifiedAttr` — a closed enum of kernel-sanctioned, model-verified attributes:
  1. `platform_profile` (thermal/fan policy)
  2. `charge_control_end_threshold` (battery, 20–100 %)
  3. `asus::kbd_backlight/brightness` (0..max_brightness)
  4. `panel_od` (0/1)
  5. `dgpu_disable` (0/1, **double-confirmation gated**)
  6. `pwm*_enable` fan FULLSPEED/AUTO (0/2 only)
  7. `intel-rapl` PL1 (25–65 W, lower-only; §1c)
  8. `kbd_rgb_mode` — TUF keyboard colour through the **upstream asus-wmi** attribute (§1d)
- There is **no API** in the workspace to: write an arbitrary path, read/write EC registers (ECRAM), invoke ACPI methods, run `acpi_call`, load kernel modules, patch kernels, or write raw fan PWM values.
- `SysfsBackend::verified_write()` is the single write funnel: capability check → confirmation check → strict range clamp (out-of-range **errors**, never silently clamps) → write → JSONL audit.
- Writes are **disabled by default**: `SysfsBackend::real(false, …)` is monitoring-only. The GUI passes `--writes` to enable the verified ops; the CLI has no write paths at all.

## 1b. Privileged daemon (`tuf-crated`)

sysfs hardware attributes are root-writable, so a user-session GUI cannot write them directly.
tuf-crate therefore ships an optional root systemd service (`tuf-crated`, installed by
`scripts/install.sh`) — the same privileged-daemon pattern as asusd / power-profiles-daemon:

- The daemon exposes **exactly** the `VerifiedAttr` operations over a local unix socket
  (`/run/tuf-crate/control.sock`), through the same capability → confirmation → clamp → audit
  funnel. It contains no EC, ACPI, PWM, module-loading, or arbitrary-path code paths.
- Socket permissions are 0666 on a single-user machine: every local request is still a
  closed-enum, clamped, audited operation — the worst a local caller can do is change the
  same verified values the GUI can.
- Daemon writes are recorded to `/var/lib/tuf-crate/audit.jsonl`.
- Remove anytime: `scripts/uninstall.sh` (disables the unit and deletes the binary).

## 1c. User-authorized exception: CPU power limit (PL1)

On 2026-10-31 the owner explicitly authorized a CPU sustained-power-limit control,
overriding the original blanket "no power-limit changes" rule for THIS feature only:
- Interface: the documented kernel `intel-rapl` powercap sysfs
  (`/sys/class/powercap/intel-rapl:0/constraint_0_power_limit_uw`).
- Clamp: 25–65 W (65 W = firmware default; you can only LOWER power — the safe direction).
- Reversibility: write 65 W, or reboot (RAPL limits are not persistent).
- Still forbidden: voltages, clocks, PL2 changes, GPU power limits, EC writes.

## 1d. Keyboard RGB (verified 2026-11-01)

- Interface: the **upstream kernel** `asus-wmi` attribute
  `/sys/class/leds/asus::kbd_backlight/kbd_rgb_mode` (write-only, format
  `cmd mode red green blue speed`, cmd 1 = `0xb4` set, cmd 0 = `0xb3`).
  The kernel driver performs the WMNB `DEVS` call (DEVID `0x00100056`) and
  sanitises the mode (9 and ≥12 → 10) before it reaches firmware.
- Fallback only: the project's `tuf_rgb` module exposes the same call through
  `/sys/devices/platform/tuf-rgb/apply` for kernels that predate the upstream
  attribute. Its buffer layout must match the kernel's (`arg0 = DEVID`,
  magic `DEVS` is the WMI method id, not a buffer dword).
- Validation: every component 0–255; 5-tuple (module) or 6-tuple (kernel) only.
- Reversibility: send any colour again; the lighting state is volatile and
  resets to the previous static colour on reboot.
- Evidence: `DSTS(0x00100056)` = `0x00010000` (presence); `DEVS` with
  `[0xb4, mode, R, G, B, speed]` returns `1` (firmware accepted).
- Not included: any raw EC write, SMI invocation, AsIO/ATKACPI access, or
  firmware flashing. The earlier claim that `0x00120057` was the RGB command
  was a misidentification — that DEVID is `ASUS_WMI_DEVID_RSOC` (battery).

## 2. Prohibited operations (never implement)

Blind/undocumented EC register writes · experimental fan-control commands · unverified ACPI method invocation · BIOS/EC firmware modification or flashing · voltage/clock/power-limit forcing (incl. WMI PPT writes) · bypassing thermal protections · disabling ASUS kernel drivers · loading unverified kernel modules · direct userspace control competing with TLP/thermald/nvidia-powerd.

## 3. Model scoping

Capabilities are detected at runtime per-attribute (interface existence), and `CapabilitySet::is_fx706hf` records whether the machine is the verified model. Interfaces absent on the machine are refused even if present in the enum. Behaviour on other ASUS models is per-attribute safe (each attr is a standard kernel interface) but is **untested** — the GUI shows the model and its verification state.

## 4. Why the unsupported features are unsupported (evidence)

**dGPU live power-off (Eco Mode) is blocked.** Writing `dgpu_disable=1` while the NVIDIA
driver/compositor is active froze this FX706HF (2026-11-02). The app only offers the safe
direction (restore Standard = `dgpu_disable=0`) and states that a reboot may be needed.
A boot-time-only switch is the only acceptable future implementation.

See `docs/RESEARCH.md` §4–5 and `docs/UNSUPPORTED.md`. Headlines: this firmware returns `0xFFFFFFFE` (unsupported) for the legacy RGB DEVIDs `0x0010005A/0x00100066` and no-ops for fan-curve DEVIDs; `pwm*_enable` can stop fans; PPT DEVIDs are AMD-oriented and rejected by this Intel platform's firmware. The TUF RGB path (`0x00100056`) **is** supported and is exposed by the upstream kernel (§1d).

## 5. Testing posture

The `SysfsBackend` takes a root prefix; the entire test suite runs against fixture sysfs trees (`tempfile`-based). **No test ever touches real hardware.** MockBackend covers GUI development. `cargo test` must pass before any deployment.

## 6. Rollback

Every write path is a sysfs attribute that can be written back (profiles: echo previous choice; charge threshold: echo 100; brightness/panel_od/dgpu_disable likewise). The audit log (`~/.local/state/tuf-crate/audit.jsonl`) records every applied change with timestamps to make reversal deterministic. `scripts/uninstall.sh` removes the application without touching any system component.

## 7. Known interactions

- **TLP**: owns CPU governor/EPP and Wi-Fi powersave. tuf-crate never writes cpufreq — the Performance tab displays, not controls.
- **thermald / nvidia-powerd**: own thermal/GPU power policy. tuf-crate reads telemetry only from them.
- **asus-armoury firmware-attributes**: tuf-crate *displays* them and prefers the dedicated kernel attrs it already uses; it does not write firmware-attributes concurrently with anything else.
