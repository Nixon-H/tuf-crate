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

## Second pass — robustness, safety and performance (2026-11-03)

| # | Finding | Fix | Verification |
|---|---|---|---|
| 24 | Free-up-memory could signal a reused PID if a selected process exited before the button was pressed. | Selection now stores `(comm, starttime)`; immediately before `SIGTERM` the app re-reads `/proc/<pid>` and skips any process whose identity changed. Deny-list matching is case-insensitive. | Identity code reviewed/compiled; all existing safety tests pass. |
| 25 | Free-up-memory rescanned every `/proc` entry on every repaint (egui repaints on hover). | Process snapshots are cached for 500 ms; CPU deltas are recomputed only on a fresh snapshot. | Capture and CPU reading. |
| 26 | Audio page ran `wpctl status` on every frame while dragging the volume slider. | Cached for 3 seconds; Refresh invalidates the cache. | Capture; no repeated process spawns. |
| 27 | “Check for updates” ran `apt list --upgradable` synchronously and froze the UI. | Runs on a worker thread; button shows `Checking...`; exit status/stderr is honoured and results are picked up without blocking. | Code path + build. |
| 28 | ASUS hotkey/activity listener threads died permanently if the event device was absent at startup or reconnected. | Both listeners retry device discovery/open/read with a 1–2 s backoff; hotkey lookup is model-name tolerant. | Build + listener review. |
| 29 | Audit JSONL grew without bound during software effects (up to ~12 RGB writes/s). | Daemon/core rotates `audit.jsonl` at 4 MiB and preserves the previous file as `audit.jsonl.1`. | Core tests pass; live log remains well below the threshold. |
| 30 | Boot config, UI settings, lighting profiles and game rules wrote in place (crash could truncate them). | All four now write a temporary file and atomically rename it. | Daemon/core/game/profile tests pass. |
| 31 | “Follow System” was cosmetic (always TUF dark). | Reads the XFCE XSETTINGS theme name and selects the light or dark surface variant, keeping the ASUS accent; falls back to dark when unavailable. | Code path; light/dark captures remain intact. |
| 32 | Home CPU/GPU specification lines, window title and Feature Library copy were hard-coded to the FX706HF. | CPU model comes from `/proc/cpuinfo`; dGPU name/memory from one read-only `nvidia-smi` query; title/feature copy use the detected family. | 1920 capture shows the detected Core i5 / RTX 2050 · 4 GB. |
| 33 | AC auto-switch could override an active game profile, because the watcher treats “same rule still running” as already applied. | AC switching is skipped while a game rule is active; automatic game profiles never rewrite PL1. | Code path. |
| 34 | Selecting Turbo did not restore a previously lowered 65 W sustained cap. | Explicit UI/hotkey Turbo selection restores the firmware-default 65 W cap (clamped, audited); automatic rules do not touch it. | Code path; description shown in Advanced controls. |
| 35 | Info popups had large runs of indentation whitespace and the Close button overlapped the last line; only the Close button dismissed them. | Tooltip text is whitespace-normalised, laid out with a real wrapping label, and closes on Escape or an outside click as well. | Outside-click capture; Escape handler added globally. |
| 36 | Minimum window (1100×680) clipped the Home product title/cards, overflowed the Aura preview column, and collapsed the wallpaper action card. | Short-window Home rhythm, smaller hero type, a shrinkable Aura split and a full-width wallpaper action card below the preview. | 1100×680 and live-resize captures. |
| 37 | Light theme had low-contrast header icons and success labels. | Theme-aware header icon colour and semantic success colour. | Light-theme capture. |
| 38 | When telemetry was unavailable, pages silently rendered blank. | Explicit “Telemetry unavailable / retrying” card; no control enabled. | Code path. |
| 39 | GPU mode cards had unequal heights at narrow widths. | Minimum card height added. | 1100×680 capture. |
| 40 | Profile names were unbounded and could contain control characters. | 48-character limit and control-character rejection; empty names already rejected. | Unit test extended. |

### Accepted local trust boundary

The control socket remains `0666` so controls work immediately after installation without a re-login. It is a local Unix socket, every operation is a closed-enum verified write that is clamped and audited, and no raw EC/ACPI path is exposed. Any local process can therefore request the same reversible actions the GUI can; a tighter group/ACL policy is a packaging decision, not a hardware capability.

