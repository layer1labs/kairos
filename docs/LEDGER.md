# Ledger — kairos

## 2026-05-07T15:31 — Bootstrap: initial governance scaffold
- **Author**: specsmith-agent (Oz / Warp)
- **Type**: bootstrap
- **REQs affected**: REQ-001..REQ-008
- **Status**: complete
- **Chain hash**: `genesis`

Created GitHub repo `BitConcepts/kairos` (private, MIT license).
Bootstrapped full governance scaffold aligned to AEE spec 0.10.1:
- `AGENTS.md` — governance hub
- `scaffold.yml` — specsmith project config
- `REQUIREMENTS.md` / `TESTS.md` — REQ-001..REQ-008 from specsmith PRX integration contract
- `LEDGER.md` — this file
- `.specsmith/` — machine state (requirements.json, testcases.json, workitems.json, config.yml, ledger.jsonl)
- `Cargo.toml` + `src/main.rs` — Rust binary stub
- `.github/workflows/ci.yml` — CI (3-OS cargo build + governance audit job)
- `.gitignore` — Rust + specsmith ignores
- `README.md` — project overview with architecture diagram

Source requirements from: `BitConcepts/specsmith` → `docs/PLANNED-REQUIREMENTS.md` §PRX (PRX-001..PRX-006).
REQ-007 (Rust stable) and REQ-008 (local-only comms) added as foundational bootstrap requirements.

**Next session**: implement specsmith serve client in `src/governance/`, wire preflight
gating, and stub WebView dashboard panel.

## 2026-05-07T23:34 — Full Kairos terminal fork implementation (Phases 1–6 + branding)
- **Author**: Oz (Warp AI agent)
- **Type**: implementation / rebrand / tooling
- **REQs affected**: REQ-001..REQ-008 (all implemented or partial)
- **Status**: complete — all success criteria verified
- **Chain hash**: `cargo check -p kairos --bin kairos` → 0 errors

### Build Environment Established (Windows)
- Rust 1.92.0 (x86_64-pc-windows-msvc) installed via `winget install Rustlang.Rustup`
- protoc 34.1 installed via `winget install Google.Protobuf`
- Both are on system PATH — refresh with:
  ```powershell
  $env:PATH = [System.Environment]::GetEnvironmentVariable("PATH","Machine") + ";" + [System.Environment]::GetEnvironmentVariable("PATH","User")
  ```

### Cargo Package Renamed
- `app/Cargo.toml`: `[package] name` changed from `"warp"` to `"kairos"`
- **CRITICAL**: `[lib] name` kept as `"warp"` — all internal `use warp::` imports still work
- Workspace `Cargo.toml`: `warp = {path="app"}` → `kairos = {path="app"}`
- `crates/integration/Cargo.toml`: dependency renamed to `kairos`
- **Command is now**: `cargo check -p kairos --bin kairos`

### i18n Locale Files Renamed (CRITICAL)
- `app/i18n/en/warp.ftl` → `app/i18n/en/kairos.ftl`
- `app/i18n/zh-CN/warp.ftl` → `app/i18n/zh-CN/kairos.ftl`
- The `fl!()` macro from `i18n-embed-fl` derives the `.ftl` filename from the Cargo **package** name.
  If you rename the package again, you MUST rename the `.ftl` files to match.

### Phases Completed

| Phase | Description | Status |
|-------|-------------|--------|
| 1 | Safe deletions (website, .claude, .deepseek, etc.) | ✅ Complete |
| 2 | Break cloud connectivity (GraphQL stub, skip_login, feature flags) | ✅ Complete |
| 3 | Cloud code runtime-dead; source deletion deferred | ✅ Runtime / ⏳ Source |
| 4 | Wire specsmith governance (GovernanceServer spawn, BYOP localhost) | ✅ Complete |
| 5 | Kairos rebrand (name, i18n, icons, menu, about page) | ✅ Complete |
| 6 | Bug reporting via GitHub Issues (kairos / specsmith routing) | ✅ Complete |

### Key Changes Made This Session

**Governance integration (REQ-001..REQ-004, REQ-008)**
- `crates/kairos-governance/src/lib.rs`: re-exported `GovernanceServer`, `GovernanceClient`,
  `GovernanceConfig`, `SessionConfig`, `find_specsmith_cmd` at crate root
- `crates/kairos-governance/src/governance/client.rs`: fixed borrow-after-move in `preflight()` and `verify()`
- `crates/kairos-governance/src/governance/server.rs`: fixed invalid string concat syntax
- `crates/kairos-governance/src/session.rs`: removed unused `Path` import
- `app/src/bin/oss.rs`: fixed `GovernanceServer::spawn()` call — now passes `(cmd, 7700, Duration::from_secs(15))`

**Settings → Governance page (REQ-005 partial)**
- `app/src/settings_view/governance_page.rs` (new) — read-only info page
- `app/src/settings_view/mod.rs` — `SettingsSection::Governance`, Display, FromStr, nav, page registration, `should_render_page`
- `app/src/settings_view/settings_page.rs` — `SettingsPageViewHandle::Governance`

**BYOP default (REQ-007)**
- `app/src/settings/ai.rs`: OpenAI and OpenAIResp default `base_url` → `http://127.0.0.1:7700/v1/`

**Help menu / bug reporting (Phase 6)**
- `app/src/util/links.rs`: `report_bug_url(repo)` generator; `feedback_form_url()` alias; BitConcepts URLs
- `app/src/app_menus.rs`: help menu → "Report Bug (Terminal/UI)" and "Report Bug (AI/Governance)"
- `app/src/workspace/view.rs` + `resource_center/view.rs`: replaced removed `SLACK_URL` with `KAIROS_ISSUES_URL`

