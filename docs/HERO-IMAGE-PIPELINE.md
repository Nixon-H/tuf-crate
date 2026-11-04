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
- The service plugins talk to ASUS through a local framework HTTP layer
  (`SOFTWARE\WOW6432Node\ASUS\ArmouryHttp`, `GetFrameWorkHttpPort`) — the actual
  cloud endpoint and request signing live in UWP managed code that was never
  extracted from the package, so the **private API itself is not recoverable** from
  the captured evidence. The old public ASUS support ASMX endpoints are dead
  (HTTP 500 / redirect to HTML), and the current asus.com search is client-side.

Verified sources for the recreation (all live-tested):

| ASUS public source | Result |
|---|---|
| `dlcdnwebimgs.asus.com/files/media/<GUID>/…/design_pd_0N.png` | detail crops (lid, keyboard deck, keys, display) — not a hero |
| `dlcdnwebimgs.asus.com/gain/<GUID>` (full size) | **2400×2400 front-facing product renders** — the same class as ProductPNG |

## What tuf-crate recreates

Faithful to the mechanism, using public infrastructure instead of Armoury's private API:

1. **Manifest-driven lookup** — `assets/vendor-reference/hero-device/hero-urls.json`
   maps the DMI model token (`FX706HF`) and `series:<key>` (`series:tuf`) to one or
   more HTTPS URLs, with kinds `png` / `alt` / `line`.
2. **Runtime download** — on startup, if the exact model render is not present in
   the cache or asset dirs, a **background thread** (never the UI thread) downloads
   the sources with `curl`:
   - https-only, 20 s timeout, 20 MB cap
   - PNG/JPEG magic validated before use
   - atomic `.part` → final rename into the cache
3. **Cache-first loading** — `~/.local/share/tuf-crate/hero-cache/` (the
   `ProductPNG` equivalent) is consulted before the repo/vendor assets, then the
   model file, then the series file, then the procedural fallback.
4. **Reload Device Image** — a button under the Home hero force-refreshes the cache
   and hot-swaps the texture, reporting success/failure through the toast system.
5. **Offline behaviour** — no local render and no network → the neutral procedural
   laptop; failures surface as a non-blocking toast, never a hang.

### Adding a model or series

Append an entry to `hero-urls.json` (model token or `series:<key>`) with URLs of an
official ASUS render, or simply drop a `{MODEL}.png` / `series-{key}.png` into
`assets/vendor-reference/hero-device/` (or `~/.local/share/tuf-crate/vendor-reference/hero-device/`).
No code changes, no rebuild.

### Current coverage

| Key | Source |
|---|---|
| `FX706HF` | official TUF F17 front render (gain GUID) + alt angle |
| `series:tuf` | same TUF front render — any TUF machine without its own file |
| ROG / ProArt / Zenbook / VivoBook / ExpertBook / Chromebook | bundled series images where a real photographic render exists (`series-rog.png`); otherwise the neutral procedural laptop until a genuine render is dropped in — no synthetic vector art is generated |
