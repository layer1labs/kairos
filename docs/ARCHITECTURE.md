# Kairos Architecture

## Overview
Kairos is a Rust-based terminal runtime governed by the Applied Epistemic Engineering
(AEE) protocol. All governance intelligence lives in
[specsmith](https://github.com/BitConcepts/specsmith); Kairos provides the terminal
UX, agent execution runtime, and a WebView-based governance dashboard.

## Separation of Concerns

```
┌─────────────────────────────────────┐
│           Kairos (Rust)             │
│  terminal UX / shell / REPL         │
│  agent execution runtime            │
│  WebView governance dashboard       │
│  Playwright-testable governance UI  │
└────────────┬────────────────────────┘
             │ HTTP/WebSocket (local only)
             │ 127.0.0.1:7700
┌────────────▼────────────────────────┐
│      specsmith serve (Python)       │
│  preflight / verify / audit         │
│  requirements / test coverage       │
│  confidence scoring / ledger        │
│  trace vault / phase tracking       │
└─────────────────────────────────────┘
```

Governance state is owned exclusively by specsmith. Kairos owns the execution surface.

## System Components

### src/main.rs — Binary Entry Point
Initializes the terminal runtime, spawns `specsmith serve` as a managed child process,
and enters the main event loop.

### src/governance/ — specsmith REST Client (planned)
Async reqwest/tokio HTTP client. Calls:
- `GET  /health` — liveness check at startup
- `POST /preflight` — gate any governed action before execution
- `POST /verify` — post-change confidence check after execution
- `WS   /ws/session/{id}` — live session I/O stream

### src/webview/ — Governance Dashboard (planned)
WebView panel (wry on Linux/macOS, webview2 on Windows) rendering the governance
dashboard: audit state, phase, confidence scores, open work items. Implemented as a
WebView so Playwright can drive it for end-to-end tests.

### e2e/ — Playwright Tests (planned)
End-to-end tests driving the WebView dashboard to verify governance state is correctly
displayed. CI runs these against a live `specsmith serve` instance.

## Architecture Invariants
- **I1**: Kairos MUST NOT call any LLM API directly. All LLM interaction goes through
  `specsmith serve`.
- **I2**: All governance HTTP calls MUST target `127.0.0.1` only.
- **I3**: `specsmith serve` MUST be started as a managed child process; its lifecycle is
  owned by Kairos.
- **I4**: The governance WebView panel MUST be Playwright-testable (no native-only UI).
- **I5**: Kairos MUST compile on Rust stable with no nightly-only feature flags.

## Sister Repo
specsmith lives at `../specsmith/` relative to this repository.
Both repos are always cloned to the same parent directory.
See `AGENTS.md §Sister Repos` for co-management details.

## Technology Decisions
- **Rust stable**: avoids nightly-only instability, maximum portability.
- **reqwest 0.12 + tokio**: standard async HTTP/WebSocket client for the governance layer.
- **wry** (planned): cross-platform WebView crate for the dashboard panel.
- **Playwright** (planned): enables governance dashboard CI testing without a display server.
- **specsmith serve** as governance backend: avoids duplicating governance logic in Rust;
  allows specsmith to evolve independently.
