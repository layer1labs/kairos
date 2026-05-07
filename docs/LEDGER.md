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
