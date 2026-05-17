# Kairos Architecture

## What Kairos Is

Kairos is a **completed fork of the open-source Warp terminal** (via OpenWarp/zerx-lab)
with all Warp cloud dependencies removed and specsmith governance wired in.

As of 2026-05-07:
1. **All Warp cloud/AI services are runtime-dead** — no Warp Drive, no cloud agents,
   no telemetry, no Warp account/login. GraphQL is permanently stubbed.
2. **specsmith AEE governance is wired** — GovernanceServer spawns at startup;
   BYOE defaults to `http://127.0.0.1:7700/v1/`.
3. **Kairos brand is complete** — name, icon, wordmark, Kairos Amber theme.
4. **cargo check -p kairos passes** — 0 errors on Rust 1.92 stable (Windows MSVC).

Kairos is **not** a standalone TUI app or a from-scratch terminal. It is a governed
fork of a proven, production terminal with a new AI governance architecture.

## Fork Lineage

```
warpdotdev/warp  (AGPL-3.0 + MIT for warpui)
        │
        ▼
zerx-lab/warp (OpenWarp)  — adds BYOE: custom provider endpoints
        │                    removes cloud agent defaults
        ▼
BitConcepts/kairos         — removes ALL remaining Warp cloud/AI
                             wires specsmith as the BYOE endpoint
                             Kairos brand, colors, theme
```

**License:** AGPL-3.0 (inherited from Warp/OpenWarp) + MIT (warpui crates).
This means Kairos is open source. specsmith (the governance backend) is MIT / commercial.

## System Architecture

```
┌─────────────────────────────────────────────────────────┐
│                  Kairos Terminal (Rust)                  │
│  (OpenWarp fork — Warp UI, shell, blocks, workflows)     │
│                                                          │
│  BYOE config:                                            │
│    base_url = http://127.0.0.1:7700                      │
│    ← points at specsmith governance-serve, NOT OpenAI    │
└──────────────────────┬──────────────────────────────────┘
                       │ POST /v1/chat/completions
                       │ (OpenAI-compatible)
┌──────────────────────▼──────────────────────────────────┐
│           specsmith governance-serve (Python)            │
│                                                          │
│  1. Intercept request, extract utterance                 │
│  2. POST /preflight  → governance gate                   │
│     if accepted: forward to real AI provider             │
│     if not accepted: return governance refusal msg       │
│  3. POST /verify  → post-response confidence check       │
│  4. Return OpenAI-compatible response to Kairos          │
│                                                          │
│  Also exposes:                                           │
│    GET  /health     — liveness probe                     │
│    POST /preflight  — direct governance gate             │
│    POST /verify     — direct verification                │
└──────────────────────┬──────────────────────────────────┘
                       │ forward (if preflight accepted)
                       │ KAIROS_AI_BASE_URL
┌──────────────────────▼──────────────────────────────────┐
│           Real AI Provider (user-configured)             │
│   Ollama (local) / vLLM / Anthropic / OpenAI / DeepSeek │
│   — any OpenAI-compatible endpoint                       │
└─────────────────────────────────────────────────────────┘
```

**Governance state** is owned exclusively by specsmith.  
**Terminal UX** is owned by Kairos (the Warp fork).  
**AI calls** flow Kairos → specsmith (governance) → real AI provider.

## What Gets Removed from OpenWarp / Warp

| Component | Status | Replacement |
|-----------|--------|-------------|
| Warp Drive (cloud sync) | **Remove** | Local governance via specsmith |
| Warp AI cloud agents | **Remove** | specsmith governance-serve |
| Warp account / login | **Remove** | No account required |
| Warp telemetry / analytics | **Remove** | None |
| OpenAI-powered agentic workflows | **Remove** | specsmith AEE governance |
| BYOE default (zerx-lab) | **Replace** | Point at `127.0.0.1:7700` |
| Warp branding (name, colors, logo) | **Replace** | Kairos brand |

## What Gets Added

