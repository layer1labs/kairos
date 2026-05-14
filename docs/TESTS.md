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

## TEST-005. Governance Settings Dashboard
- **ID:** TEST-005
- **Title:** Governance Settings Dashboard
- **Description:** Verify the Settings → Governance page is reachable from the settings sidebar, activates `SettingsSection::Governance`, and renders the live health status panel without panicking. The `GovernanceClient::health()` async call fires on init and on page-select; HealthStatus::Unknown is acceptable if specsmith is not running.
- **Requirement ID:** REQ-005
- **Type:** integration
- **Verification Method:** Kairos integration framework (`crates/integration`) + governance unit tests (`crates/kairos-governance/tests/governance_tests.rs`). Full UI path verified by `test_governance_page_renders` (marked `#[ignore]` in CI; requires real display). Health client unit tests cover `HealthStatus` state machine and `GovernanceConfig` invariants.
- **Input:** Kairos terminal open; Settings sidebar; optional `specsmith serve --port 7700` for health check to resolve.
- **Expected Behavior:** `SettingsView::current_settings_section()` returns `SettingsSection::Governance` after sidebar click. Page renders without panic. HealthStatus is `Unknown` (no serve) or `Healthy { version }` (with serve).
- **Confidence:** 0.9

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

## TEST-009. GPU-Aware Context Window Sizing in Governance Panel
- **ID:** TEST-009
- **Title:** GPU-Aware Context Window Sizing in Governance Panel
- **Description:** The Governance panel renders an "Ollama Context" card with the recommended `num_ctx` value for the detected VRAM tier. On a machine with no NVIDIA/AMD GPU, the card shows 4096 (CPU fallback). On a simulated 24 GB GPU, it shows 32768.
- **Requirement ID:** REQ-009
- **Type:** integration
- **Verification Method:** manual (Rust UI build required)
- **Input:** Settings → Governance → Ollama Context card; mock specsmith GPU response
- **Expected Behavior:** Correct context recommendation displayed; no crash on GPU detection failure
- **Confidence:** 0.85

## TEST-010. Context Fill Indicator and Auto-Compression
- **ID:** TEST-010
- **Title:** Context Fill Indicator and Auto-Compression
- **Description:** When specsmith emits a `context_fill` JSONL event with `pct=82`, the agent footer fill bar shows 82% and yellow color. When `pct` reaches `compression_threshold` (80%), `SummarizeAIConversation` is fired before the next user input.
- **Requirement ID:** REQ-010
- **Type:** integration
- **Verification Method:** cargo-test
- **Input:** Synthetic `context_fill` JSONL events with pct=50, 80, 82
- **Expected Behavior:** Fill bar updates; compression fired at threshold; no crash
- **Confidence:** 0.85

## TEST-011. Hard Context Ceiling Enforcement
- **ID:** TEST-011
- **Title:** Hard Context Ceiling Enforcement
- **Description:** When specsmith emits `context_fill` with `pct=85` (hard ceiling signal), Kairos triggers emergency compression before accepting further input. Context fill never reaches 100%. The 15% reservation is non-configurable.
- **Requirement ID:** REQ-011
- **Type:** integration
- **Verification Method:** cargo-test
- **Input:** Synthetic `context_fill` event with pct=85 (ContextFullError)
- **Expected Behavior:** Emergency compression triggered; user input blocked until compressed
- **Confidence:** 0.85

## TEST-012. AI Providers Bucket Score Columns
- **ID:** TEST-012
- **Title:** AI Providers Bucket Score Columns
- **Description:** The Agents → AI Providers table renders R/C/L columns with numeric scores from specsmith. The Sync Scores button calls `GET /api/model-intel/sync` and refreshes the table. Long model names (`o4-mini-deep-research`) are clipped and do not overflow.
- **Requirement ID:** REQ-012
- **Type:** integration
- **Verification Method:** manual (Rust UI build + visual inspection)
- **Input:** Navigate to Agents → AI Providers; click Sync Scores
- **Expected Behavior:** R/C/L columns populated; no overflow; sync triggers API call
- **Confidence:** 0.8

