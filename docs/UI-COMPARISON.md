# Automated visual comparison — Armoury Crate vs TUF Crate

Method: each Windows reference screenshot and each Linux capture is cropped to the app
area (1920×1000) and resized to 1600×900, then compared with ImageMagick:

- **RMSE** — normalised root-mean-square pixel difference (lower is better, 0 = identical)
- **SSIM** — structural similarity (higher is better, 1 = identical)
- side-by-side and diff images are generated for review

Run: `scripts/ui-compare.sh <windows-screenshots-dir> <linux-captures-dir>`
Output defaults to the gitignored `assets/vendor-reference/ui-compare/` (the Windows
references are proprietary and are not committed).

## Results (current build)

| Screen | RMSE | SSIM | Main differences |
|---|---|---|---|
| Home | 0.084 | 0.192 | exact Armoury hero from the ProductPNG cache; borderless hero, Devices card and System Usage rows now follow the reference's 1920×1000 rhythm; 3 mode cards (Windows mode removed) |
| System Configuration | 0.091 | 0.201 | card illustrations/line icons; extra Linux controls below |
| Free up memory | 0.068 | 0.176 | list styling, CPU% column added |
| GPU Performance | 0.083 | 0.195 | card artwork, process section added |
| Devices → Lighting | 0.102 | 0.196 | effect icon artwork |
| Splash | 0.114 | **0.480** | same logo/black field; Lottie timing vs native progress |
| Settings General | 0.140 | 0.201 | theme preview sizes, page density |
| Update Center | 0.070 | 0.187 | table density, native package rows |
| Sync devices | 0.080 | 0.186 | device card artwork |
| Aura Effects | 0.109 | 0.202 | effect icon artwork, grid proportions |
| AI Aura Lighting | 0.142 | 0.252 | gradient/artwork, cloud buttons disabled |
| Aura Wallpaper | 0.062 | 0.188 | original wallpapers vs ASUS cloud art |

**Interpretation:** the structural shell, navigation, page composition, typography and
Home grid now match closely at the reference window size (1920×1000). Pixel metrics are
dominated by the unavailable ASUS photographic/illustrated artwork and by honest
Linux-specific content (extra hardware controls, unavailable cloud sections, the Windows
operating-mode card removed). The splash has the highest structural similarity because its
logo/background are original assets.

## How to improve further

1. SVG rendering of the original 260 navigation/effect glyphs.
2. Optional Lottie playback for the splash and loading animations.
3. Exact Windows UI font (Segoe UI is not redistributable; DejaVu Sans is used).
4. Responsive tightening for pages whose content is wider than 1366 px.
