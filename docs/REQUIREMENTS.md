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

## 6. Kairos BYOE Fork Foundation
- **ID:** REQ-006
- **Title:** Kairos BYOE Fork Foundation
- **Description:** The Kairos terminal MUST be based on the OpenWarp/BYOE fork that includes BYOE (Bring Your Own Endpoint) support.
- **Source:** ARCHITECTURE.md
- **Status:** implemented — `BitConcepts/kairos` IS the OpenWarp fork

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

## 9. GPU-Aware Context Window Sizing in Governance Panel
- **ID:** REQ-009
- **Title:** GPU-Aware Context Window Sizing in Governance Panel
- **Description:** The Kairos Governance settings page MUST display a recommended Ollama `num_ctx` value based on detected GPU VRAM (via specsmith `specsmith ollama gpu`). VRAM tiers: <6 GB → 4096, 6–11 GB → 8192, 12–19 GB → 16384, ≥20 GB → 32768 tokens.
- **Source:** ARCHITECTURE.md §Context Window Management
- **Status:** implemented — `app/src/settings_view/governance_page.rs` Ollama Context card

## 10. Context Fill Indicator and Auto-Compression
- **ID:** REQ-010
- **Title:** Context Fill Indicator and Auto-Compression
- **Description:** Kairos MUST render a compact fill bar in the agent footer showing current context fill percentage (from specsmith `context_fill` JSONL events). When fill reaches the compression threshold (default 80%), Kairos MUST fire `SummarizeAIConversation` before the next agent turn.
- **Source:** ARCHITECTURE.md §Context Window Management
- **Status:** implemented — `app/src/terminal/view/use_agent_footer/`; `WorkspaceView` listens for context_fill events

## 11. Hard Context Ceiling Enforcement
- **ID:** REQ-011
- **Title:** Hard Context Ceiling Enforcement
- **Description:** When context fill reaches 85% (hard ceiling), Kairos MUST trigger emergency compression immediately before accepting further user input. Context fill MUST never reach 100%. The 15% reservation is a safety invariant, not a setting.
- **Source:** ARCHITECTURE.md §Context Window Management
- **Status:** implemented — `WorkspaceView` emergency compression path; `ContextFullError` propagated from specsmith

## 12. AI Providers Bucket Score Columns
- **ID:** REQ-012
- **Title:** AI Providers Bucket Score Columns
- **Description:** The Kairos Agents → AI Providers table MUST display three additional columns: R (reasoning score), C (conversational score), L (longform score), populated from specsmith HF leaderboard data. A Sync Scores button MUST trigger a background sync without interrupting the active session.
- **Source:** ARCHITECTURE.md §AI Model Intelligence Panel
- **Status:** implemented — `app/src/settings_view/ai_providers_page.rs`

## 13. Model Intelligence REST Endpoints
- **ID:** REQ-013
- **Title:** Model Intelligence REST Endpoints
- **Description:** The specsmith governance server MUST expose `GET /api/model-intel/scores` returning `{"scores": [...]}` and `GET /api/model-intel/recommendations` returning `{"recommendations": [...], "bucket": "..."}`. Kairos MUST consume these endpoints to populate the AI Providers table.
- **Source:** ARCHITECTURE.md §AI Model Intelligence Panel
- **Status:** implemented — `specsmith.governance_logic.GovernanceHTTPServer`; kairos consumes via governance client

## 14. ESDB Settings Page
- **ID:** REQ-014
- **Title:** ESDB Settings Page
- **Description:** The Kairos Settings → Specsmith → ESDB page MUST render database status (backend, record count, chain validity) and provide action buttons (Refresh, Export JSON, Import, Backup, Rollback, Compact) that invoke the corresponding `specsmith esdb *` CLI commands.
- **Source:** ARCHITECTURE.md §Kairos Settings Extensions
- **Status:** implemented — `app/src/settings_view/specsmith_page.rs` ESDB subview

## 15. Skills Settings Page
- **ID:** REQ-015
- **Title:** Skills Settings Page
- **Description:** The Kairos Settings → Specsmith → Skills page MUST render a header, description, and CLI hint for `specsmith skills list/build/activate` without requiring a live specsmith connection.
- **Source:** ARCHITECTURE.md §Kairos Settings Extensions
- **Status:** implemented — `app/src/settings_view/specsmith_page.rs` Skills subview

## 16. Eval Settings Page
- **ID:** REQ-016
- **Title:** Eval Settings Page
- **Description:** The Kairos Settings → Specsmith → Eval page MUST render a header, description, and CLI hint for `specsmith eval run/report` without requiring a live specsmith connection.
- **Source:** ARCHITECTURE.md §Kairos Settings Extensions
- **Status:** implemented — `app/src/settings_view/specsmith_page.rs` Eval subview

## 17. MCP AI Builder Card
- **ID:** REQ-017
- **Title:** MCP AI Builder Card
- **Description:** The Kairos Agents → MCP servers page MUST include a collapsible AI Builder card that: (1) accepts a natural-language description, (2) generates a server stub via `specsmith mcp generate <desc>`, (3) displays the JSON, (4) appends to `~/.specsmith/mcp.json` on user confirmation.
- **Source:** ARCHITECTURE.md §Kairos Settings Extensions
- **Status:** implemented — `app/src/settings_view/mcp_page.rs` AI Builder card

## 18. specsmith YAML Governance CI Gate
- **ID:** REQ-018
- **Title:** specsmith YAML Governance CI Gate
- **Description:** The Kairos CI `governance` job MUST install specsmith and run `specsmith validate --strict --project-dir .` and `specsmith sync --check --project-dir .` to enforce governance schema integrity and machine-state sync on every push. Failures MUST block the CI.
- **Source:** ARCHITECTURE.md §specsmith YAML Governance Awareness
- **Status:** implemented — `.github/workflows/ci.yml` `governance` job runs `specsmith validate --strict --json` and `specsmith sync --check` on every push; both steps block the CI on failure
