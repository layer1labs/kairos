# AGENTS.md — kairos

## Identity
- **Project**: kairos
- **Type**: Rust terminal (CLI + WebView) — AEE-governed
- **Spec version**: 0.10.1
- **Language**: Rust stable
- **Platforms**: Windows, Linux, macOS
- **Role**: Epistemically-governed terminal runtime. Consumes `specsmith serve`
  as the governance backend (BitConcepts/specsmith).

## Purpose
Kairos is a Rust-based terminal built on the open-source OpenWarp/BYOE fork. It connects
to `specsmith serve` via HTTP/WebSocket for all governance operations: preflight
approval, post-change verification, confidence scoring, and a WebView-based governance
dashboard. Governance state lives exclusively in specsmith. Kairos owns terminal UX and
the agent execution runtime.

## Quick Commands
- `cargo check -p kairos --bin kairos` — full app compile check (2 min cold, ~10s warm)
- `cargo test -p kairos-governance` — governance crate tests (fast, ~4s)
- `cargo clippy --workspace -- -D warnings` — lint
- `cargo fmt --check` — format check
- `.\Open-Kairos.ps1` — build and launch (Windows)
- `.\Open-Kairos.ps1 -Release` — release build and launch
- `.\Open-Kairos.ps1 -NoBuild` — launch last compiled binary
- `py -m specsmith governance-serve --port 7700` — start governance backend manually

## Session Start
1. Read `LEDGER.md` — check last session state and open TODOs
2. Run `py -m specsmith audit --project-dir .` — governance health check
3. Confirm `specsmith serve` is reachable at `http://127.0.0.1:7700/health`

## Workflow
All changes follow: **propose → check → execute → verify → record**.
- Every code change requires a ledger entry before execution
- Run `py -m specsmith preflight "<description>"` before any change
- Run `py -m specsmith verify` after changes, before marking complete
- Never commit without updating documentation (H14)

## File Registry
**Binary entry point**
- `app/src/bin/oss.rs` — Kairos binary; spawns GovernanceServer at startup

**Governance crate** (`crates/kairos-governance/`)
- `src/governance/client.rs` — GovernanceClient: async HTTP to specsmith (/health, /preflight, /verify)
- `src/governance/server.rs` — GovernanceServer: spawns + manages specsmith child process
- `src/session.rs` — SessionConfig, find_specsmith_cmd (platform-aware specsmith detection)
- `src/lib.rs` — crate root re-exports (GovernanceClient, GovernanceServer, SessionConfig, find_specsmith_cmd)

**App settings UI**
- `app/src/settings_view/governance_page.rs` — Settings → Governance page
- `app/src/settings_view/mod.rs` — SettingsSection::Governance, nav wiring
- `app/src/settings_view/settings_page.rs` — SettingsPageViewHandle::Governance

**i18n locale files**
- `app/i18n/en/kairos.ftl` — English strings (renamed from warp.ftl — MUST match package name)
- `app/i18n/zh-CN/kairos.ftl` — Simplified Chinese strings

**Brand assets** (embedded via rust-embed)
- `app/assets/bundled/png/kairos-icon.png` — app icon
- `app/assets/bundled/png/kairos-wordmark.png` — wordmark (shown in About page)
- `app/assets/bundled/svg/kairos-icon.svg` — icon SVG
- `app/assets/bundled/svg/kairos-wordmark.svg` — wordmark SVG

**Themes**
- `app/src/themes/default_themes.rs` — kairos_amber() function + KAIROS_AMBER colors
- `app/src/themes/theme.rs` — ThemeKind::KairosAmber, WarpThemeConfig registration
- `themes/kairos_amber.yaml` — user-installable YAML version of the theme

**Cargo manifests**
- `app/Cargo.toml` — package name = "kairos"; lib name = "warp" (keep lib name to avoid mass-rename)
- `Cargo.toml` — workspace; kairos = {path = "app"}; authors = BitConcepts

**Documentation** (all in `docs/`)
- `docs/ARCHITECTURE.md` — architecture reference and invariants
- `docs/REQUIREMENTS.md` — REQ-001..REQ-008 (all implemented)
- `docs/TESTS.md` — test specifications
- `docs/LEDGER.md` — session ledger
- `specs/cloud-removal/PRODUCT.md` — phase tracker (Phases 1-6 complete)

**Scripts / convenience**
- `Open-Kairos.ps1` — build + launch script (Windows, PS 5.1 + PS 7 compatible)
- `logo.png` — Kairos icon at repo root
- `.github/kairos-wordmark.png` — wordmark for README rendering on GitHub
- `.github/ISSUE_TEMPLATE/` — bug_report.md, feature_request.md, config.yml

## Governance (Hard Rules)
- **H11** — Every loop/blocking wait must have a timeout, fallback exit, and diagnostic
- **H12** — Windows multi-step automation goes into `.cmd` files, not inline shell
- **H13** — Agent tools must declare epistemic contracts
- **H14** — Documentation updates are mandatory in the same commit as code changes
- AGENTS.md must remain under 200 lines
- All agent-invoked commands must have timeouts
- Record every session in LEDGER.md

## Tech Stack
- Terminal: Rust 1.92 stable | Foundation: OpenWarp fork (COMPLETE)
- App package: `kairos` (lib name kept as `warp` for internal imports)
- Governance client: reqwest 0.12 + tokio 1 | Serialization: serde_json
- i18n: i18n-embed + Fluent (.ftl files named after package: kairos.ftl)
- Themes: WarpTheme Rust structs in default_themes.rs (ThemeKind enum)
- CI: GitHub Actions | Lint: clippy | Format: rustfmt
- Governance backend: BitConcepts/specsmith via `specsmith governance-serve`
- Build env (Windows): Rust 1.92 via winget Rustlang.Rustup; protoc 34.1 via winget Google.Protobuf

## Governance Backend (specsmith serve)
Default endpoint: `http://127.0.0.1:7700`
- `GET  /health` — backend liveness
- `POST /preflight` — classify and gate an utterance
- `POST /verify` — post-change confidence check
- `GET  /audit` — governance audit results
- `WS   /ws/session/{id}` — live session I/O (JSONL events)

## Sister Repos
specsmith and kairos are **sister repos** — always located in the same parent directory.
Use relative paths to reference each other across machines (absolute paths vary):
- specsmith: `../specsmith/`

**Session management**: Both repos are currently governed from the **specsmith** chat
session and Kairos context (not a dedicated kairos session). When working on kairos,
open it from within the same Kairos session used for specsmith. This arrangement holds
until kairos has its own stable agent session setup. Changes made here are recorded
in both `LEDGER.md` files — kairos changes are also noted in specsmith's LEDGER.md
during this co-management period.

## Shorthand Commands
When user says `audit`: `py -m specsmith audit --project-dir .`
When user says `session-end`: `py -m specsmith session-end --project-dir .`
