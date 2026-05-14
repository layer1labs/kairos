# Ledger — kairos

## 2026-05-14T16:00 — Phase 2: Token/Context UX — REQ-020/021/022
- **Author**: oz-agent
- **Type**: feature — UX / settings
- **REQs affected**: REQ-020, REQ-021, REQ-022
- **Status**: complete
- **Chain hash**: auto

### Summary

Phase 2 Token/Context UX: added token usage panel, context fill bar in Governance
page, and editable `num_ctx` control.

### kairos changes

**`app/src/kairos_context_fill.rs`** (singleton model — REQ-021/022)
- `ContextFillState` singleton: holds `fill_pct: Option<f32>`, `custom_num_ctx`,
  `pending_num_ctx_str`, `save_in_progress`, `save_result`.
- `FillTier` enum: Low (<60%), Medium (60-79%), High (≥80%), Unknown.
- `set_fill()`, `load_num_ctx()`, `start_save()` methods.
- Registered in `app/src/lib.rs` `initialize_app()`.

**`app/src/settings_view/token_usage_page.rs`** (REQ-020)
- `TokenUsagePageView` + `TokenUsageWidget` settings page.
- On init/refresh: spawns `py -m specsmith credits summary --json --project-dir ~`,
  falls back to `specsmith credits summary --json --project-dir ~`.
- Displays: budget bar, alerts, total tokens in/out, cost, session/entry counts,
  per-model breakdown (sorted by cost desc).
- Refresh button (`TokenUsagePageAction::Refresh`).
- Clear hint pointing to `specsmith credits record --clear`.

**`app/src/settings_view/mod.rs`** (wiring)
- `SettingsSection::TokenUsage` variant added.
- `Display` → `"Token Usage"`, `FromStr` accepts `"TokenUsage"`/`"Token Usage"`.
- `pub(crate) mod token_usage_page;` added.
- `SettingsNavItem::Page(SettingsSection::TokenUsage)` after BugReport in nav.
- `settings_pages.extend` includes `token_usage_page_handle`.

**`app/src/settings_view/settings_page.rs`** (wiring)
- `SettingsPageViewHandle::TokenUsage(ViewHandle<TokenUsagePageView>)` added.
- `child_view()` arm added.

**`app/src/settings_view/governance_page.rs`** (REQ-021/022)
- Imports: `ContextFillState`, `SubmittableTextInput`, `SubmittableTextInputEvent`, `ChildView`.
- `num_ctx_input: ViewHandle<SubmittableTextInput>` field.
- Subscribes to `ContextFillState` in `new()` for re-renders.
- Calls `load_num_ctx()` on init.
- `on_num_ctx_event` handler: validates and calls `start_save` on submit.
- Context Window card in widget render: fill dot + %, num_ctx label + input + save result.
- Assembled after engine card, before updater section.

### Docs
- `docs/REQUIREMENTS.md`: REQ-019 (retroactive), REQ-020, REQ-021, REQ-022 added.
- `docs/TESTS.md`: TEST-019 (retroactive), TEST-020, TEST-021, TEST-022 added.
- `.specsmith/requirements.json` + `.specsmith/testcases.json`: matching entries added.

---

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
| 4 | Wire specsmith governance (GovernanceServer spawn, BYOE localhost) | ✅ Complete |
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

**BYOE default (REQ-007)**
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
- Description, topics (terminal, rust, ai, governance, warp-fork, specsmith, BYOE, developer-tools)
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
- [x] BYOE default is `http://127.0.0.1:7700/v1/`
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

---

## 2026-05-14T10:55 — WI-0514: REQ-005 completed — governance page integration test + docs
- **Author**: oz-agent
- **Type**: feature / test
- **REQs affected**: REQ-005
- **Status**: complete
- **Chain hash**: auto

REQ-005 advanced from `partial` to `implemented`. Changes:
- Added `crates/integration/src/test/settings_governance.rs` with `test_governance_page_renders` — opens Settings, clicks Governance sidebar item, asserts `SettingsSection::Governance` is active. Registered in `src/test.rs`, `src/bin/integration.rs`, and `tests/integration/ui_tests.rs` (with `#[ignore]` annotation; requires real display).
- Updated `docs/REQUIREMENTS.md` REQ-005: title + description corrected (native Rust page, not WebView); status `partial` → `implemented`.
- Updated `docs/TESTS.md` TEST-005: verification method updated to reflect Kairos integration framework + governance unit tests (not Playwright).
- Updated `docs/ARCHITECTURE.md` invariant I4: Playwright-testable → Kairos integration framework testable.
- Updated `.specsmith/requirements.json` and `.specsmith/testcases.json` to match.
- Also cross-recorded in specsmith `docs/LEDGER.md` WI-0514 entry.

