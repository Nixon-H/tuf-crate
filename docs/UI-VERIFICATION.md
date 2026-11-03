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
- dGPU Standard restore works; live Eco disable is blocked with an explanation (it froze the compositor once) ✅

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


## Addendum — daemon release (2026-10-31, later session)
- **Privileged daemon `tuf-crated`** added: GUI controls now work as the normal user
  (backend auto-switches to `daemon`; header tag `controls: daemon` — verified on-screen).
- **Live fan verification on the FX706HF** via the daemon's audited funnel:
  balanced ≈ 4000/3900 RPM → performance ≈ 4400/4400 RPM (+6–10 s) → reverted to balanced.
  Profile switching demonstrably drives fan speed through the firmware thermal policy.
- GUI restyled (dark + TUF-orange, mode cards, ring gauges) — verified by screenshot.
- Daemon protocol covered by 6 new tests (5 core + 12 hal + 6 daemon = 23 total).

## Addendum — Armoury-style shell redesign (2026-11-01)

The UI was compared against the owner's Windows Armoury Crate screenshots (Home, Aura Effects,
System Settings) extracted read-only from the Windows partition.

**Now matching Armoury's structure:**
- icon sidebar with reference labels (Playground / Assistant), regular-case group headers,
  warm translucent selected row with a thin accent rail, and bottom Feature Library/Settings
- page title in the regular UI face plus the reference header glyphs (ROG mark, keyboard,
  audio, battery with live tooltips)
- Home at the reference 1920×1000 rhythm: borderless hero with the exact Armoury product
  render, model/spec lines, tall outlined Devices (0) card with a dashed add tile, System
  Usage CPU/GPU/Fan/Memory blocks with dim placeholder rows, and three Operating Mode cards
  (Windows mode removed because it cannot work on Linux)
- Lighting & Aura: effect icon grid (11 original line icons) + right-hand detail panel with
  Pattern strip, Speed (33/67/100 %), Apply and Reset — mirroring Armoury's Aura Effects page

**Deliberate differences:**
- the original ASUS render/logo/font/theme previews are loaded from the local-only
  `assets/vendor-reference/` when present (they are proprietary and gitignored); the vector
  laptop is only a fallback
- no Macros, Game Library, Content Platform or Promotion sections — Windows-only services
  or no hardware meaning on this laptop; Aura Wallpaper is Linux-native
- effect icons are simplified line art; the single-zone keyboard rules out per-key previews
- the four-mode Operating Mode row is reduced to the three firmware-backed modes
- Windows UI font (Segoe UI) is not redistributable; the regular sans face is used

Verified headlessly (Xvfb) — see `docs/images/home-hero.png` and `docs/images/lighting-aura.png`.

## Addendum — responsive layout, live controls and dGPU safety (2026-11-02)

- The UI now normalises the X11 native scale to 1:1 physical pixels by default. Some
  sessions report 1.333×, which made the reference layout render 33 % too large, clip
  content and select the wrong responsive breakpoints. Settings exposes 75/100/125/150 %
  for users who want a larger interface.
- Every page resets its scroll position when switching pages/sub-tabs, so a scrolled
  System Settings page can no longer reappear as blank cards.
- Home fills a maximised 1920×1080 window: the Devices card grows and Operating Mode is
  pushed to the bottom; at 1366×768 the same content scrolls with a visible scrollbar.
- Card grids are width-driven (4/3/2 columns), the sidebar is fixed and the content area
  uses the full remaining width.
- System Configuration now exposes live control tiles (mode, keyboard brightness, panel
  overdrive, battery limit, lighting, audio, CPU cap, monitor) instead of dead
  "Windows-only" cards. Firmware features that genuinely cannot work are listed with the
  reason, without buttons.
- GameVisual applies reversible X11 gamma profiles and panel overdrive.
- Scenario Profiles, Aura Wallpaper, Feature Library and Settings were rebuilt with
  card-based layouts; wallpaper fit modes use native XFCE properties.
