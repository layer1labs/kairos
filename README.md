# Kairos

> Epistemically-governed terminal runtime.

Kairos is a Rust-based terminal built on a [Warp](https://github.com/warpdotdev/warp)
BYOE fork. It uses [specsmith](https://github.com/BitConcepts/specsmith) as its
governance backend via the `specsmith serve` REST/WebSocket API. Every governed action
is preflight-checked and post-verified through the Applied Epistemic Engineering (AEE)
protocol.

## Architecture

```
specsmith serve ──(HTTP/WS)──► Kairos terminal
  │ preflight                    │ shell / REPL
  │ verify                       │ WebView dashboard
  └ audit                        └ Playwright tests
```

Governance state lives exclusively in specsmith. Kairos owns the terminal UX, agent
execution runtime, and client integration.

## Quick Start

```sh
# Start the governance backend (Python)
py -m specsmith serve

# Build and run Kairos (requires Rust stable)
cargo run
```

## Session Start (for AI agents)

1. Read `AGENTS.md` — governance hub and quick commands
2. Read `LEDGER.md` — last session state and open TODOs
3. Run `py -m specsmith audit --project-dir .` — governance health check
4. Ensure `specsmith serve` is running before any governed action

## Integration Contract

| Endpoint               | Purpose                                  |
|------------------------|------------------------------------------|
| `GET  /health`         | Backend liveness                         |
| `POST /preflight`      | Classify + gate an utterance             |
| `POST /verify`         | Post-change confidence check             |
| `GET  /audit`          | Governance audit results                 |
| `WS   /ws/session/{id}`| Live session I/O (JSONL event stream)    |

All calls go to `http://127.0.0.1:7700` by default.

## Repository Layout

```
kairos/
├── src/
│   ├── main.rs          # Binary entry point
│   └── governance/      # specsmith REST client (planned)
├── tests/               # Rust integration tests
├── e2e/                 # Playwright end-to-end tests (planned)
├── .github/workflows/   # CI (build + governance audit)
├── .specsmith/          # Machine state (managed by specsmith)
├── AGENTS.md            # Governance hub for AI agents
├── REQUIREMENTS.md      # Formal requirements (authoritative)
├── TESTS.md             # Test specifications (authoritative)
└── LEDGER.md            # Session ledger
```

## License

MIT © BitConcepts
