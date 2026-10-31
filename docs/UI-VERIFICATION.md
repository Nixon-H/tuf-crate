# UI verification & bug audit — 2026-10-31

Method: the GUI ran against a **mock backend on an isolated Xvfb virtual display** and was
driven with `xdotool` (tab clicks, button presses, confirmation flows), with `xwd` captures
reviewed after each step. The user's live desktop session was deliberately **not** touched
(an initial attempt that interacted with the real display was aborted and its captures deleted).

## Render verification — all 7 tabs
| Tab | Result |
|---|---|
| Dashboard | ✅ telemetry grid, thermal bars |
| Performance | ✅ Silent/Balanced/Performance buttons + policy notes |
| Fans & Thermals | ✅ RPM rows, GPU throttle warning, curve-unsupported explanation |
| Battery | ✅ health, slider, presets, Apply |
| Lighting | ✅ brightness levels, EC-persistence and RGB notes |
| Display & GPU | ✅ panel_od checkbox, GPU telemetry, dGPU section |
| About | ✅ capability grid incl. model verification |

## Interaction verification (mock backend)
- Profile → Silent: ✅ status `profile -> Silent`; Dashboard shows `Current profile: quiet` (mock CPU temp model follows the profile)
- Battery 60 % preset + Apply: ✅ slider + state update; status `charge limit -> 60 %`
- Battery 100 % preset + Apply: ✅ `charge limit -> 100 %`
- Lighting level click: ✅ (state change, success path)
- panel_od checkbox toggle: ✅ visual + state flip both ways
- dGPU disable → confirmation dialog appears ✅ → Cancel leaves state unchanged ✅

## Bugs found by the audit and fixed
1. **Misleading monitoring-mode errors** — every control reported "unsupported operation on this
   hardware" when writes were merely disabled. All five write sites now report:
   *"writes are disabled — launch tuf-crate with --writes to enable hardware controls"*
   (`friendly_error()` mapping), verified on-screen in monitoring mode.
2. **MockBackend ignored the monitoring posture** — the mock accepted writes while the header
   claimed "monitoring-only". `MockBackend::with_writes(bool)` now enforces the same gate as
   `SysfsBackend`; default is monitoring-only everywhere. Verified both directions.
3. **Mode label bug** — `--mock --writes` displayed "monitoring-only" (label condition had
   `&& !mock`). Now shows `writes ENABLED (mock)`. Verified on-screen.
4. **Tofu glyph** — status messages used "→", missing from egui's default font (rendered as □).
   Switched to ASCII "->". Verified on-screen.
5. **backup-config.sh could emit invalid JSON** when a sysfs file existed but was empty.
   Rewritten around python3 with null/integer handling; validated against the live machine.

## Test suite after the audit
17 tests (5 tuf-core + 12 tuf-hal) — including four new degraded-capability fixtures
(no battery / no asus-nb-wmi / no LED+profile / non-FX706HF DMI) proving graceful,
per-interface-safe behaviour on machines that lack any given interface.