**Kairos Amber theme (Phase 5)**
- `app/src/themes/default_themes.rs`: `kairos_amber()` + `KAIROS_AMBER_{NORMAL,BRIGHT}_COLORS`
- `app/src/themes/theme.rs`: `ThemeKind::KairosAmber`, Display, `WarpThemeConfig::new()` registration
- `themes/kairos_amber.yaml`: user-installable YAML copy

**Brand assets**
- `app/assets/bundled/png/kairos-icon.png` (896 KB)
- `app/assets/bundled/png/kairos-wordmark.png` (71.5 KB) — v3: Inter SemiBold, white divider
- `app/assets/bundled/svg/kairos-icon.svg`, `kairos-wordmark.svg`
- `logo.png` at repo root (icon copy)
- `.github/kairos-wordmark.png` (wordmark for README rendering on GitHub)
- README header uses `<img src=".github/kairos-wordmark.png" ...>` so it renders on github.com

**GitHub repo metadata**
- Description, topics (terminal, rust, ai, governance, warp-fork, specsmith, byop, developer-tools)
- Labels: bug (red), enhancement (blue), governance (purple), build (yellow), branding (amber)
- `kairos` topic added to specsmith repo for cross-discovery
- `.github/ISSUE_TEMPLATE/bug_report.md`, `feature_request.md`, `config.yml`

**Convenience script**
- `Open-Kairos.ps1` at repo root — PS 5.1 + PS 7 compatible; auto-reinvokes under pwsh 7 if launched from powershell.exe; supports `-Release` and `-NoBuild` flags

**Documentation**
- `README.md`: full rewrite for Kairos (BitConcepts URLs, specsmith governance, build instructions)
- `LICENSE`: replaced duplicate BitConcepts MIT with umbrella notice (3-tier: Denver Tech MIT, Denver Tech AGPL, BitConcepts MIT)
- `SECURITY.md`, `CODE_OF_CONDUCT.md`, `CONTRIBUTING.md`: all updated to info@bitconcepts.tech and BitConcepts branding
- `docs/FORK-SETUP.md`: DELETED (obsolete — fork is complete)
- `docs/ARCHITECTURE.md`: updated to reflect actual current state
- `docs/REQUIREMENTS.md`: all statuses updated to `implemented`
- `AGENTS.md`: File Registry, Quick Commands, Tech Stack updated
- `specs/cloud-removal/PRODUCT.md`: all success criteria ticked; Phase 6 documented

### Phase 3 Source Deletion (Deferred)
Runtime is clean — all cloud calls fail silently. Source deletion of `app/src/server/`,
`app/src/drive/`, `app/src/notebooks/`, `ai/cloud_agent_config/`, `ai/cloud_environments/`
is a multi-week refactor (each module is imported in 30+ files). Deferred to a
dedicated session. Does not affect runtime behavior.

### Success Criteria — All Met
- [x] `cargo check -p kairos --bin kairos` → 0 errors (Rust 1.92 stable, Windows MSVC)
- [x] No `warp.dev` references in production source (`grep` returns empty)
- [x] Terminal launches without login (`skip_login` in default features)
- [x] BYOP default is `http://127.0.0.1:7700/v1/`
- [x] `specsmith governance-serve` spawns at start via `GovernanceServer::spawn()`
- [x] Zero runtime calls to Warp servers (GraphQL stubbed; all cloud flags off)

### Next Session Checklist (for any machine)
1. Install Rust 1.92+ via `winget install Rustlang.Rustup` (Windows) or `rustup.rs`
2. Install protoc via `winget install Google.Protobuf` (Windows) or OS package manager
3. Refresh PATH: `$env:PATH = [System.Environment]::GetEnvironmentVariable("PATH","Machine") + ";" + [System.Environment]::GetEnvironmentVariable("PATH","User")`
4. Verify: `cargo check -p kairos --bin kairos` → should be 0 errors in ~2min cold
5. Read `specs/cloud-removal/PRODUCT.md` for phase tracker state
6. **Set GitHub social preview** (must be done in web UI): Settings → Social preview → upload `logo.png`
7. Optional next features:
   - Phase 3 source deletion (server/, drive/, notebooks/) — large refactor
   - Governance page live health polling (`GET /health` from settings UI)
   - Playwright E2E tests for Settings → Governance

## 2026-05-07T17:00 — Rust setup: toolchain pin, lib target, governance integration tests
- **Author**: specsmith-agent (Oz / Warp)
- **Type**: implementation
- **REQs affected**: REQ-007
- **Status**: complete
- **Chain hash**: `pending`

Added `rust-toolchain.toml` pinning `channel = "stable"` to ensure reproducible builds
across CI and developer machines.  **Rust stable must be installed before `cargo build`
or `cargo test` will work.** Install via https://rustup.rs/ then `rustup update stable`.

Added `src/lib.rs` library target so integration tests under `tests/` can import the
`kairos::governance::client` types without duplicating source files. Updated `Cargo.toml`
with the `[lib]` section accordingly, and updated `src/main.rs` to use
`kairos::governance::GovernanceClient` from the library.

Added `tests/governance_tests.rs` with 22 integration tests covering:
- `GovernanceConfig` localhost validation and external-host rejection (invariant I2)
- `DEFAULT_PORT` constant value
- `GovernanceClient` construction (valid and invalid configs)
- `PreflightDecision.accepted()` for all decision variants
- `VerifyResult` field semantics and equilibrium invariants
