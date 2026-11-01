# Automated visual comparison — Armoury Crate vs TUF Crate

Method: each Windows reference screenshot is cropped to the app area (1920×1000) and each
Linux capture to the 1800×1000 app window; both are resized to 1600×900 and compared with
ImageMagick:

- **RMSE** — normalised root-mean-square pixel difference (lower is better, 0 = identical)
- **SSIM** — structural similarity (higher is better, 1 = identical)
- side-by-side and diff images are generated for review

Run: `scripts/ui-compare.sh <windows-screenshots-dir> <linux-captures-dir>`
Output defaults to the gitignored `assets/vendor-reference/ui-compare/` (the Windows
references are proprietary and are not committed).

## Results (current build)

| Screen | RMSE | SSIM | Main differences |
|---|---|---|---|
| Home | 0.113 | 0.226 | hero is an original vector render (ASUS photo unavailable); right column proportions |
| System Configuration | 0.099 | 0.220 | card illustrations/line icons; extra Linux controls below |
| Free up memory | 0.076 | 0.191 | list styling, no CPU% column |
| GPU Performance | 0.090 | 0.214 | card artwork, no process section |
| Devices → Lighting | 0.108 | 0.215 | effect icon artwork |
| Splash | 0.109 | **0.481** | same logo/black field; Lottie timing vs native progress |
| Settings General | 0.145 | 0.216 | theme preview sizes, page density |
| Update Center | 0.077 | 0.202 | table density, native package rows |
| Sync devices | 0.089 | 0.203 | device card artwork |
| Aura Effects | 0.117 | 0.221 | effect icon artwork, grid proportions |
| AI Aura Lighting | 0.148 | 0.269 | gradient/artwork, cloud buttons disabled |
| Aura Wallpaper | 0.074 | 0.212 | original wallpapers vs ASUS cloud art |

**Interpretation:** the structural shell, navigation, page composition and typography now
match closely; pixel metrics are dominated by the unavailable ASUS photographic/illustrated
artwork and by honest Linux-specific content (extra hardware controls, unavailable cloud
sections). The splash has the highest similarity because its logo/background are original
assets.

## How to improve further

1. SVG rendering of the original 260 navigation/effect glyphs.
2. Optional Lottie playback for the splash and loading animations.
3. More precise spacing/type-scale measurements from the reference (font sizes, card
   paddings) and responsive passes at 1600×900 / 1366×768.
4. Theme tokens across every card (beyond the accent token implemented now).
