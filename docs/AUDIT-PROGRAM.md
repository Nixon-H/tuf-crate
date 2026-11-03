# Ten-plus-pass audit program — tuf-crate UI

Objective: audit UI, UX, glitches, bugs and code repeatedly, refining until the shell is
coherent, responsive, safe and verifiable. This file is the completion index for **11
audit passes** (the requested minimum was 10). Detailed findings remain in
[`UI-AUDIT-2026-11.md`](UI-AUDIT-2026-11.md) and test evidence in
[`UI-VERIFICATION.md`](UI-VERIFICATION.md).

## Audit passes

| Pass | Scope | Representative fixes | Evidence |
|---|---|---|---|
| 1. Visual composition | Home/Devices/Aura/Settings layout, palette, hero, text walls, selected fills | responsive Home, neutral `#2C2C31` selection, exact hero placement, card/bento layouts, segmented controls, fan history | 1920×1000/1080 captures; first audit table |
| 2. Interaction surface | navigation, header icons, sidebar, tooltips, info icons, hit areas | clickable ROG/keyboard/speaker/battery, animated sidebar, clickable/pinned info, toast instead of duplicate status bar, compact min-size Home | header/sidebar captures; interaction suite |
| 3. State and safety | restart state, process list, dGPU, daemon writes, audit log | PID-reuse-safe termination, atomic settings, live dGPU guard, audit rotation, async apt check | daemon/core/HAL tests; runtime guard output |
| 4. UI/UX consistency | labels, empty states, dead buttons, hover/focus, microcopy | palette-token effect buttons, Aura Sync action hierarchy, dynamic GPU/panel labels, pointer affordances, larger info target | page captures; copy/code review |
| 5. Responsive/DPI | 1100×680, 1366×768, 1920×1000/1080, 150%, live resize | short-window Home, auto-collapsed sidebar at narrow/short viewports, overlay drawer, scrollable nav, no content squeeze | `/tmp/scale150-1366*.png`; suite resize captures |
| 6. Control behavior | every tab, radio, checkbox, combo, segmented control, theme card, header/sidebar control | device-image persistence, applied-RGB persistence, immediate process refresh, Follow System test, header/sidebar/resize automation | extended `ui-interaction-test.sh` |
| 7. Persistence/restart | theme, scale, launch page, device image, GameVisual, RGB, timeout, audio | display mode/dim restore, applied-RGB fields, device-image save, audio re-read on page entry, timeout state | isolated `ui.json` flip test (`device_line_art` false→true→false); schema inspection |
| 8. Monitoring edge cases | absent dGPU, N/A fields, no battery, missing sensors, fan zero/static | no fake `0 °C/0 W` readings, N/A nvidia-smi rejected, battery empty state, auto-scaled fan history | nvidia parser tests; captures |
| 9. Safety review | closed write enum, confirmations, clamps, dGPU/fan/PL1, process kill, daemon socket | live-session dGPU skip, PID identity revalidation, daemon/client timeouts, request cap, systemd sandbox, read-only telemetry | `dgpu_disable=skipped-live-session` runtime log; `systemd-analyze verify` |
| 10. Performance/code quality | repaint cadence, subprocesses, scans, panic paths, parsing, unused code | pause idle animations when unfocused, process/audio caches, bounded nvidia-smi, no GUI/CLI unwraps, dead-binding cleanup | clippy; process/CPU captures |
| 11. Accessibility/theme/assets | contrast, focus indicators, labels, dark/light, asset paths, installer assets | theme-aware header/success colors, widget labels/focus rings, keyboard activation for nav/theme/header/segments, 20px info targets, canonical install-relative asset lookup, local-only asset copy | Tab/Enter focus pass; light-theme captures; installed-binary launch from `/tmp` with isolated HOME and installed assets |

## Prompt-to-artifact checklist

| Requirement | Concrete evidence |
|---|---|
| At least 10 audit passes | This 11-row audit index; dated findings tables in `UI-AUDIT-2026-11.md` |
| UI audit | Screenshot set for every screen at 1920×1000/1080, 1366×768, 1100×680, 150% scale, dark/light themes |
| UX audit | Progressive disclosure, tooltips, focus/hover states, empty/error states, scoped watcher copy |
| Glitches/bugs fixed | 68 findings across all rounds: overflow, clipping, duplicate toast, glyph boxes, fake readings, stale state, mid-animation clipping |
| Code audit | `cargo clippy --workspace --all-targets` clean; no runtime `unwrap`/`panic!` in GUI/daemon paths; bounded subprocesses |
| Safety | Closed verified-op enum only; no raw EC/ACPI; dGPU guard; PID identity; timeouts; unit sandbox; audit rotation |
| Regression tests | `cargo test --workspace`: **41 passed, 0 failed** (5 core + 9 daemon + 10 GUI + 17 HAL) |
| Interaction tests | `scripts/ui-interaction-test.sh` final: **failures=0**; ROG/Light/Follow-System persistence, audited RGB write, splash deltas, header/sidebar/resize checks |
| No destructive operations added | No new sysfs paths or EC/ACPI calls; hardware actions remain profile/brightness/charge/panel/RGB/PL1/boot-dGPU (clamped, confirmed, audited) |

## Current verification snapshot

```text
cargo test --workspace                 41 passed, 0 failed
cargo clippy --workspace --all-targets clean
scripts/ui-interaction-test.sh         failures=0
systemd-analyze verify <unit>          clean
daemon live dGPU guard                 dgpu_disable=skipped-live-session
```

## Known accepted limits

- Always-on game rules and backlight timeout require tuf-crate to be open; the GUI is not
  a background service.
- The local verified-op socket remains `0666` for immediate use without re-login. It is
  bounded, audited, and exposes no raw EC/ACPI path.
- Daemon/service changes are live: the privileged installer completed and restarted `tuf-crated`; the installed daemon hash matches the release build.
- ASUS cloud AI, Aura per-key lighting, MUX switching and firmware fan-curve/PPT attributes
  remain hardware/service limits documented in `SCOPE.md`/`UNSUPPORTED.md`.
