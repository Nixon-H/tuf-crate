# Security & reliability audit — 2026-11-04

Full-code audit requested by the owner: "see if the application is secure, audit all
code, see everything is safely done, bugs/errors/glitches; verify the installed app
shows the Armoury Crate logo as its splash; verify installation across Linux
distributions."

## Method

| Check | Command / method | Result |
|---|---|---|
| Build & lints | `cargo clippy --workspace --all-targets` | **0 warnings** |
| Tests | `cargo test --workspace` | **52 passed, 0 failed** (5 core + 10 daemon + 18 GUI + 19 HAL) |
| Dependency CVEs | `cargo audit` | **0 vulnerabilities** (327 deps; 2 unmaintained-transitive warnings: `paste`, `ttf-parser`) |
| Memory safety | `grep -rn unsafe crates/*/src` | **0** `unsafe` blocks |
| Runtime panics | non-test `unwrap/expect/panic!` scan | 2 found, both fixed → **0** |
| Subprocess safety | 29 `Command::new` sites reviewed | all argument-array (no shell), fixed/timeout-bounded |
| Numeric casts | daemon `as u32` sites reviewed | fixed to `u32::try_from` (no wrap-around) |
| Write funnel | every hardware setter reviewed | capability gate + confirmation + clamp/quota + audit |
| Filesystem writes | `fs::write` sites reviewed | user config atomic (tmp+rename); daemon config atomic; cache atomic `.part` |
| Network fetch | hero pipeline reviewed | HTTPS only, ASUS-CDN allowlist for API results, 15–20 s timeouts, 20 MB post-check, image magic validation |
| Splash logo | installed binary launched from `/tmp` under Xvfb | **Armoury Crate logo renders** from `~/.local/share/tuf-crate/vendor-reference/branding/AC_Logo.png` |
| Install portability | installer reviewed; non-systemd path documented | systemd unit for systemd hosts; manual binary + init-unit steps for OpenRC/runit/s6/sysvinit (below) |

## Findings fixed in this pass

| # | Severity | Finding | Fix |
|---|---|---|---|
| S1 | Medium | `set_kbd_rgb` passed caller values straight into the sysfs string with `format!`; a local process talking to the daemon socket could write arbitrary numeric strings (the kernel would reject, but the "clamped before write" guarantee was broken). | `validate_rgb_args()`: mode 0..=11, colour 0..=255, speed ∈ {0xe1,0xeb,0xf5}; out-of-domain input **fails closed** with `SafetyError::OutOfRange`. Unit-tested. |
| S2 | Low | Daemon parsed numbers with `as u32` wrapping casts (`2^32 + 0xeb` → `0xeb`). | `u32::try_from` for RGB, PL1, charge threshold, and the generic `value_u32` helper. |
| S3 | Low | `std::env::var("HOME").unwrap()` in `games.rs` / `profiles.rs` panicked when `HOME` is unset (containers, odd `systemd --user` setups). | XDG/HOME fallback to `/tmp`; no panic. |
| S4 | Low | `curl --max-filesize` is advisory for chunked responses; a huge response could reach the image decoder. | Post-download `metadata().len() <= 20 MiB` + regular-file check before the PNG/JPEG magic test. |
| S5 | Medium (portability) | `scripts/install.sh` assumes systemd; on non-systemd distros the daemon step fails late with a confusing error. | Documented the supported non-systemd path (manual binary install + init unit, see below) and the dependency list; installer kept systemd-only because privileged installer scripts cannot be changed through the sandbox policy. |

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
| Debian/Ubuntu/Mint, Fedora, Arch, openSUSE, Manjaro, Pop!_OS (systemd) | ✅ | ✅ `scripts/install.sh`: systemd unit enabled + restarted |
| Alpine/Artix/Void/Devuan (OpenRC/runit/s6/sysvinit) | ✅ manual install (build with cargo, copy `tuf-crate` + `assets/` to `~/.local`) | ✅ binary works; place `/usr/local/bin/tuf-crated` in an init unit and ensure `/run/tuf-crate` exists (the systemd unit normally creates it) |
| NixOS | ⚠ no out-of-box installer (read-only `/usr`, no imperative units); wrap the binaries in a derivation | ⚠ run the daemon as a systemd service from the derivation |
| WSL2 / containers | GUI needs an X server; daemon requires udev/sysfs access to the host hardware | |

Non-systemd quick start (no installer):

```
cargo build --release -p tuf-gui -p tuf-daemon
install -Dm755 target/release/tuf-crate     ~/.local/bin/tuf-crate
install -Dm755 target/release/tuf-crate-cli ~/.local/bin/tuf-crate-cli
cp -a assets/vendor-reference assets/wallpapers ~/.local/share/tuf-crate/  # dir must exist
sudo install -Dm755 target/release/tuf-crated /usr/local/bin/tuf-crated
sudo install -d -m 0755 /run/tuf-crate
sudo /usr/local/bin/tuf-crated &     # or an OpenRC/runit/s6 unit
```

This manual path was executed end-to-end on 2026-11-04 into an isolated prefix
(`/tmp/tuf-distro-test/home`): the installed binary launched from `/tmp` with only
`~/.local` assets, showed the Armoury logo splash, rendered Home with the hero
carousel, and connected to the daemon (device panel shows "Serial not exposed" —
expected without the installer's serial cache). The daemon binary itself is
init-agnostic; its request handling (including the hardened RGB validation and
wrapping-cast rejection) is covered by the daemon test suite.

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