| Component | Description |
|-----------|-------------|
| `src/governance/` crate | GovernanceClient, GovernanceServer (already built in this stub repo) |
| Governance server auto-spawn | GovernanceServer::spawn() called at Kairos startup |
| BYOE default → specsmith | Kairos BYOE configured to `http://127.0.0.1:7700` out of box |
| Governance Settings page | Native Rust settings page (Settings → Governance) showing live engine health, channel selector, context window, CI/CD status, and specsmith updater (REQ-005) |
| Kairos theme | Custom colors/brand (to be designed — NOT Warp blue/purple) |

## Kairos Brand

- **Name**: Kairos (Greek: the opportune moment for action)
- **Tagline**: *A fully local, governance-ready terminal*
- **Colors**: Amber `#F5A623` accent on near-black `#0D0D10` background
- **Theme**: `ThemeKind::KairosAmber` — bundled default in the theme chooser
- **Icon**: Arc + chevron + dash mark (amber on black)
- **Wordmark**: Geometric Inter SemiBold, white divider, amber mark
- **Contact**: info@bitconcepts.tech
- **GitHub**: github.com/BitConcepts/kairos

## Current Repository State

The `BitConcepts/kairos` repo IS the terminal fork (not a stub). As of 2026-05-07:

- `app/` — Full Warp/OpenWarp terminal application (Rust, ~50k files)
- `crates/kairos-governance/` — GovernanceClient, GovernanceServer, SessionConfig
- `app/src/bin/oss.rs` — Kairos binary entry point; spawns GovernanceServer at startup
- `app/src/settings_view/governance_page.rs` — Settings → Governance panel
- `app/i18n/en/kairos.ftl` — i18n locale (MUST be named after Cargo package name)

**Fork setup is complete.** `docs/FORK-SETUP.md` has been removed (obsolete).

## Architecture Invariants
- **I1**: Kairos MUST NOT call any LLM API directly. All AI goes through specsmith.
- **I2**: All governance HTTP calls MUST target `127.0.0.1` only.
- **I3**: `specsmith governance-serve` MUST be spawned as a managed child process at startup.
- **I4**: The governance dashboard panel MUST be testable via the Kairos integration framework (`crates/integration`). The panel is implemented as a native Rust settings page (not a WebView), so coverage is provided by `test_governance_page_renders` (full UI, real-display) and `crates/kairos-governance/tests/governance_tests.rs` (health client unit tests).
- **I5**: Kairos MUST compile on Rust stable with no nightly-only feature flags.
- **I6**: No Warp cloud service calls may remain in the forked codebase.
- **I7**: BYOE default endpoint MUST be `http://127.0.0.1:7700` (specsmith governance-serve).

## Per-Project Shell Memory
Source: `app/src/kairos_shell_memory.rs`

When the user opens a new tab with an explicit shell (`AddTabWithShell`), Kairos persists that choice to `.kairos/shell-pref.json` in the nearest **project root** so subsequent `AddDefaultTab` calls in the same project open the same shell automatically — without touching the global startup-shell setting.

**Project root detection** (`find_project_root`): walks up from the active pane's working directory until it finds `.git`, `.kairos`, or `scaffold.yml`. Returns the current directory if none is found.

**File format** (`.kairos/shell-pref.json`):
```json
{ "shell": { "WSL": "Ubuntu-24.04" } }
```
The `shell` field is a serialised `NewSessionShell` value covering all variants (SystemDefault, Executable, WSL, MSYS2, Custom).

**Hook points in `workspace/view.rs`**:
- `add_tab_with_shell` — calls `save_shell_pref(cwd, &NewSessionShell::from(shell))` after telemetry, gated on `#[cfg(feature = "local_tty")]`.
- `AddDefaultTab` Terminal/Agent path — calls `load_shell_pref(cwd)`, resolves the stored `NewSessionShell` via `AvailableShells::matches_preference`, and uses the returned `AvailableShell` instead of the global default. Falls back to the existing welcome-tab / `add_terminal_tab` path when no preference is found.

**Scope note:** `<project>/.kairos/` (per-project governance data) is distinct from the global Kairos app config dir (renamed from `.openwarp`). Both use the `.kairos` name at different filesystem levels; the project root walk anchors usage unambiguously.

## SSH Integration (formerly "Warpify")
The SSH integration subsystem allows Kairos to add block-based input modes and shell integration to SSH sessions. It was previously called "Warpify" throughout the codebase; all user-visible strings and keybinding descriptions now use "SSH Integration" / "integrate" / "Integration".

