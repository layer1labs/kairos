# Contributing to Kairos

## Governance First
All changes follow the AEE **propose → check → execute → verify → record** workflow.
Before writing code, read `AGENTS.md` and run `py -m specsmith audit --project-dir .`.

## Prerequisites
- Rust stable (`rustup show` to confirm toolchain)
- Python 3.10+ with specsmith installed (`pip install specsmith`)
- `specsmith serve` running at `http://127.0.0.1:7700` for governed development

## Development Workflow
1. **Preflight**: `py -m specsmith preflight "<description of change>"` — must return `accepted`
2. **Build**: `cargo build`
3. **Lint**: `cargo clippy -- -D warnings`
4. **Format**: `cargo fmt`
5. **Test**: `cargo test`
6. **Verify**: `py -m specsmith verify` — check post-change confidence score
7. **Ledger**: record the session in `LEDGER.md`

## Pull Requests
- Target the `develop` branch (not `main`)
- CI must be green: `cargo build`, `cargo test`, `cargo clippy`, `cargo fmt --check`,
  and the governance audit job
- Include a ledger entry describing what changed and why
- Link the relevant REQ-NNN requirement(s) in the PR description

## Repository Layout
See `README.md` and `docs/ARCHITECTURE.md` for structure and component descriptions.

## Code Standards
- No `unsafe` blocks without explicit justification in a comment
- All `async` functions must have a timeout (H11)
- No direct LLM API calls from Kairos code (I1 — use `specsmith serve`)
- All governance HTTP calls must target `127.0.0.1` only (I2)
- Rust stable only — no nightly features (I5)