## TEST-013. Model Intelligence REST Endpoints
- **ID:** TEST-013
- **Title:** Model Intelligence REST Endpoints
- **Description:** `GET /api/model-intel/scores` returns HTTP 200 with `{"scores": [...]}` where each entry has `model_name`, `reasoning_score`, `conversational_score`, `longform_score`. `GET /api/model-intel/recommendations` returns HTTP 200 with `{"recommendations": [...], "bucket": "reasoning"}`.
- **Requirement ID:** REQ-013
- **Type:** integration
- **Verification Method:** cargo-test (HTTP client against specsmith governance server)
- **Input:** Running specsmith governance-serve --port 7700; GET /api/model-intel/scores; GET /api/model-intel/recommendations
- **Expected Behavior:** 200 with correct JSON shapes; at least one model returned
- **Confidence:** 0.9

## TEST-014. ESDB Settings Page Renders Without Overflow
- **ID:** TEST-014
- **Title:** ESDB Settings Page Renders Without Overflow
- **Description:** Settings → Specsmith → ESDB page renders status row, action buttons (Refresh, Export JSON, Import, Backup, Rollback, Compact) without layout errors. Refresh button triggers `specsmith esdb status`. All buttons are clickable.
- **Requirement ID:** REQ-014
- **Type:** integration
- **Verification Method:** manual (Rust UI build required)
- **Input:** Navigate to Settings → Specsmith → ESDB
- **Expected Behavior:** All elements visible; status text displayed; no crash
- **Confidence:** 0.8

## TEST-015. Skills Settings Page Renders
- **ID:** TEST-015
- **Title:** Skills Settings Page Renders
- **Description:** Settings → Specsmith → Skills page renders header, description, and CLI hint for `specsmith skills list/build/activate` without errors. Does not require a live specsmith connection to render.
- **Requirement ID:** REQ-015
- **Type:** integration
- **Verification Method:** manual (Rust UI build required)
- **Input:** Navigate to Settings → Specsmith → Skills
- **Expected Behavior:** Page content displayed without crash; CLI hint visible
- **Confidence:** 0.8

## TEST-016. Eval Settings Page Renders
- **ID:** TEST-016
- **Title:** Eval Settings Page Renders
- **Description:** Settings → Specsmith → Eval page renders header, description, and CLI hint for `specsmith eval run/report` without errors.
- **Requirement ID:** REQ-016
- **Type:** integration
- **Verification Method:** manual (Rust UI build required)
- **Input:** Navigate to Settings → Specsmith → Eval
- **Expected Behavior:** Page content displayed without crash
- **Confidence:** 0.8

## TEST-017. MCP AI Builder Card Generates and Saves Stub
- **ID:** TEST-017
- **Title:** MCP AI Builder Card Generates and Saves Stub
- **Description:** The MCP AI Builder card in Agents → MCP servers accepts a description, generates a stub via `specsmith mcp generate <desc>`, displays the JSON preview, and appends to `~/.specsmith/mcp.json` on Add to Config click.
- **Requirement ID:** REQ-017
- **Type:** integration
- **Verification Method:** manual (Rust UI build required)
- **Input:** Enter description; click Generate; click Add to Config
- **Expected Behavior:** JSON stub displayed; mcp.json updated after add
- **Confidence:** 0.8

## TEST-019. Bug Report Form with Duplicate Detection
- **ID:** TEST-019
- **Title:** Bug Report Form with Duplicate Detection
- **Description:** Verify the Settings → Bug Report page renders, accepts title/description input, calls `specsmith issue check --json` before filing, displays matching issues with similarity scores, blocks filing when duplicates ≥ 0.60 exist, and files via `specsmith issue file --json` when cleared or forced.
- **Requirement ID:** REQ-019
- **Type:** integration
- **Verification Method:** manual (Rust UI build + specsmith + gh CLI required). States tested manually: Idle, Checking, NoMatches, Matches, Filing, Filed, Error.
- **Input:** Title input → Return → Check Duplicates; File Report; File Anyway.
- **Expected Behavior:** Duplicate guard fires; filed issue URL displayed and clickable; reset clears form.
- **Confidence:** 0.85

## TEST-018. specsmith YAML Governance CI Gate
- **ID:** TEST-018
- **Title:** specsmith YAML Governance CI Gate
- **Description:** The Kairos `governance` CI job installs specsmith, runs `specsmith validate --strict --project-dir .` (exits 0 on clean schema), and runs `specsmith sync --check --project-dir .` (exits 0 when in sync). Both steps block the CI on failure.
- **Requirement ID:** REQ-018
- **Type:** integration
- **Verification Method:** CI
- **Input:** Push to main or develop branch; `governance` job in .github/workflows/ci.yml
- **Expected Behavior:** Both steps exit 0; CI governance job passes
- **Confidence:** 0.9
