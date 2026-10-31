# Phase 5 dependency security audit — result

Command: `cargo install cargo-audit --locked && cargo audit` (2026-10-31)

- Advisory database: 1,395 advisories
- Dependencies scanned: **324**
- **Vulnerabilities: 0**

Informational (unmaintained, not vulnerable):
| Crate | Version | Advisory | Note |
|---|---|---|---|
| paste | 1.0.15 | RUSTSEC-2024-0436 | transitive via the egui/eframe stack; no fix available upstream |
| ttf-parser | 0.25.1 | RUSTSEC-2026-0192 | transitive via epaint (egui font rasteriser); successor crate exists but egui has not migrated |

Both are `unmaintained` notices, not vulnerability reports. Re-run `cargo audit` after dependency updates.