**Affected files:**
- `app/i18n/en/kairos.ftl` — all `settings-warpify-*`, `terminal-warpify-*`, and `keybinding-desc-*warpify*` keys.
- `app/i18n/zh-CN/kairos.ftl` — same keys in Chinese.
- `app/src/settings_view/mod.rs` — `SettingsSection::Warpify` `FromStr` accepts both `"Warpify"` (backward compat) and `"SSH Integration"` / `"SSH integration"`.

The SSH bootstrap path (`app/src/terminal/ssh/warpify.rs`, `app/src/terminal/warpify/`) is retained unchanged. Only user-facing strings are updated.

## Context Window Management
See specsmith `src/specsmith/context_window.py` for the shared Python implementation.

**Kairos side (REQ-009–011):**
- `GovernanceSettings` settings struct carries `ollama_num_ctx: u32`,
  `context_compression_threshold_pct: u8`, `context_auto_compress: bool`.
- `use_agent_footer` area renders a compact context fill progress bar
  subscribed to `TerminalModel` context fill state.
- `WorkspaceView` listens for `context_fill` JSONL events from the agent
  stream: fires `SummarizeAIConversation` when `pct >= compression_threshold`
  (REQ-010), and forces emergency compression when `pct >= 85` (REQ-011).
- GPU detection for Ollama `num_ctx` recommendation surfaces in the
  Governance settings page under an "Ollama Context" card (REQ-009).

**Invariant:** context window MUST NOT reach 100% fill. A hard reservation
of 15% (minimum 2048 tokens) is enforced in the agent runner before any
user input is accepted.

## AI Model Intelligence Panel

Kairos surfaces specsmith's AI Model Intelligence layer in the Agents UI.

**Providers Table — Bucket Score Columns (REQ-012)**

The **Agents → AI Providers** table includes three additional columns:
- **R** (reasoning score 0–100) — from specsmith HF leaderboard sync
- **C** (conversational score 0–100)
- **L** (longform score 0–100)

A **Sync Scores** button triggers `GET /api/model-intel/sync` on the
specsmith governance server and refreshes the table without interrupting
the active session.

Data source: `GET /api/model-intel/scores` (REQ-013) returns
`{"scores": [{"model_name": ..., "reasoning_score": ..., ...}]}`.
`GET /api/model-intel/recommendations?bucket=reasoning` returns the
top-10 models for a given task bucket.

Implemented in `app/src/settings_view/ai_providers_page.rs`.

## Kairos Settings Extensions

The Kairos settings view includes Specsmith-specific pages grouped under a
**Specsmith** umbrella in the sidebar:

- **ESDB** (REQ-014) — database status (backend, record count, chain
  validity), action buttons (Refresh, Export JSON, Import, Backup,
  Rollback, Compact), all via `specsmith esdb *` commands.
- **Skills** (REQ-015) — header, description, and CLI hint for
  `specsmith skills list/build/activate`.
- **Eval** (REQ-016) — header, description, and CLI hint for
  `specsmith eval run/report`.
- **MCP AI Builder** (REQ-017) — collapsible card that accepts a
  description, generates a stub via `specsmith mcp generate <desc>`,
  displays the JSON, and appends to `~/.specsmith/mcp.json` on click.

## specsmith YAML Governance Awareness

Kairos CI installs specsmith and runs `specsmith validate --strict` and
`specsmith sync --check` on the docs/ governance files (REQ-018).

When specsmith operates in YAML-first mode (`.specsmith/governance-mode = yaml`):
- `docs/REQUIREMENTS.md` and `docs/TESTS.md` are **generated artifacts**
  — do not hand-edit them. Edit `docs/requirements/*.yml` instead.
- `specsmith generate docs` regenerates the Markdown from YAML.
- `specsmith validate --strict` enforces schema integrity.
- The CI `governance` job runs these checks on every push.

## Sister Repo
`specsmith` lives at `../specsmith/` relative to this repository.
Both repos are always cloned to the same parent directory.
See `AGENTS.md §Sister Repos` for co-management details.
