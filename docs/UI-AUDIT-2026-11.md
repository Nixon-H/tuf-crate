# UI/UX audit — 2026-11-03

Scope: the Armoury-style shell in `crates/tuf-gui`, checked against the local
Windows Armoury Crate reference at 1920×1000/1080 and 1366×768, in dark and light
themes, with the sidebar expanded/collapsed. No hardware write surface was added
or widened during this pass.

## Findings and fixes

| # | Finding | Fix | Verification |
|---|---|---|---|
| 1 | Transient action results appeared twice: a bottom status bar and a toast. | Removed the transient status bar; only a persistent telemetry failure keeps a bottom strip. Actions use one toast. | 7 s and 10 s captures; no duplicate line. |
| 2 | The toast covered the Operating Mode cards at the bottom right. | Toast moved to the free top-centre strip, slides/fades in, auto-dismisses after 2.6 s. | Screenshots + timer. |
| 3 | Home was clipped at 1000 px: Devices and Operating Mode were cut off. | Hero slot reduced to 245 px, right-column gaps tightened, bottom cards fixed at 160 px, no forced "extra" spacer. | 1920×1000 and 1920×1080 captures: headers at the same baseline, cards fully visible. |
| 4 | Home overflowed at 1366×768 (short-window regression). | Added a compact layout (<820 px available height): smaller hero, tighter usage blocks, 96 px mode cards, 160 px device card. | 1366×768 capture: all cards and labels visible. |
| 5 | Right-aligned Home values ran under the overlay scrollbar at 1366. | Reserved 18 px for the scrollbar in the Home width calculation. | 1366×768: `Power Saving` and memory values clear of the scrollbar. |
| 6 | Device placeholder copy was clipped at compact widths. | Below 440 px card width the plus tile stacks above centred copy instead of sharing a row. | 1366×768 capture. |
| 7 | Sidebar collapse clipped labels mid-word. | Sidebar content now fades with the width animation; left page gutter is stable at 47 px expanded / 32 px collapsed. | Mid-animation capture. |
| 8 | Accent-alpha selected fills rendered as a muddy brown wash (egui blends low alpha linearly). | Selected surfaces use opaque neutral `#2C2C31`; accent remains as a border/rail/text. Applied to nav, selection visuals, segments and badges. | Pixel checks (`srgb(44,44,49)`) on Home and Aura. |
| 9 | Theme cards had uneven heights and duplicated embedded captions. | Uniform 148 px cards, artwork cropped above a dedicated 26 px caption bar, light-theme caption variant. | 1920 and light-theme captures. |
| 10 | Theme row stopped at 1570 px on a 1920 window. | Card width ceiling raised to 260 px so six cards fill the row. | Capture: row ends at the content margin. |
| 11 | CPU frequency flickered every second (spiky max-core clock). | 10-sample trimmed mean plus a slow EMA, displayed rounded to 10 MHz. | Live readings stay within a stable band while load changes. |
| 12 | System Settings used three stacked full-width mode rows instead of a segmented control. | Shared 30 px segmented control (Silent/Balanced/Turbo, 80/100, On/Off, brightness) with hover animation and orange selected border. | 1920 and 1366 captures; interaction test clicks a segment. |
| 13 | `Restore 65 W cap`, `Open Aura Effects`, `Audio`, `Monitor` looked like orphan/dead buttons. | Replaced with quiet text actions; primary/secondary button tokens added. | Captures. |
| 14 | Info strip wrapped inside values at 1366 (`MUX: not / present`). | Replaced wrapping labels with an atomic 7/4-column grid. | 1366×768 capture. |
| 15 | Missing glyph boxes: `→`, `●`, `✕`, `＋`, `🎨`, `💾`. | Removed non-portable glyphs; delete crosses are painter-drawn; arrows are ASCII. | Visual sweep of every page. |
| 16 | Process-table zebra rows were far too bright (low-alpha white blended in linear space). | Zebra is now an explicit `#1D1D20`; child rows `#19191C`. | Pixel check + captures. |
| 17 | Resource Monitor used red/green/blue/orange values. | Values are white; only CPU ≥90 °C turns warning red; semantics stay in labels. | Capture. |
| 18 | Fan RPM looked static, and a fixed 0–6000 graph would hide 100 RPM changes. | Added a 60 s auto-scaled CPU/GPU fan history graph with current RPM in the legend. | Graph shows 4950–5450 RPM variation from the EC. |
| 19 | Audio page exposed a raw `wpctl status` wall. | Raw graph is behind a collapsed `PipeWire device inventory` card; volume slider widened and accent-filled; mute uses the secondary token. | Capture; collapse toggle checked. |
| 20 | Lighting apply buttons were army-green and speed/brightness were tiny text labels. | Primary apply is accent orange; speed and brightness are segmented controls; reset is secondary. | Capture; audited RGB write still passes. |
| 21 | Update Center and About were plain text walls with dead `Update All`. | Update Center has an app summary, component grid, update-policy card and read-only checker; About has capability chips, two collapsible parity cards and a diagnostics card. Removed the dead update button. | Captures; checker remains a read-only `apt list --upgradable`. |
| 22 | Scenario delete affordance was a box glyph; Add rule used `＋`. | Delete is a drawn hover cross; Add rule is a primary action. | Capture. |
| 23 | GameVisual reported "no X11 output" in green; dimming slider was almost invisible. | Status is muted when no output exists; dimming slider uses an accent fill and a clear percentage. | Capture. |

## Interaction and safety checks

- `cargo test --workspace`: **40 passed, 0 failed** (5 core, 9 + 10 HAL,
  16 daemon fixture tests).
- `cargo clippy --workspace --all-targets`: **clean**.
- `scripts/ui-interaction-test.sh`: **failures=0**; theme persistence (ROG and
  Light), splash frame deltas, sidebar/page navigation and a real audited
  `kbd_rgb` write all pass.
- Header controls were clicked individually: ROG mark → Home, keyboard → Aura
  Effects, speaker → Audio, battery → System Configuration.
- Sidebar collapse/reopen and both theme variants were captured.
- No new sysfs paths, EC writes, ACPI calls or privileged operations were added.
  Segmented controls call the existing capability-gated backend methods.

## Remaining honest limits

These are hardware/firmware limits, not missing UI wiring: custom fan curves and
PPT attributes are not exposed by this kernel/firmware, the panel has no MUX,
live dGPU power-off is unsafe on this EC (only the reboot-safe boot schedule is
offered), and ASUS cloud AI generation is a Windows service.

## Screenshot set

Captured under Xvfb at 1920×1000/1080 and 1366×768: Home, System Configuration,
Free up memory, GPU Performance, Lighting, Audio, Resource Monitor, Aura Sync /
Effects / AI Aura, Aura Wallpaper, Scenario Profiles, GameVisual, Feature
Library, Settings General / Update Center / About, sidebar mid/closed/open, dark
and light themes. Local-only proprietary hero/theme assets remain under the
gitignored `assets/vendor-reference/` tree.