## 2026-05-09T01:00 — Governance page upgrades, SSH Integration rename, Gruvbox Dark default, per-project shell memory, context window management
- **Author**: Oz (Warp AI agent)
- **Type**: implementation — compliance / UX / context
- **REQs affected**: REQ-001..REQ-008 (governance page), context window (new)
- **Status**: complete — CI green
- **Chain hash**: `c0bb0ac`

### Summary

This session adds compliance UI, per-project shell memory, SSH Integration rename,
Gruvbox Dark as the new-user default theme, and context window management wiring.
All changes compiled cleanly (`cargo check` passes) and CI is green.

### Governance Page Upgrades (`app/src/settings_view/governance_page.rs`)

**Multi-manager update check**
The specsmith update button now checks `pipx upgrade specsmith` first, then falls
back to `pip install --upgrade specsmith`, then `pip3 install --upgrade specsmith`.
This covers all common installation methods without requiring the user to know which
one they used.

**Clickable bug report links**
Bug report entries in the Governance page are now wrapped in `Hoverable` elements that
dispatch `OpenLink(String)` actions, opening the correct GitHub Issues repo in the
system browser:
- Terminal/UI bugs → `https://github.com/BitConcepts/kairos/issues/new`
- AI/Governance bugs → `https://github.com/BitConcepts/specsmith/issues/new`

Button and hint text updated to match.

### SSH Integration Rename (formerly "Warpify")

All user-visible strings referring to "Warpify" have been renamed to
"SSH Integration" / "integrate" / "Integration":
- `app/i18n/en/kairos.ftl` — all `settings-warpify-*`, `terminal-warpify-*`,
  and `keybinding-desc-*warpify*` keys
- `app/i18n/zh-CN/kairos.ftl` — same keys in Simplified Chinese
- `app/src/settings_view/mod.rs` — `SettingsSection::Warpify` `FromStr` now accepts
  both `"Warpify"` (backward compat) and `"SSH Integration"` / `"SSH integration"`

The SSH bootstrap path (`app/src/terminal/ssh/warpify.rs`) is unchanged — only
user-facing strings are updated.

### Gruvbox Dark Default Theme

`app/src/settings/initializer.rs`: new users are initialised with `GruvboxDark` as
the default theme instead of the Kairos Amber theme. Kairos Amber remains available
in Settings → Themes.

### Per-Project Shell Memory (`app/src/kairos_shell_memory.rs`)

New module `kairos_shell_memory` (registered in `lib.rs`) implements:
- `find_project_root(cwd)` — walks up to find `.git`, `.kairos`, or `scaffold.yml`
- `save_shell_pref(cwd, shell)` — writes `.kairos/shell-pref.json` at project root
- `load_shell_pref(cwd)` — reads the stored `NewSessionShell`; returns `None` if absent

Hook points in `app/src/workspace/view.rs`:
- `add_tab_with_shell` — saves the chosen shell after telemetry
  (gated on `#[cfg(feature = "local_tty")]`)
- `AddDefaultTab` Terminal/Agent path — loads the stored shell preference and
  uses it instead of the global default; falls back gracefully when none is found

File format: `{ "shell": { "WSL": "Ubuntu-24.04" } }` (serialised `NewSessionShell`)

### Context Window Management Wiring

Kairos-side wiring for specsmith's `context_window.py`:
- `GovernanceSettings` gains `ollama_num_ctx`, `context_compression_threshold_pct`,
  and `context_auto_compress` fields
- Agent footer renders a compact fill progress bar from `context_fill` JSONL events
- `WorkspaceView` fires `SummarizeAIConversation` at the compression threshold (80%)
  and emergency compression at the hard ceiling (85%)
- GPU detection result surfaces in the Governance panel under an "Ollama Context" card

### Documentation
- `docs/ARCHITECTURE.md`: Per-Project Shell Memory, SSH Integration, and
  Context Window Management sections added
- `README.md`: full rewrite with compliance standards, Governance Tools Panel,
  Per-Project Shell Memory, Context Window Management, and SSH Integration sections
- `docs/LEDGER.md`: this entry

### CI
- `cargo fmt --all` applied to fix formatting drift in `lib.rs`,
  `governance_page.rs`, and `workspace/view.rs`
- CI (format check + build matrix): ✓
- Commit: `c0bb0ac` on `develop`
