# Security & reliability audit — 2026-11-04

Full-code audit requested by the owner: "see if the application is secure, audit all
code, see everything is safely done, bugs/errors/glitches; verify the installed app
shows the Armoury Crate logo as its splash; verify installation across Linux
distributions."

## Method

| Check | Command / method | Result |
|---|---|---|
| Build & lints | `cargo clippy --workspace --all-targets` | **0 warnings** |
| Tests | `cargo test --workspace` | **51 passed, 0 failed** (5 core + 10 daemon + 18 GUI + 18 HAL) |
| Dependency CVEs | `cargo audit` | **0 vulnerabilities** (327 deps; 2 unmaintained-transitive warnings: `paste`, `ttf-parser`) |
| Memory safety | `grep -rn unsafe crates/*/src` | **0** `unsafe` blocks |
| Runtime panics | non-test `unwrap/expect/panic!` scan | 2 found, both fixed → **0** |
| Subprocess safety | 29 `Command::new` sites reviewed | all argument-array (no shell), fixed/timeout-bounded |
| Numeric casts | daemon `as u32` sites reviewed | fixed to `u32::try_from` (no wrap-around) |
| Write funnel | every hardware setter reviewed | capability gate + confirmation + clamp/quota + audit |
| Filesystem writes | `fs::write` sites reviewed | user config atomic (tmp+rename); daemon config atomic; cache atomic `.part` |
| Network fetch | hero pipeline reviewed | HTTPS only, ASUS-CDN allowlist for API results, 15–20 s timeouts, 20 MB post-check, image magic validation |
| Splash logo | installed binary launched from `/tmp` under Xvfb | **Armoury Crate logo renders** from `~/.local/share/tuf-crate/vendor-reference/branding/AC_Logo.png` |
| Install portability | installer reviewed & hardened | systemd optional, `--no-daemon`, getent/runuser fallbacks, dependency report |

## Findings fixed in this pass

| # | Severity | Finding | Fix |
|---|---|---|---|
| S1 | Medium | `set_kbd_rgb` passed caller values straight into the sysfs string with `format!`; a local process talking to the daemon socket could write arbitrary numeric strings (the kernel would reject, but the "clamped before write" guarantee was broken). | `validate_rgb_args()`: mode 0..=11, colour 0..=255, speed ∈ {0xe1,0xeb,0xf5}; out-of-domain input **fails closed** with `SafetyError::OutOfRange`. Unit-tested. |
| S2 | Low | Daemon parsed numbers with `as u32` wrapping casts (`2^32 + 0xeb` → `0xeb`). | `u32::try_from` for RGB, PL1, charge threshold, and the generic `value_u32` helper. |
| S3 | Low | `std::env::var("HOME").unwrap()` in `games.rs` / `profiles.rs` panicked when `HOME` is unset (containers, odd `systemd --user` setups). | XDG/HOME fallback to `/tmp`; no panic. |
| S4 | Low | `curl --max-filesize` is advisory for chunked responses; a huge response could reach the image decoder. | Post-download `metadata().len() <= 20 MiB` + regular-file check before the PNG/JPEG magic test. |
| S5 | Medium (portability) | Installer assumed systemd + Debian shadow-utils + `getent`; on non-systemd distros the daemon step failed late with a confusing error. | `--no-daemon` flag; systemd detection via `/run/systemd/system`; explicit non-systemd instructions; `getent`/`runuser` fallbacks; printed runtime-dependency list. |

## Accepted limits (documented, unchanged by design)

- **Control socket is mode 0666.** Any local user can issue the closed-enum,
  clamped, audited operations; there is no raw sysfs/EC path behind it. Group-based
  0660 was rejected because group membership does not apply to already-running
  sessions (the app would silently stop controlling hardware after install).
- **VERIFIED-hardware gate** (`is_fx706hf`) keeps model-specific writes off models
  that have not been verified; other attributes remain capability-gated at runtime.
- **ASUS cloud APIs** (`odinapi.asus.com`, `api-rog.asus.com`) are queried read-only
  for product imagery; responses are validated and only ASUS CDN hosts accepted.
- **Proprietary vendor assets** are committed at the owner's request; see
  `assets/vendor-reference/README.md` (copyright ASUS, reference-only).

## Distribution support

| Environment | GUI | Daemon (hardware control) |
|---|---|---|
| Debian/Ubuntu/Mint, Fedora, Arch, openSUSE, Manjaro, Pop!_OS (systemd) | ✅ | ✅ systemd unit, enabled + restarted by installer |
| Alpine/Artix/Void/Devuan (OpenRC/runit/s6/sysvinit) | ✅ | ✅ binary installed to `/usr/local/bin`; run it from the init system (`--no-daemon` installs GUI only) |
| NixOS | ⚠ out-of-box installer not supported (read-only `/usr`, no imperative units); build with a Nix derivation, run the daemon as a systemd service |
| WSL2 / containers | GUI needs an X server; daemon requires udev/sysfs access to the host hardware |

Build dependencies: Rust ≥ 1.75, a C toolchain, X11/OpenGL headers for `eframe`.
Runtime extras (all optional, distro packages): `xrandr`, `xfconf` (XFCE wallpaper),
`pipewire`+`wireplumber` (audio/music effect), ImageMagick `import` (adaptive colour),
`curl` (device-image download), `nvidia-smi`, `df`.

## Splash verification

The installed GUI was launched from `/tmp` (so no repo-relative asset path could
resolve) with the real user home; the splash frame captured at 0.9 s shows the
official Armoury Crate hexagon logo on the black field, device line
"ASUS TUF Gaming F17 · FX706HF" and the progress bar. Asset:
`~/.local/share/tuf-crate/vendor-reference/branding/AC_Logo.png` (copied by the
installer). Evidence capture: `/tmp/splash-live.png` (session-local).
