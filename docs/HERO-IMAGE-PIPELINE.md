# Home hero — how Armoury Crate gets the device render, and how tuf-crate recreates it

## What Armoury Crate actually does (evidence from the owner's Windows install)

Armoury does **not** ship the Home laptop render in its package. It downloads it at
runtime and caches it in the UWP app data:

```
%LOCALAPPDATA%\Packages\B9ECED6F.ArmouryCrate_qmba6cd70vzyy\LocalState\ProductPNG\
    90NR0HC4-M003D0.png        2400×2400 RGBA   (source product render)
    90NR0HC4-M003D0_crop.png   2160×1502 RGBA   (what the Home page displays)
```

- The file name is the machine's **ASUS part number** (`90NR0HC4-M003D0`), not the
  marketing model name — the cloud lookup is keyed by PN.
- The UI exposes a **Reload Device Image** action; when the network fails it reports
  “Failed to Reload Device Image … check your network connection” (strings recovered
  from the package's `resources.pri`). Full evidence: `archive/docs/WINDOWS-ASSET-INVENTORY.md`.
- **The exact cloud endpoint was recovered** from the UWP managed assembly
  (`ArmouryCrate.dll`, string table around `ProductImageInfo`, `ProductPNG`,
  `DefaultProductImageUrlPrefix/Suffix`):

```
https://odinapi.asus.com/apiv2/GetPDInfoByMultiplePID
    ?SystemCode=asus&WebsiteCode=<site>&partNumber=<PN>&ModelType=3
```

  Response (live, verified 2026-11-04):

```json
{"Status":"200","Message":"Success","Result":[{
  "ProductID":15630,
  "partNumber":"90NR0HC4-M003D0",
  "ProductMarketName":"2021 ASUS TUF Gaming F17",
  "ImagePath":"https://dlcdnwebimgs.asus.com/gain/51099666-2767-4410-99b8-cae7a7de8a95/"
}]}
```

  Downloading `ImagePath` yields a **2400×2400 RGBA PNG whose SHA-256 is
  `08a7979658e3e81c19a4e832a6a1c681b057b7f48243b1829d8d3064a7d996ee` — byte-identical
  to the `LocalState\ProductPNG\90NR0HC4-M003D0.png` Armoury had already
  downloaded on this machine.** That closes the loop: PN → Odin API → exact
  ProductPNG. Armoury then crops the square render (`GetCroppedBitmapAsync`,
  `GetColorFromPixel`) into the `_crop.png` it displays; tuf-crate shows the full
  square with contain-fit instead.
- A second ROG endpoint exists for ROG marketing data:
  `https://api-rog.asus.com/api/v1/RogInformation?PartNo=<PN>`.
- Part numbers are required by the API (model names return an empty `Result`).
  On Windows Armoury gets the PN from the ASUS service/WMI. On Linux the number
  lives in SMBIOS OEM strings (root-only) or `product_sku` (some models):
  the installer persists it to `~/.local/share/tuf-crate/part_number`; a user
  override (`~/.config/tuf-crate/part-number` or `TUF_CRATE_PART_NUMBER`) always
  wins.

## What tuf-crate does (same API, same cache behaviour)

1. **PN lookup** — `assets::part_number()` reads `TUF_CRATE_PART_NUMBER`, then
   `~/.config/tuf-crate/part-number`, then the installer-persisted
   `~/.local/share/tuf-crate/part_number`, then `/sys/class/dmi/id/product_sku`
   (first valid ASUS PN wins).
2. **Odin call** — the background fetch thread calls the recovered endpoint above,
   parses `Result[0].ImagePath` (must be on `dlcdnwebimgs.asus.com`), and downloads
   it. Validation: https-only, timeouts, 20 MB cap, PNG/JPEG magic, atomic write.
3. **Manifest fallback** — `hero-urls.json` (model token / `series:<key>` →
   HTTPS renders) covers machines without a readable part number and provides
   alternate angles; `series-*` files are skipped when the exact model render
   was just fetched.
4. **Cache-first loading** — `~/.local/share/tuf-crate/hero-cache/` (the
   `ProductPNG` equivalent) is consulted before the repo/vendor assets, then the
   model file, then the series file, then the procedural fallback.
5. **Reload Device Image** — a button under the Home hero force-refreshes the cache
   and hot-swaps the texture, reporting success/up-to-date/failure through toasts.
6. **Offline behaviour** — no local render and no network → neutral procedural
   laptop; failures surface as a non-blocking toast, never a hang.

### Coverage for all ASUS gaming models

Any machine where a PN is available (installer read of SMBIOS OEM strings, DMI
`product_sku`, or a one-line override) gets its **exact** official render through
Odin — TUF, ROG, Strix, Zephyrus, Flow, ProArt, TUF Dash, etc.; the API is keyed
purely by part number, so there is no per-model code. Machines without a readable
PN still fall back to the manifest (per-model or per-series photographic renders)
and then the neutral procedural laptop.

### Adding a model or series

Persist a PN (`~/.config/tuf-crate/part-number`), or append an entry to
`hero-urls.json`, or simply drop a `{MODEL}.png` / `series-{key}.png` into
`assets/vendor-reference/hero-device/` (or `~/.local/share/tuf-crate/vendor-reference/hero-device/`).
No code changes, no rebuild.

### Current coverage

| Key | Source |
|---|---|
| any model with a PN | Odin API → byte-identical Armoury ProductPNG render |
| `FX706HF` | verified: Odin `ImagePath` downloaded, SHA-256 equals the Armoury cache file |
| `series:tuf` | bundled TUF render + manifest URL — TUF machines without a PN |
| ROG / ProArt / Zenbook / VivoBook / ExpertBook / Chromebook | Odin covers them live with a PN; `series-rog.png` bundled; otherwise the neutral procedural laptop — no synthetic vector art is generated |
