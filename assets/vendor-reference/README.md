# Vendor reference assets — LOCAL ONLY (gitignored)

Extracted read-only from the owner's Windows installation on 2026-11-01 for visual
reference and local use while recreating Armoury Crate. **These files are proprietary
ASUS assets and are NOT redistributed**: the directory is excluded from Git
(`assets/vendor-reference/`), and the application loads them only if present locally.

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
