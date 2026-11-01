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

## The hero laptop image — EXACT Armoury asset recovered

The Home hero is **not** in the UWP package's `Assets` tree: Armoury downloads it at runtime
into its UWP application data. That cache was found during this analysis:

```
C:\Users\Aditya\AppData\Local\Packages\B9ECED6F.ArmouryCrate_qmba6cd70vzyy\
  LocalState\ProductPNG\
    90NR0HC4-M003D0.png       2400×2400  RGBA  (source product image)
    90NR0HC4-M003D0_crop.png  2160×1502  RGBA  (what the Home page displays)
```

`90NR0HC4-M003D0` is the machine's ASUS part number. The cropped PNG is the **exact image
shown on the Armoury Home screen** (front-facing TUF F17, TUF GAMING screen wallpaper, orange
keyboard). SHA-256:

- `90NR0HC4-M003D0.png` — `08a7979658e3e81c19a4e832a6a1c681b057b7f48243b1829d8d3064a7d996ee`
- `90NR0HC4-M003D0_crop.png` — `412fc84360b616550c7886a895a2df51b0749676780389eaf45ea7e6c34c3d9b`

Stored local-only in `assets/vendor-reference/hero-device/` (gitignored). The application
copies the crop to `FX706HF.png` for the model-auto-detection loader; the original files are
kept alongside for provenance. `scripts/fetch-hero.sh` downloads official ASUS product
renders as a fallback and can be replaced by the ProductPNG extraction on a machine that has
Armoury Crate installed.
