# Requirements

## 1. specsmith serve as Sole Governance Interface
- **ID:** REQ-001
- **Title:** specsmith serve as Sole Governance Interface
- **Description:** `specsmith serve` MUST be the sole interface between the Kairos Rust terminal and the Python governance stack. No direct LLM API calls are made from Kairos.
- **Source:** ARCHITECTURE.md
- **Status:** implemented — `crates/kairos-governance/src/governance/client.rs`

## 2. Kairos Spawns specsmith serve as Managed Child Process
- **ID:** REQ-002
- **Title:** Kairos Spawns specsmith serve as Managed Child Process
- **Description:** Kairos MUST spawn `specsmith serve` as a managed child process at terminal startup and cleanly terminate it on exit.
- **Source:** ARCHITECTURE.md
- **Status:** implemented — `app/src/bin/oss.rs` calls `GovernanceServer::spawn()`

## 3. Preflight via REST API
- **ID:** REQ-003
- **Title:** Preflight via REST API
- **Description:** Kairos MUST call `POST /preflight` on `specsmith serve` before executing any governance-gated action, and MUST block the action on a rejection response.
- **Source:** ARCHITECTURE.md
- **Status:** implemented — `crates/kairos-governance/src/governance/client.rs::preflight()`

## 4. Verify via REST API
- **ID:** REQ-004
- **Title:** Verify via REST API
- **Description:** Kairos MUST call `POST /verify` on `specsmith serve` after changes and MUST display the returned confidence score in the terminal UI.
- **Source:** ARCHITECTURE.md
- **Status:** implemented — `crates/kairos-governance/src/governance/client.rs::verify()`

## 5. WebView Governance Dashboard
- **ID:** REQ-005
- **Title:** WebView Governance Dashboard
- **Description:** Kairos settings and governance dashboard MUST be implemented as a WebView panel, enabling Playwright-based end-to-end testing of governance state.
- **Source:** ARCHITECTURE.md
- **Status:** partial — `Settings → Governance` page implemented (`app/src/settings_view/governance_page.rs`); shows static specsmith info. Live health polling and Playwright E2E tests are future work.

## 6. Warp BYOE Fork Foundation
- **ID:** REQ-006
- **Title:** Warp BYOE Fork Foundation
- **Description:** The Kairos terminal MUST be based on the open-source Warp fork that includes BYOE (Bring Your Own Endpoint) support.
- **Source:** ARCHITECTURE.md
- **Status:** implemented — `BitConcepts/kairos` IS the Warp/OpenWarp fork

## 7. Rust Stable Implementation
- **ID:** REQ-007
- **Title:** Rust Stable Implementation
- **Description:** The Kairos terminal binary MUST compile and run on Rust stable with no nightly-only feature flags in the main build.
- **Source:** ARCHITECTURE.md
- **Status:** implemented — `cargo check -p kairos --bin kairos` passes on Rust 1.92 stable

## 8. Local-Only Governance Communication
- **ID:** REQ-008
- **Title:** Local-Only Governance Communication
- **Description:** All governance communication between Kairos and specsmith MUST occur over local HTTP/WebSocket only (127.0.0.1). No governance operations may route to external network addresses.
- **Source:** ARCHITECTURE.md
- **Status:** implemented — `GovernanceConfig::validate()` enforces localhost-only; all governance HTTP targets `127.0.0.1:7700`
