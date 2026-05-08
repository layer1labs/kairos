# Test Specification

## TEST-001. specsmith serve as Sole Governance Interface
- **ID:** TEST-001
- **Title:** specsmith serve as Sole Governance Interface
- **Description:** Verify Kairos only calls governance endpoints via `specsmith serve`; no direct LLM API calls are made from the Rust binary.
- **Requirement ID:** REQ-001
- **Type:** integration
- **Verification Method:** static-analysis
- **Input:** Source files `src/governance/client.rs`, `src/governance/mod.rs`, `src/main.rs`
- **Expected Behavior:** `grep` finds no references to any LLM provider SDK (openai, anthropic, gemini, ollama) in the Rust source. All external calls route through `GovernanceClient` targeting `127.0.0.1`.
- **Confidence:** 1.0

## TEST-002. Kairos Spawns specsmith serve as Managed Child Process
- **ID:** TEST-002
- **Title:** Kairos Spawns specsmith serve as Managed Child Process
- **Description:** Verify Kairos starts `specsmith serve` as a child process on launch and terminates it cleanly on exit.
- **Requirement ID:** REQ-002
- **Type:** integration
- **Verification Method:** cargo-test
- **Input:** `GovernanceServer::spawn("specsmith", 7700, Duration::from_secs(10))`; mock specsmith binary that responds to `/health` and exits on SIGTERM.
- **Expected Behavior:** Server handle is returned; `GET /health` returns 200; `drop(server)` terminates the child process within 2 seconds.
- **Confidence:** 0.9

## TEST-003. Preflight via REST API
- **ID:** TEST-003
- **Title:** Preflight via REST API
- **Description:** Verify Kairos calls `POST /preflight` before governed actions and blocks execution on a rejection response.
- **Requirement ID:** REQ-003
- **Type:** integration
- **Verification Method:** cargo-test
- **Input:** Mock HTTP server at 127.0.0.1:7700 returning `{"decision":"accepted", "work_item_id":"WI-XXXX", "requirement_ids":["REQ-003"], "test_case_ids":["TEST-003"], "confidence_target":0.85, "instruction":"ok", "intent":"change"}`.
- **Expected Behavior:** `GovernanceClient::preflight()` returns a `PreflightDecision` with `accepted() == true`. When the mock returns `"needs_clarification"`, `accepted() == false`.
- **Confidence:** 0.9

## TEST-004. Verify via REST API
- **ID:** TEST-004
- **Title:** Verify via REST API
- **Description:** Verify Kairos calls `POST /verify` after changes and renders the returned confidence score in the terminal UI.
- **Requirement ID:** REQ-004
- **Type:** integration
- **Verification Method:** cargo-test
- **Input:** Mock HTTP server returning `{"equilibrium":true, "confidence":0.85, "summary":"All tests passed.", "retry_strategy":"", "files_changed":["src/main.rs"]}`.
- **Expected Behavior:** `GovernanceClient::verify()` returns `VerifyResult` with `equilibrium == true` and `confidence == 0.85`. Terminal UI must display the confidence score (manual verification at UI layer).
- **Confidence:** 0.85

## TEST-005. WebView Governance Dashboard
- **ID:** TEST-005
- **Title:** WebView Governance Dashboard
- **Description:** Verify the governance dashboard WebView panel is reachable by Playwright and correctly displays audit state from `specsmith serve`.
- **Requirement ID:** REQ-005
- **Type:** e2e
- **Verification Method:** playwright
- **Input:** Running `specsmith serve --port 7700` + Kairos terminal with WebView panel open.
- **Expected Behavior:** Playwright can navigate to the WebView URL, find an element with `data-testid="governance-status"`, and confirm its text matches the `specsmith serve` health response.
- **Confidence:** 0.7

## TEST-006. Kairos BYOE Fork Foundation
- **ID:** TEST-006
- **Title:** Kairos BYOE Fork Foundation
- **Description:** Verify Kairos compiles from the OpenWarp/BYOE fork base with governance hooks in place.
- **Requirement ID:** REQ-006
- **Type:** build
- **Verification Method:** cargo
- **Input:** `cargo build --release` in the kairos repository root with Rust stable toolchain.
- **Expected Behavior:** Build succeeds with exit code 0. Binary at `target/release/kairos` is present and executes (`./kairos --version` returns non-error).
- **Confidence:** 0.9

## TEST-007. Rust Stable Implementation
- **ID:** TEST-007
- **Title:** Rust Stable Implementation
- **Description:** Verify `cargo +stable build` succeeds and no nightly-only feature flags are present in `Cargo.toml` or build scripts.
- **Requirement ID:** REQ-007
- **Type:** build
- **Verification Method:** cargo
- **Input:** `cargo +stable build` and `grep -r "#!\[feature" src/`.
- **Expected Behavior:** `cargo +stable build` exits 0. `grep` finds no `#![feature(...)]` attributes in source. `rust-toolchain.toml` specifies `channel = "stable"`.
- **Confidence:** 1.0

## TEST-008. Local-Only Governance Communication
- **ID:** TEST-008
- **Title:** Local-Only Governance Communication
- **Description:** Verify all governance HTTP calls target `127.0.0.1` only; no external hostnames or IPs appear in governance client code.
- **Requirement ID:** REQ-008
- **Type:** integration
- **Verification Method:** static-analysis + cargo-test
- **Input:** `src/governance/client.rs` source + unit tests for `GovernanceConfig::validate()`.
- **Expected Behavior:** `GovernanceConfig::validate()` rejects any non-localhost base URL with an error. Tests `config_validate_rejects_external_host` and `config_validate_rejects_lan_ip` pass (already in `tests/governance_tests.rs`).
- **Confidence:** 1.0