- Live fan history (60 s) replaces the fake "fan acoustics" estimate; CPU% is smoothed,
  CPU frequency is the maximum core clock, and fan RPM comes from the ASUS EC.
- **Safety:** live dGPU Eco power-off is blocked after it froze the compositor on this
  machine. Only restoring Standard is offered live; “Schedule Eco for next boot” writes the
  boot config, which the daemon applies before the display manager. docs/SAFETY.md,
  UNSUPPORTED.md and ASUS-LINUX-FEATURE-RESEARCH.md record the evidence.

## Addendum — model-aware shell and live header controls (2026-11-03)

- Header icons are interactive: ROG mark → Home, keyboard → Aura Effects, speaker → Audio,
  battery tile → System Configuration; each has a tooltip and pointing cursor.
- Sidebar hamburger collapses the navigation; a floating hamburger reopens it.
- Product family is read from DMI and used in the splash and Home title
  (e.g. `ASUS TUF Gaming F17 · FX706HF`). Hero assets resolve by detected model
  (`hero-device/<MODEL>.png`); other models get the vector fallback instead of a wrong
  FX706HF render.
- Home hero is 37 % larger (405 px wide at 1920) while caption positions stay fixed.

## Addendum — Devices and Aura UI pass (2026-11-02, later)

- System Configuration: four large control cards (Performance, Keyboard, Power & battery,
  Display & audio) with live values and direct actions instead of a dense tile grid.
- Lighting: full-width hero card (live preview + effect/speed/brightness/apply) and a
  6/4/3-column effect grid with an Off card; expanded colour mixer with preview.
- Audio: split into a large output card (volume, mute, default sink) and a routing card;
  PipeWire inventory is a full-width card.
- Free up memory: memory/CPU/selection summary cards and a striped process table card.
- GPU Performance: telemetry cards, boot-time Eco schedule card, full-width mode cards and
  a GPU-process table card.
- Resource Monitor: eight metric cards (CPU/GPU/fans/memory + battery/storage/profile/model)
  and a striped process table.
- Aura Sync devices: a single large keyboard device card with live preview and a separate
  sync-status panel; dead Windows toggles replaced by an explicit “not available” note.
- Aura Effects: effect tiles spread across the free width; colour mixer enlarged with a
  current-colour preview/apply card.
- AI Aura: hero banner plus three working local-effect cards (Smart/Adaptive/Music).
- Control socket now works immediately after install (0666 local, verified ops only).

## Addendum — progressive-disclosure cleanup (2026-11-02)

- Technical explanation blocks in Devices are now hidden behind small **(i)** tooltips
  (operating modes, CPU cap, persistence, fans, battery, display/GPU, inventory, Aura).
- Advanced sections are elevated cards (`#242424`, 1px border, subtle shadow, 16px padding)
  with the chevron on the far right; Performance & power opens by default, the rest collapse.
- Operating modes are a three-zone segmented control with large icons; the active profile
  gets the solid orange treatment and a short description.
- Hardware inventory now uses four circular gauges (CPU/GPU/fan RPM) plus a two-column
  sensor grid; the duplicate raw list and thermal bars are gone.
- Free up memory groups identical process names into collapsible rows, right-aligns CPU/RAM
  and uses low-opacity zebra striping. Group checkboxes select/clear every PID in the group.
- Palette applied: page `#161616`, cards `#242424`, accent `#FF6B00`, primary text
  `#FFFFFF`, secondary `#A0A0A0`, warnings `#FF4444`.
- The Free up memory explanatory sentence is now an (i) tooltip; the page starts directly
  with summary cards, so every explanatory paragraph in the Devices section is behind an
  info icon. Info icons also open a pinned popup when clicked (Close or click again).
- Device controls are a two-column bento of large cards, each split into identity/value and
  controls, with a subtle animated accent border on hover.
