# Windows Armoury Crate — asset & resource inventory

Read-only analysis of the owner's Windows 11 installation, performed on 2026-11-01 from
Debian using NTFS (TSK) access. **No files were modified; proprietary assets stay local.**

## Sources inspected

| Source | Path / evidence |
|---|---|
| UWP app package | `C:\Program Files\WindowsApps\B9ECED6F.ArmouryCrate_6.7.6.0_x64__qmba6cd70vzyy` (1,528 images, resources.pri, XAML resource index) |
| Hardware service | `C:\Program Files\ASUS\Armoury Crate Service\` (plugins, AuraPlugin sphere GIFs) |
| ProgramData | `C:\ProgramData\ASUS\` — Aura Wallpaper content, FestsEffect/PermanentEffect GIFs, `ArmouryCrate_v1.5.db` |
| Local app state | `C:\Users\Aditya\AppData\Local\Packages\B9ECED6F.ArmouryCrate_qmba6cd70vzyy\` (LocalState/LocalCache/…) |
| Screenshots | `C:\Users\Aditya\Pictures\Screenshots\Screenshot_2026-10-31_*` (16 Armoury UI captures) |

## Extracted reference set (local only, gitignored)

`assets/vendor-reference/` — 384 files, 24.3 MB, with `manifest/assets-manifest.json`
and `.csv` (SHA-256, bytes, dimensions per file):

| Category | Count | Examples |
|---|---|---|
| Navigation icons (TUF theme, SVG) | 260 | `ac_common_add_n.svg`, `ac_common_arrow …`, common glyph set |
| Animations | 80 | `AuraPlugin/Sphere/*.gif` (Aura effect sphere frames) |
| Lighting page assets | 18 | TUF AuraSync page SVGs/PNGs, AuraEffects backgrounds/tutorials |
| Theme previews | 8 | `ac_settings_img_tuftheme.png`, `…_rogtheme`, `…_asustheme`, `…_proarttheme`, `…_lighttheme`, `…_follow system`, `…_20th`, `…_kjptheme` |
| Loading / splash (Lottie JSON) | 5 | `CommonControl/Assets/{TUF,ASUS,ROG}/00_Animation/ac_common_splash.json`, `ac_common_loading.json`, `ac_common_small loading.json` |
| Branding | 5 | `AC Logo.png`, TXFigure frame/IP/top-logo layers |
| Fonts | 2 | `ROGFonts-Regular.ttf` (display font, 112 mapped glyphs), `Glyphter.ttf` (1 glyph) |
| Wallpapers | 0 usable | ProgramData wallpapers were **placeholder images** (grey broken-image icon), not real artwork — removed |

## The hero laptop image — Armoury's own render is cloud-only; official ASUS render recovered

The Armoury Home hero is **not a bundled asset**. The UWP resources contain only the
decorative `ac_homepage_device_lines_*.svg` overlay and device list badges; `resources.pri`
contains the runtime strings *“Reload Device Image”*, *“Failed to Reload Device Image”* and
*“Please check your network connection and try again to reload the device image.”* — the
render is fetched from ASUS's device-image service at runtime. No cached copy exists in the
package, LocalState/LocalCache, ProgramData, the internet cache or `ArmouryCrate_v1.5.db`.

**Recovered instead:** the official ASUS TUF Gaming F17 product renders from the ASUS
product CDN (design page assets), at 1718×1157 with transparency:

- `design_pd_01.png` → `assets/vendor-reference/hero-device/FX706HF.png`
- `design_pd_02.png` → `FX706HF_alt.png`
- `design_pd_03.png` → `FX706HF_keyboard.png`

These are genuine manufacturer renders of the same laptop, far closer to Armoury than a
generic illustration. They remain **local-only** (gitignored; ASUS marketing artwork, not
redistributed). `scripts/fetch-hero.sh` re-downloads them and prints SHA-256.

The application auto-detects the model from DMI (`/sys/class/dmi/id/board_name` →
`FX706HF`), loads `hero-device/FX706HF.png`, and falls back to the original vector laptop
only if the local asset is absent or the model is unknown.

## Fonts

`ROGFonts-Regular.ttf` is the Armoury display font. It is loaded **locally** at runtime
(gitignored) as a dedicated `rog` family for page titles, section headings, CPU/GPU/Fan/
Memory labels and the brand — body text uses the default sans, matching Armoury's mixed
typography. If the file is absent the family falls back to the default font.

## Splash / loading animation

The startup animation is a **Lottie JSON** (`ac_common_splash.json`, TUF/ASUS/ROG
variants). TUF Crate does not embed a Lottie runtime; the splash is recreated natively:
black field, original `AC Logo.png` texture (if present), orange glow, rotating accent
ring, fade-out and a thin progress line, using the same durations/ordering.

## Licensing

All extracted ASUS assets are proprietary and **excluded from Git**
(`.gitignore: assets/vendor-reference/`). They are used locally on the owner's machine
for visual fidelity. The redistributable originals created for this project
(`assets/wallpapers/*.png`, vector icons, vector laptop render) are committed under
`assets/` and `docs/images/`.
