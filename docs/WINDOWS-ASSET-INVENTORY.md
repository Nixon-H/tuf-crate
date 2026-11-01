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

## The hero laptop image — not recoverable from the package

The Home hero is **not a bundled asset**. The UWP resources contain only the decorative
`ac_homepage_device_lines_*.svg` overlay and the device list badges. `resources.pri`
contains the UI strings:

- `Reload Device Image`
- `Failed to Reload Device Image`
- `Please check your network connection and try again to reload the device image.`

i.e. the photographic render is **downloaded at runtime** from ASUS's device-image service.
No cached copy exists in the UWP package, `LocalState`, `LocalCache`, `ProgramData`
(Armoury service, Download Service, Aura Wallpaper), the internet cache, or the
`ArmouryCrate_v1.5.db`. Therefore:

- TUF Crate renders an **original vector laptop** (metallic deck, gradient screen,
  TUF-style lettering, live RGB keyboard) in the same composition.
- The substitution is documented in the About → parity list and `docs/SCOPE.md`.
- The extraction workflow stores any future recovered image in
  `assets/vendor-reference/hero-device/` (local) and the app prefers it if present.

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