- Resource Monitor uses the same grouped, zebra-striped, right-aligned process table as
  Free up memory (groups expand by clicking).

## Addendum — hotkeys, backlight timeout and flicker-free dimming (2026-11-03)

- The app reads the kernel `Asus WMI hotkeys` event device read-only. Fn+F5 cycles
  Silent/Balanced/Turbo; keyboard-illumination up/down adjust the LED brightness.
  Verified by injecting KEY_KBDILLUMUP (audited brightness write) and KEY_FN_F5
  (profile write; the active game rule immediately re-applied performance).
- Optional backlight timeout (Settings → Keyboard & hotkeys) tracks system-wide
  keyboard/mouse activity read-only, writes brightness 0 after the timeout and restores
  the previous level on activity.
- GameVisual adds software flicker-free dimming through XRandR brightness while leaving
  the panel backlight unchanged; gamma is re-applied after mode changes. It cannot alter
  the panel's PWM frequency.
- Device info now reports whether a MUX or `asus_custom_fan_curve` node is present, so
  ROG models with those interfaces are detected even though this TUF does not have them.

## Addendum — full UI audit, responsive fit and animation pass (2026-11-03)

- Full findings/fix table: [`docs/UI-AUDIT-2026-11.md`](UI-AUDIT-2026-11.md).
- Home now fits 1920×1000 and 1366×768 without clipped cards; Devices/Operating
  Mode share a baseline, the placeholder card reflows in narrow columns and the
  right column reserves the scrollbar gutter.
- Selection styling is opaque neutral `#2C2C31` with the accent reserved for
  borders/rails/text, eliminating the muddy accent wash across nav, segments and
  badges.
- One transient toast at the top centre replaces the duplicate bottom status bar;
  only a persistent telemetry failure keeps a bottom strip.
- System Settings uses segmented controls for performance, brightness, charge
  limit and panel overdrive; primary/secondary/text action tokens are shared.
- Resource Monitor restores the strict palette and adds an auto-scaled 60 s fan
  history graph; the fan curve visibly varies (about 4950–5450 RPM on this EC).
- Audio's raw `wpctl` dump is behind a collapsed card; Update Center and About
  were rebuilt as summarised cards with progressive disclosure and no dead
  updater button.
- Fixed portable-font boxes (`→ ● ✕ ＋ 🎨 💾`), over-bright zebra rows, theme
  caption duplication and the green "no X11 output" status.
- Verified: 41 workspace tests pass, clippy clean, interaction suite failures=0,
  theme persistence and audited RGB write pass, header controls and sidebar
  collapse/reopen checked.

## Addendum — robustness/safety/performance audit (2026-11-03, second pass)

- Findings 24–40 are documented in [`docs/UI-AUDIT-2026-11.md`](UI-AUDIT-2026-11.md):
  PID-reuse-safe process termination, cached `/proc`/`wpctl` scans, asynchronous apt
  checks, reconnectable hotkey listeners, bounded audit log, atomic settings/profile/
  boot/game persistence, real Follow System detection, model-aware CPU/GPU specs,
  game-vs-AC profile priority, Turbo cap restoration, and minimum-window fixes.
- Minimum-window reflow verified at 1100×680, including live resize from 1920×1000.
- Info tooltips normalise whitespace and dismiss on outside click; Escape is handled
  in the global input path.
- Tests: 41 workspace tests pass, clippy clean, interaction suite failures=0.

## Addendum — lifecycle, display state and daemon hardening (2026-11-03, third pass)

- Findings 41–51 are in [`docs/UI-AUDIT-2026-11.md`](UI-AUDIT-2026-11.md).
- Backlight restores on window close if the inactivity timeout had dimmed it; manual
  brightness changes cancel the timeout restore.
- GameVisual gamma/dim and the last applied keyboard RGB state persist in `ui.json`;
  GameVisual restores the saved profile at startup. No RGB write is made on launch.
