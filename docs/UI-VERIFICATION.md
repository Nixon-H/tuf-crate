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
