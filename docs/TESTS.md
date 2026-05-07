# Test Specification

## TEST-001. specsmith serve as Sole Governance Interface
- **ID:** TEST-001
- **Title:** specsmith serve as Sole Governance Interface
- **Description:** Verify Kairos only calls governance endpoints via `specsmith serve`; no direct LLM API calls are made from the Rust binary.
- **Requirement ID:** REQ-001
- **Type:** integration
- **Verification Method:** evaluator
- **Input:** {}
- **Expected Behavior:** {}
- **Confidence:** 1.0

## TEST-002. Kairos Spawns specsmith serve as Managed Child Process
- **ID:** TEST-002
- **Title:** Kairos Spawns specsmith serve as Managed Child Process
- **Description:** Verify Kairos starts `specsmith serve` as a child process on launch and terminates it cleanly on exit.
- **Requirement ID:** REQ-002
- **Type:** integration
- **Verification Method:** evaluator
- **Input:** {}
- **Expected Behavior:** {}
- **Confidence:** 1.0

## TEST-003. Preflight via REST API
- **ID:** TEST-003
- **Title:** Preflight via REST API
- **Description:** Verify Kairos calls `POST /preflight` before governed actions and blocks execution on a rejection response.
- **Requirement ID:** REQ-003
- **Type:** integration
- **Verification Method:** evaluator
- **Input:** {}
- **Expected Behavior:** {}
- **Confidence:** 1.0

## TEST-004. Verify via REST API
- **ID:** TEST-004
- **Title:** Verify via REST API
- **Description:** Verify Kairos calls `POST /verify` after changes and renders the returned confidence score in the terminal UI.
- **Requirement ID:** REQ-004
- **Type:** integration
- **Verification Method:** evaluator
- **Input:** {}
- **Expected Behavior:** {}
- **Confidence:** 1.0

## TEST-005. WebView Governance Dashboard
- **ID:** TEST-005
- **Title:** WebView Governance Dashboard
- **Description:** Verify the governance dashboard WebView panel is reachable by Playwright and correctly displays audit state from `specsmith serve`.
- **Requirement ID:** REQ-005
- **Type:** e2e
- **Verification Method:** playwright
- **Input:** {}
- **Expected Behavior:** {}
- **Confidence:** 1.0

## TEST-006. Warp BYOE Fork Foundation
- **ID:** TEST-006
- **Title:** Warp BYOE Fork Foundation
- **Description:** Verify Kairos compiles from the Warp BYOE fork base with governance hooks in place.
- **Requirement ID:** REQ-006
- **Type:** build
- **Verification Method:** evaluator
- **Input:** {}
- **Expected Behavior:** {}
- **Confidence:** 1.0

## TEST-007. Rust Stable Implementation
- **ID:** TEST-007
- **Title:** Rust Stable Implementation
- **Description:** Verify `cargo +stable build` succeeds and no nightly-only feature flags are present in `Cargo.toml` or build scripts.
- **Requirement ID:** REQ-007
- **Type:** build
- **Verification Method:** cargo
- **Input:** {}
- **Expected Behavior:** {}
- **Confidence:** 1.0

## TEST-008. Local-Only Governance Communication
- **ID:** TEST-008
- **Title:** Local-Only Governance Communication
- **Description:** Verify all governance HTTP calls target `127.0.0.1` only; no external hostnames or IPs appear in governance client code.
- **Requirement ID:** REQ-008
- **Type:** integration
- **Verification Method:** evaluator
- **Input:** {}
- **Expected Behavior:** {}
- **Confidence:** 1.0
