# Kairos Architecture

## What Kairos Is

Kairos is a **fork of the open-source Warp terminal** ([zerx-lab/warp](https://github.com/zerx-lab/warp),
also known as OpenWarp) with:

1. **All Warp cloud/AI services removed** — no Warp Drive, no cloud agents, no telemetry,
   no Warp account/login, no OpenAI-sponsored GPT workflows.
2. **specsmith AEE governance wired in** — every AI request is gated by the specsmith
   governance layer before reaching any real AI model.
3. **Kairos brand** — distinct name, colors, and theme. Not Warp.

Kairos is **not** a standalone TUI app or a from-scratch terminal. It is a governed
fork of a proven, production terminal with a new AI governance architecture.

## Fork Lineage

```
warpdotdev/warp  (AGPL-3.0 + MIT for warpui)
        │
        ▼
zerx-lab/warp (OpenWarp)  — adds BYOP: custom provider endpoints
        │                    removes cloud agent defaults
        ▼
BitConcepts/kairos         — removes ALL remaining Warp cloud/AI
                             wires specsmith as the BYOP endpoint
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
│  BYOP config:                                            │
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
| BYOP default (zerx-lab) | **Replace** | Point at `127.0.0.1:7700` |
| Warp branding (name, colors, logo) | **Replace** | Kairos brand |

## What Gets Added

| Component | Description |
|-----------|-------------|
| `src/governance/` crate | GovernanceClient, GovernanceServer (already built in this stub repo) |
| Governance server auto-spawn | GovernanceServer::spawn() called at Kairos startup |
| BYOP default → specsmith | Kairos BYOP configured to `http://127.0.0.1:7700` out of box |
| Governance WebView panel | Settings panel showing phase, confidence, open work items (REQ-005) |
| Kairos theme | Custom colors/brand (to be designed — NOT Warp blue/purple) |

## Kairos Brand Direction

- **Name**: Kairos (Greek: the opportune moment for action)
- **Tagline**: *Intelligence proposes. Governance decides.*
- **Color palette**: TBD — warm amber/gold tones (distinct from Warp's blue/purple)
- **Font**: Inherit from Warp (WarpMono / system mono), update config
- **UI mode**: Terminal-first (minimal, high-information-density)
- **Logo**: Hourglass or compass — reflects the temporal/decision theme of AEE

## This Repo (Development Stub)

The current `BitConcepts/kairos` repo is a **governance module stub** built during
architecture and requirements phases. It contains:

- `src/governance/` — GovernanceClient + GovernanceServer (correct, will be embedded in fork)
- `src/session.rs` — SessionConfig, find_specsmith_cmd (correct, will be embedded)
- `src/main.rs` — Development test daemon (NOT the real terminal binary)

**To set up the real Kairos fork**, see `docs/FORK-SETUP.md`.

## Architecture Invariants
- **I1**: Kairos MUST NOT call any LLM API directly. All AI goes through specsmith.
- **I2**: All governance HTTP calls MUST target `127.0.0.1` only.
- **I3**: `specsmith governance-serve` MUST be spawned as a managed child process at startup.
- **I4**: The governance dashboard panel MUST be Playwright-testable (Warp WebView).
- **I5**: Kairos MUST compile on Rust stable with no nightly-only feature flags.
- **I6**: No Warp cloud service calls may remain in the forked codebase.
- **I7**: BYOP default endpoint MUST be `http://127.0.0.1:7700` (specsmith governance-serve).

## Sister Repo
`specsmith` lives at `../specsmith/` relative to this repository.
Both repos are always cloned to the same parent directory.
See `AGENTS.md §Sister Repos` for co-management details.