### Second-pass verification

- `cargo test --workspace`: **40 passed, 0 failed**.
- `cargo clippy --workspace --all-targets`: **clean**.
- `scripts/ui-interaction-test.sh`: **failures=0**; audited RGB writes, theme persistence and splash deltas pass.
- Live resizing from 1920×1000 to 1100×680 and back reflows correctly.
- Info popup outside-click dismissal verified; Escape dismissal is implemented in the global input path (Xvfb has no window manager, so synthetic Escape focus is unreliable).

## Third pass — lifecycle, display state and daemon hardening (2026-11-03)

| # | Finding | Fix | Verification |
|---|---|---|---|
| 41 | Closing the window while the backlight timeout had dimmed the keyboard left it dark. | Restore the saved brightness on viewport close and in `Drop` as a fallback. | Code path; close lifecycle compiled. |
| 42 | GameVisual gamma/dim state lived only in the X server; after an app restart the UI showed Default/100 % while the display kept the old profile. | `display_mode` and `display_dim` persist in `ui.json`; the saved values are re-applied at startup and saved on every mode/dim change. | Schema check after the interaction suite. |
| 43 | Applied keyboard RGB mode/colour/speed reset to defaults in the UI after restart even though firmware retained them. | Persist last applied RGB state and restore it into the picker; never auto-writes on launch. | `ui.json` shows mode/color/speed. |
| 44 | A manual brightness choice while the timeout had darkened the keyboard could be overwritten by the timeout restore. | Any successful manual brightness change clears the timeout dim state; hotkey path already did this. | Code path. |
| 45 | A daemon restart while the desktop was live re-applied a persisted dGPU-off boot setting, risking the compositor freeze seen earlier. | Boot-config dGPU application is skipped when a display manager/X11/Wayland session is already active; retried at the next real boot. | Daemon tests pass; guard reviewed. |
| 46 | One idle or slow client could block the single-threaded daemon accept loop indefinitely; request lines were unbounded. | 5 s read timeout per connection and 64 KiB request-line limit. | Daemon protocol tests pass. |
| 47 | A hung daemon could freeze a GUI write request forever. | Client read/write timeouts (3 s) on every socket operation. | HAL tests pass. |
| 48 | Root daemon had no systemd sandboxing beyond defaults. | Added `NoNewPrivileges`, `PrivateTmp`, `ProtectHome`, kernel-module/log/cgroup protection, `AF_UNIX`-only sockets, namespace/personality/realtime/SUID restrictions and native syscall ABI. | `systemd-analyze verify` clean. |
| 49 | Display controls targeted the first connected output, which may be an external HDMI monitor rather than the laptop panel. | Prefer `eDP`/`LVDS`/`DSI` outputs, fall back to the first connected output. | Code path. |
| 50 | Scenario watcher and hotkey timeout sounded always-on although they only run while the app is open. | Copy now says “while tuf-crate is open/runs”; the daemon remains the only boot-time actor. | UI capture. |
| 51 | UI still claimed some watcher/backlight behaviour was global. | Scope wording corrected; game profiles explicitly documented as active while the window is open. | Copy review. |

### Accepted / documented limits

- Game-profile restore only runs while tuf-crate is open; a crash mid-game leaves the game profile applied until the next manual change. A background service would be required for always-on rules.
- The local socket remains `0666` for immediate use without re-login; it exposes only the closed verified-op enum, is bounded and audited, and never a raw EC/ACPI path.
- Daemon-side changes (dGPU guard, audit rotation, boot-config atomicity, service sandbox) take effect after the next `sudo scripts/install.sh` + service restart.

### Third-pass verification

- `cargo test --workspace`: **40 passed, 0 failed**.
- `cargo clippy --workspace --all-targets`: **clean**.
- `scripts/ui-interaction-test.sh`: **failures=0** (themes, splash deltas, audited RGB write).
- `systemd-analyze verify` on the hardened unit: **clean**.
- Persisted schema now includes `display_mode`, `display_dim`, `rgb_mode`, `rgb_color`, `rgb_speed`.
