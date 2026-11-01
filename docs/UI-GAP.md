# Armoury Crate → TUF Crate — screen inventory and visual gap analysis

Reference: the owner's 16 Windows screenshots (1919×1079), deduplicated to 12 UI states.
Linux captures: 1800×1000 window on 1920×1080 (Xvfb), original assets/fonts loaded.

Legend: ✅ matched · 🟡 approximate · ⛔ unavailable/intentionally closed

## 1. Home — `Screenshot_2026-10-31_055240/055253`
| Windows element | TUF Crate | Status |
|---|---|---|
| Tall left product card, TUF F17 render | Left product card with the **exact Armoury hero** recovered from `LocalState\ProductPNG\90NR0HC4-M003D0_crop.png`; model auto-detected | ✅ |
| Model + CPU/GPU spec lines | Same lines, populated | ✅ |
| Devices (0) card, dashed +, View All | Recreated, click → Devices | ✅ |
| System Usage CPU/GPU/Fan/Memory with orange bars | Recreated with live telemetry; unavailable values shown as “—” | ✅ |
| Operating Mode (Silent/Performance/Turbo) icons | Three icon cards mapped to kernel `platform_profile`; Windows mode removed because it cannot work on Linux | ✅ |
| Top bar: title, Aura, battery, profile | Recreated (Aura shortcut, battery icon, profile chip) | ✅ |
| Original display font | ROG font loaded locally for headings/brand | ✅ |

## 2. System Settings → System Configuration — `055321/055332`
Feature cards (Win Key, Touch Pad, Modern Standby, Panel Power Saver, Power mode sync) are
shown with **honest Windows-only labels**; the Linux hardware controls (profiles, PL1,
fans, battery, display/GPU, inventory) live in collapsible sections on the same page. 🟡
(original cards are photographic/illustrated; ours use line icons).

## 3. System Settings → Free up memory — `055340`
Process list with checkboxes, CPU%, RAM, refresh, confirmation gate, SIGTERM to selected
**user-owned, non-critical** processes (system/session denylist). ✅

## 4. System Settings → GPU Performance — `055349`
Three cards: Standard (dGPU enabled), Eco Mode (dGPU disabled, confirmation), Optimized
(**unsupported — no MUX switch**); live `nvidia-smi` GPU process list. ✅

## 4b. System Settings → Audio
Default PipeWire output volume slider + mute (native `wpctl`, reversible) and a read-only
device inventory. TV/audio enhancements are desktop-owned. ✅

## 5. System Settings → Lighting — `055358`
Reused full Lighting page (effect grid + right panel + colour mixer + software effects +
brightness). ✅

 — `060313/060317`
Native splash with original AC logo texture, glow, rotating ring, fade and progress line.
The Lottie JSON source is inventoried; a Lottie renderer is not embedded. 🟡

## 7. Settings → General — `063849`
Launch-page dropdown, theme cards using the **original ASUS theme preview images**
(TUF/ROG/PRIME/ProArt/Light/Follow System), accent + egui visuals applied on selection,
persisted to `~/.config/tuf-crate/ui.json`; device-image style toggle and reload notice. 🟡
(theme now retints the shell, cards, text and meters; light mode verified).

## 8. Settings → Update Center — `063929/064041`
Native, read-only Debian state: app/kernel/Debian versions, `apt list --upgradable` check,
Update All disabled with guidance. **No package installation, no repositories, no
unverified scripts.** ⛔ in-app updating (by design); ✅ information surface.

## 9. Lighting & Aura Sync → Sync devices — `064317`
Single-zone device card with live keyboard preview and current effect summary; Aura
Performance Mode / smoothness / hue connection marked Windows-only. 🟡 (no multi-device
selection; none present).

## 10. Aura Effects — `064323`
11 firmware effect tiles with large line icons, right-aligned detail panel (Pattern, Speed
33/67/100, Apply, Reset), software effects (Music/Smart/Adaptive) and full 0–255 colour
mixer. ✅ structure; 🟡 icons are original line art (original SVGs inventoried but not
rendered — no SVG runtime).

## 11. AI Aura Lighting — `064331`
Gradient hero, headline, Create new / Lighting Gallery (disabled, ASUS cloud), honest
notice, and **local Smart/Adaptive alternatives** wired to real hardware. ✅ layout,
⛔ cloud generation.

## 12. Aura Wallpaper — `064402`
Gallery with four original redistributable wallpapers; selection; **native XFCE wallpaper
application via xfconf-query**; Windows-only activation/suspension/multi-display noted. 🟡
(ASUS wallpapers are cloud downloads; local copies were placeholders).

## 13. Dialog capture — `2026-11-01_075346`
A modal/diagnostic dialog; not recreated as a separate surface. ⛔ (content was a
Windows tooltip/log window, not an Armoury feature page).

## Remaining fidelity work
1. Apply the theme accent/background across every page (design tokens), not just the shell.
2. Optional SVG rendering for the original 260 nav icons and effect glyphs.
3. Splash: optional Lottie playback for exact keyframes.
4. Page/tab transition animations, hover lift on cards, toast notifications.
5. Free-up-memory CPU%, GPU process section, Update Center firmware listing.
6. Responsive passes at 1600×900 and 1366×768; automated screenshot comparisons.
