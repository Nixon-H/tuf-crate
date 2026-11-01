# Interaction & animation test report (headless)

Executed with `scripts/ui-interaction-test.sh` under Xvfb 1920×1080. The owner's live
desktop is never touched. Screenshots are written to `/tmp/tuf-ui-test/`.

## Result: PASS (0 failures)

| Check | Result |
|---|---|
| Sidebar navigation: Home, Devices, Lighting & Aura Sync, Aura Wallpaper, Scenario Profiles, Macros, GameVisual, Content Platform, Feature Library, User Center, Settings | ✅ all pages render, app stays alive |
| Devices sub-tabs: System Configuration, Free up memory, GPU Performance, Lighting, Audio, Resource Monitor | ✅ |
| Aura Sync tabs: Sync devices, Aura Effects, AI Aura Lighting | ✅ |
| Settings tabs: General, Update Center, About | ✅ |
| Theme selection + persistence (ROG → `ui.json` contains `"Rog"`) | ✅ PASS |
| Light theme applied + persisted (readable light shell/cards/text) | ✅ PASS |
| Theme revert to TUF | ✅ |
| Effect tile click → audited RGB write (2722 → 2724 `kbd_rgb` entries) | ✅ PASS |
| Window resize 1600×900 and 1366×768 | ✅ app alive, layout usable |
| Splash animation frame deltas (`compare -metric RMSE`) | 0.040 / 0.009 — animation is evolving |

## Notes

- The RGB check writes one static frame through the verified daemon path; the audit log is
  the evidence (no hardware assumption beyond the already-verified channel).
- Splash frames are captured at ~0.45 s, 1.0 s and 1.55 s; non-zero deltas prove the logo
  entrance/glow/progress are animating.
- Page-transition fade is 200 ms; captured implicitly between navigation screenshots.
- Re-run: `scripts/ui-interaction-test.sh [out-dir]` (requires Xvfb, xdotool, ImageMagick).
