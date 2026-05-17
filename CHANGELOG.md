# Changelog

All notable changes to **Kairos** are documented here.

Format: [Keep a Changelog](https://keepachangelog.com/en/1.0.0/) ·
Versioning: [Semantic Versioning](https://semver.org/spec/v2.0.0.html)

---

## [Unreleased]

### Added
- `Code Quality` CI workflow: independent Rustfmt, Clippy
  (`kairos-governance` enforced; dispatch UI advisory), and specsmith
  governance validate + sync checks on every push/PR.
- specsmith dispatch panel (`app/`) — live DAG graph subscribed to
  specsmith serve SSE, topological layout, bezier dependency edges,
  Gantt timeline strip, Retry/Abort action buttons per node (REQ-332..334).
- `Agent Defaults` settings page, VRAM auto-detection, endpoint discovery
  tab, provider cards UX revamp.
- ESDB dashboard scroll fix; project-path–aware ChronoStore path.
- `ChronoStore` integration — initialise ESDB on first launch, close
  NIST-RMF MEASURE-1 gap (REQ-023).

### Changed
- `.chronomemory/*.wal` and `*.tmp` added to `.gitignore` (runtime ESDB
  files should not be version-controlled).
- Machine state (`requirements.json`, `testcases.json`) regenerated from
  docs/ YAML sources.
- CI: `dtolnay/rust-toolchain` and `actions/cache` pinned to commit SHAs
  (CodeQL #284 supply-chain hardening).

### Fixed
- AI Providers table responsive layout — NAME/ID columns flex-grow.
- Cargo fmt applied across `about_page`, `kairos_updater`, `github`,
  `ai_providers_page` modules.
- Resolved 9 compiler errors in `compliance_page.rs` and `esdb_page.rs`.
- Clippy `disallowed_types` in `kairos-governance` crate.
- Linux system deps (`libdbus-1-dev`) added to CI; workspace check made
  advisory for upstream Warp crates.

---

## [0.1.0-alpha] — 2026-05-13

First public alpha release of Kairos — the AEE/specsmith companion desktop
client built on the open Warp codebase.

### Added
- **Automatic updates** with stable/dev channel selector (REQ-023).
- **In-app bug report form** with duplicate detection (REQ-019).
- **Token / Context UX Phase 2** — context window visualisation and
  optimisation controls (REQ-020, REQ-021, REQ-022).
- **Compliance page** — EU AI Act + North American AI regulation sections,
  scrollable regulation cards, H1–H22 NIST RMF coverage table with OEA
  governance ledger entry.
- **AI Providers settings page** — persistence to
  `~/.specsmith/providers.json`, bucket score columns (R/C/L), three-section
  redesign (local / cloud / custom), agent-aware VRAM recommendation.
- **Governance page** — per-agent governance view, CI status card, H15–H22
  NIST RMF rules display, SSH Integration rename.
- **ESDB Dashboard** — ChronoMemory settings page.
- **MCP AI Builder** card in settings assistant; click-fix resolved.
- **Skills + Eval** pages wired into settings view.
- **SettingsAgentView** wired into sidebar with settings assistant.
- Gruvbox Dark theme option.
- Shell memory integration in governance page.
- REQ-005 governance integration test + docs update.
- `kairos-governance` crate: foundational compliance and governance logic.
- Full specsmith governance scaffold (AEE spec 0.10.1) — REQUIREMENTS.md,
  TESTS.md, ARCHITECTURE.md, LEDGER.md, AGENTS.md, docs/governance/.
- Sister Repos section in AGENTS.md linking kairos ↔ specsmith.
- CodeQL analysis workflow (Python + Actions + Rust).

### Changed
- All OpenWarp / Warp branding replaced with Kairos across autoupdate,
  About page, FTL strings, governance page, channel icons, and sidebar.
- Governance + Compliance panels moved to tools panel.
- Chinese comments translated to English.
- Skill names updated to Kairos conventions.

### Fixed
- Stale WebView reference in ARCHITECTURE.md (REQ-005).
- Cargo fmt across all modified crates at release cut.
- Compile errors in `compliance_page.rs`, `esdb_page.rs`, `governance_page.rs`.
- Unused imports and variable warnings across workspace.

### Security
- `rand` bumped 0.9.1 → 0.9.4 (GHSA-cq8v-f236-94qc).
- `diesel`, `openssl`, `actix-http` bumped (Dependabot security advisories).
- `validator` 0.19 → 0.20, `walkdir` 2.4 → 2.5, `anyhow` 1.0.79 → 1.0.102.

---

## [0.0.1] — 2026-05-07 (Bootstrap)

### Added
- Initial commit from zerx-lab/warp (OpenWarp) @ 8f4eef1.
- `.gitattributes` normalising all files to LF.
- AEE governance scaffold seeded (spec 0.10.1).
- Architecture phase advanced; community and docs files added.

---

[Unreleased]: https://github.com/layer1labs/kairos/compare/v0.1.0-alpha...HEAD
[0.1.0-alpha]: https://github.com/layer1labs/kairos/compare/v0.0.1...v0.1.0-alpha
[0.0.1]: https://github.com/layer1labs/kairos/releases/tag/v0.0.1
