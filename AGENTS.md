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
Kairos is a Rust-based terminal built on the open-source Warp BYOE fork. It connects
to `specsmith serve` via HTTP/WebSocket for all governance operations: preflight
approval, post-change verification, confidence scoring, and a WebView-based governance
dashboard. Governance state lives exclusively in specsmith. Kairos owns terminal UX and
the agent execution runtime.

## Quick Commands
- `cargo build` — build Kairos
- `cargo test` — run Rust tests
- `cargo clippy -- -D warnings` — lint
- `cargo fmt --check` — format check
- `py -m specsmith audit --project-dir .` — governance health check
- `py -m specsmith serve` — start governance backend (required for governed actions)
- `py -m specsmith preflight "<utterance>"` — classify + approve a change
- `py -m specsmith verify` — post-change confidence check

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
- `src/main.rs` — binary entry point
- `src/governance/mod.rs` — governance module root
- `src/governance/client.rs` — async reqwest client for specsmith serve (/health, /preflight, /verify)
- `src/governance/server.rs` — GovernanceServer: spawns + manages specsmith serve child process
- `src/webview/` — governance dashboard WebView panel (planned)
- `tests/` — Rust integration tests
- `e2e/` — Playwright end-to-end tests (planned)
- **All governance files live in `docs/`** (except AGENTS.md at root):
- `docs/SPECSMITH.yml` — project scaffold config (canonical)
- `docs/ARCHITECTURE.md` — architecture reference and invariants
- `docs/REQUIREMENTS.md` — formal requirements (REQ-001..REQ-008)
- `docs/TESTS.md` — test specifications
- `docs/LEDGER.md` — session ledger
- `.specsmith/` — machine state (config.yml, requirements.json, testcases.json, workitems.json)

## Governance (Hard Rules)
- **H11** — Every loop/blocking wait must have a timeout, fallback exit, and diagnostic
- **H12** — Windows multi-step automation goes into `.cmd` files, not inline shell
- **H13** — Agent tools must declare epistemic contracts
- **H14** — Documentation updates are mandatory in the same commit as code changes
- AGENTS.md must remain under 200 lines
- All agent-invoked commands must have timeouts
- Record every session in LEDGER.md

## Tech Stack
- Terminal: Rust stable | Foundation: Warp BYOE fork (planned)
- Governance client: reqwest 0.12 + tokio 1 | Serialization: serde_json
- WebView: planned (wry / webview2 on Windows, wry on Linux/macOS)
- E2E tests: Playwright | Rust tests: cargo test
- CI: GitHub Actions (3 OS) | Lint: clippy | Format: rustfmt
- Governance backend: BitConcepts/specsmith via `specsmith serve`

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
session and Warp context (not a dedicated kairos session). When working on kairos,
open it from within the same Warp session used for specsmith. This arrangement holds
until kairos has its own stable agent session setup. Changes made here are recorded
in both `LEDGER.md` files — kairos changes are also noted in specsmith's LEDGER.md
during this co-management period.

## Shorthand Commands
When user says `audit`: `py -m specsmith audit --project-dir .`
When user says `session-end`: `py -m specsmith session-end --project-dir .`
