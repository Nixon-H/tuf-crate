# Original-objective compliance matrix — requirement → delivered evidence

Maps every explicit requirement of the original goal ("Reverse Engineer and Recreate ASUS
Armoury Crate for Linux") to its concrete artifact and verification status.

## Phases
| Phase (original §7) | Requirement | Delivered | Evidence |
|---|---|---|---|
| 1 Research | RE Armoury Crate functionality on this model | `archive/research/acpi/DSDT.dsl` (71,010 L decompiled), `archive/research/acpi/WMNB.dsl` (1,257 L dispatcher), `archive/research/kernel/*` (current kernel drivers), `archive/docs/RESEARCH.md` | DEVID inventory incl. TUF_RGB block, fan-curve DEVIDs, PPT IDs; firmware's own 0xFFFFFFFE rejections documented |
| 2 Compatibility mapping | feature-by-feature matrix | `docs/COMPATIBILITY.md` | 6-way classification, per-feature channel |
| 3 Architecture + simulation | HAL with mocks, no hardware commands | `tuf-hal` (`SysfsBackend{root}` seam + `MockBackend`), `docs/ARCHITECTURE.md` | all 17 tests run on fixture trees, zero hardware contact |
| 4 Implementation | recreate supported features natively | `tuf-core` + `tuf-hal` + `tuf-gui` (5 write ops, full monitoring), `tuf-crate-cli` | built, 17/17 tests, clippy 0 |
| 5 Static analysis + isolated tests | unit/integration/static/deps/perms/regression | `cargo test` (17), `cargo clippy` (0 warnings), `cargo audit` (0 vulns/324 deps), `docs/DEPENDENCY-AUDIT.md`, `docs/UI-VERIFICATION.md` | recorded results |
| 6 Hardware integration | read-only first, verified controls only | monitoring-only default; 5 verified write ops; read-only CLI has no write code | real-machine CLI run (FX706HF-verified: true) |
| 7 Final deployment | non-disruptive install/uninstall | `scripts/install.sh`, `uninstall.sh`, backup/restore; no services/udev/sudo/modules | installed to ~/.local, verified |

## Deliverables (original §8, 1–14)
| # | Deliverable | Artifact |
|---|---|---|
| 1 | Functionality & protocol research | `archive/docs/RESEARCH.md` + `archive/research/` corpus |
| 2 | FX706HF compatibility matrix | `docs/COMPATIBILITY.md` |
| 3 | RE findings with evidence | RESEARCH.md §3–5 (DSDT line references), `archive/research/acpi/WMNB.dsl` |
| 4 | Architecture & technical docs | `docs/ARCHITECTURE.md` |
| 5 | Native Linux backend | `crates/tuf-core`, `crates/tuf-hal` (Rust) |
| 6 | Hardware abstraction + safety layer | HAL root-seam, `VerifiedAttr` closed enum, clamps, `CapabilitySet`, audit log |
| 7 | Graphical application | `crates/tuf-gui` (eframe) — 7 tabs, all render- and interaction-verified on Xvfb |
| 8 | Simulated hardware & test env | `MockBackend` + fixture sysfs trees |
| 9 | Unit & integration tests | 17 tests (5 core + 12 hal incl. 4 degraded-capability fixtures) |
| 10 | Install/uninstall procedures | `scripts/install.sh`, `scripts/uninstall.sh` |
| 11 | Config backup & restoration | `scripts/backup-config.sh`, `scripts/restore-config.sh`, audit JSONL |
| 12 | Hardware safety documentation | `docs/SAFETY.md` (binding policy, enforced by construction) |
| 13 | Feature-by-feature comparison | `docs/COMPARISON.md` |
| 14 | Unsupported features + why | `docs/UNSUPPORTED.md` (firmware-evidence per item) |

## Feature sections (original §2) — status
- **Performance/modes**: Silent/Balanced/Performance(Turbo) ✅ implemented; CPU power mgmt ✅ (delegated to TLP, displayed); GPU mgmt ✅ monitoring; hybrid/dGPU ✅ (offload launcher + gated disable); mode switching ✅
- **Fans/thermals**: temp monitoring ✅; RPM monitoring ✅; fan control/curves ⛔ (firmware no-ops, PWM unsafe — documented); throttle monitoring ✅; perf-vs-temp ✅ (dashboard bars)
- **Keyboard/RGB**: brightness ✅; static colour ⛔ (EC persists Windows-set value); dynamic effects ⛔ (firmware rejects WMI RGB — investigated as required, evidence in RESEARCH §4); profiles/hotkeys ⛔ (brightness hotkey handled by kernel)
- **Battery**: thresholds ✅; health ✅; charge modes ✅; power draw ✅; AC/battery switching ✅ (displayed; TLP owns policy)
- **GPU/display**: iGPU/dGPU switching ✅ (gated); NVIDIA power mgmt ✅ monitoring (writes forbidden); refresh rate ➖ desktop-owned; panel overdrive ✅

## Safety requirements (original §4) — enforcement
Every restriction is enforced by code shape, not convention: no EC/ACPI/firmware/PWM/module
APIs exist in the workspace; writes = closed 5-attr enum through one gated funnel; CLI has no
write paths; monitoring-only default; double-confirmation for dGPU; audit log for rollback.
See `docs/SAFETY.md`.

## Known deviations
1. Fan curves / RGB dynamics / PPT — unsupported **by this hardware's firmware** (evidence documented), per the "rather unsupported than unsafe" instruction.
2. `asusctl`/`supergfxctl` not used — no Debian packages; native interfaces already cover the exposed feature set (goal's "build upon existing kernel drivers" instruction).
3. Refresh-rate switching lives in the desktop environment — duplicating it would create display-state conflicts.