- Boot-config dGPU-off is skipped when a display manager/X11/Wayland session is already
  running, preventing the live-restart freeze hazard.
- Daemon connections have read timeouts and a request-size cap; GUI writes have 3 s
  timeouts. The systemd unit is sandboxed and `systemd-analyze verify` is clean.
- Internal eDP/LVDS/DSI outputs are preferred over external monitors for gamma/dimming.
- Verified: 42 tests pass, clippy clean, interaction suite failures=0, persisted
  `ui.json` schema includes display and RGB state.

## Addendum — eleven-pass audit program (2026-11-03)

- The complete pass index and prompt-to-artifact checklist are in
  [`docs/AUDIT-PROGRAM.md`](AUDIT-PROGRAM.md).
- Additional rounds covered UI consistency, 75–150% DPI / 1100×680 reflow, control
  behaviour, restart persistence, monitoring edge cases, safety, performance/code
  quality and accessibility/asset resolution.
- New evidence includes: narrow viewport auto-collapse + overlay drawer, persisted
  GameVisual/RGB/device-image state, N/A nvidia-smi rejection, bounded nvidia-smi,
  runtime `dgpu_disable=skipped-live-session`, hardened unit syntax, focus rings and
  install-relative local asset lookup.
- Final gates: **42 tests pass**, clippy clean, interaction suite `failures=0`.

## Addendum — final live daemon + keyboard-focus verification (2026-11-03)

- Ran the privileged installer successfully (`exit=0`): daemon reinstalled, unit reloaded,
  and `tuf-crated` restarted. Installed `/usr/local/bin/tuf-crated` SHA-256 matches the
  release build; the sandboxed unit is active.
- Re-ran `scripts/ui-interaction-test.sh` against the restarted daemon: **failures=0**
  (ROG/Light/Follow-System persistence, audited RGB write, header/sidebar/min-size checks,
  splash deltas).
- Keyboard-focus pass under Xvfb: `windowfocus` + Tab showed the orange focus ring on the
  sidebar Home/Devices rows, and Enter activated Devices/navigated to System Settings.
  Segmented controls accept `[` / `]` to move and Enter/Space to apply while focused
  (arrow keys remain reserved by egui for focus navigation).

## Addendum — reported device/alignment/refresh detail pass (2026-11-03)

- Home now auto-detects the laptop as a device: `Devices (1)` with family name,
  `Notebook · Chassis ID FX706HF`, BIOS and Connected status; the old “No Devices
  Connected” placeholder remains only for an undetected model.
- Fixed the reported control gaps: 10 px keyboard swatch→label and wallpaper
  style-row→folder-button separation.
- Operating Mode icon/name/description spacing is now compact in tall, medium
  (1366×768) and short windows; advanced mode cards were tightened too.
- Refresh rate: responsive card width, all supported rates visible with wrapping,
  equal-height GPU/panel/refresh row, neutral active fill, and a corrected parser
  for real `xrandr` rows such as `144.03*+` (the previous UI showed `-- Hz`).
- Final installed GUI matches the release binary; daemon restarted at 16:19:30
  and is active. Tests: **42 passed**, clippy clean, interaction suite failures=0.

## Addendum — final reported-issues verification (2026-11-03)

- Detected device: Home shows `Devices (1)`, family name, Notebook/Chassis ID,
  user-local chassis serial, BIOS and Connected state. Serial is written by the
  installer with mode 0600 and is never committed or sent anywhere.
- Verified the four reported visual defects in captures: Operating Mode icon/text
  spacing, Keyboard swatch→Static gap, Wallpaper style-row→folder-button gap, and
  the complete Refresh-rate list.
- Refresh parser now handles real indented `xrandr` rows and `144.03*+` tokens; a
  regression test covers it.
- Final installed build: GUI/daemon hashes match, daemon restarted at 16:57 and
  active; interaction suite failures=0; **42 tests pass**, clippy clean.
