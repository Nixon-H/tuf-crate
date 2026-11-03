# Vendor reference assets — Armoury Crate resources (copyright ASUS)

Extracted read-only from the owner's Windows installation on 2026-11-01 for visual
reference while recreating Armoury Crate. **These files are proprietary ASUS assets**;
they are committed here at the owner's explicit request as reference material for this
educational recreation and are **not licensed for redistribution** — respect ASUS's
terms if you reuse anything in this directory.

### Hero-device series files (auto-picked by detected model)

`crates/tuf-gui/src/assets.rs` resolves the Home hero at runtime as:

1. `{DMI_MODEL}.png` / `_alt.png` / `_line.png` — exact model render (e.g. `FX706HF.png`)
2. `series-{KEY}.png` / `_alt.png` — series render, where KEY comes from the DMI
   family string: `tuf`, `rog`, `proart`, `zenbook`, `vivobook`, `expertbook`, `chromebook`
3. neutral procedural laptop (no wrong-brand artwork)

Present series files (all photographic product renders — no vector art):
- `series-tuf.png` / `series-tuf_alt.png` — from the TUF F17 vendor renders
- `series-rog.png` — the ROG device photo from the Armoury resources
- other series: drop a real photographic render named `series-<key>.png`; until then
  those machines show the neutral procedural laptop.

Sources:
- UWP package `C:\Program Files\WindowsApps\B9ECED6F.ArmouryCrate_6.7.6.0_x64__qmba6cd70vzyy`
  (navigation icons, theme previews, splash/loading Lottie JSON, lighting page assets)
- `C:\Program Files\ASUS\Armoury Crate Service` (AuraPlugin sphere GIFs, AC logo)
- `C:\ProgramData\ASUS\ROG Live Service\DeviceContent\Aura Wallpaper` (wallpaper samples)
- `C:\ProgramData\ASUS\ARMOURY CRATE Service\ArmouryCrate_v1.5.db` (inventory evidence)

Not recovered: the **Home hero laptop render** is not a bundled asset — the UI fetches it
at runtime ("Reload Device Image", "Failed to Reload Device Image … check your network
connection" strings in `resources.pri`), and no cached copy exists in the package,
LocalState, LocalCache, ProgramData or the internet cache. Evidence recorded in
`docs/WINDOWS-ASSET-INVENTORY.md`. TUF Crate therefore renders an original vector laptop
and documents the substitution.

See `manifest/assets-manifest.json` / `.csv` for SHA-256, sizes and dimensions.
