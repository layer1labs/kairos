<div align="center">

# Kairos

**A fully local, governance-ready terminal — your AI, your agents, your rules, your machine.**

Kairos is a fork of [Warp](https://github.com/warpdotdev/warp) (via
[OpenWarp](https://github.com/zerx-lab/openwarp)) that strips all mandatory
cloud dependencies, wires in [specsmith](https://github.com/BitConcepts/specsmith)
as a local AI governance engine, and opens the AI provider layer to any
OpenAI-compatible endpoint.

> Early development. No official release yet. **Not affiliated with Warp, Inc.**

</div>

---

## What Kairos is

Upstream Warp requires a Warp account, Warp's cloud servers, and Warp's AI gateway.
Kairos removes all of that and replaces it with a locally-governed stack:

| | Upstream Warp | Kairos |
| --- | --- | --- |
| Cloud dependency | Hard dependency on Warp backend | **Zero — no account, no login, no cloud calls** |
| AI governance | Warp cloud + opaque server rules | **[specsmith](https://github.com/BitConcepts/specsmith) local governance engine** |
| AI provider | Warp gateway only | **Any OpenAI-compatible endpoint, BYOP** |
| Default BYOP endpoint | `warp.dev` servers | **`http://127.0.0.1:7700` (local specsmith)** |
| Credentials | Cloud account | **Local config, never leaves the device** |
| Bug reporting | Warp feedback form | **GitHub Issues: kairos or specsmith repo** |
| Blocks / Workflows / Keymaps | Kept | **Fully preserved** |
| Terminal core | Full Warp UX | **Full Warp UX** |
| License | AGPL-3.0 / MIT dual | **Same (see [LICENSE](LICENSE))** |

## How it works

**01 · specsmith governance-serve starts locally**
At launch, Kairos spawns `specsmith governance-serve` as a managed child process
on port 7700. This is the local AI governance backend — preflight checks,
verification, confidence scoring, and audit all run on your machine.

**02 · BYOP wired to localhost by default**
The AI provider endpoint defaults to `http://127.0.0.1:7700/v1/`. Point it
at any OpenAI-compatible endpoint in Settings → AI if you want a different
model or provider. Credentials are stored locally only.

**03 · Full Warp terminal experience, zero cloud**
Blocks, Workflows, AI agent sessions, themes, keymaps, SSH manager — all
the Warp UX you know, running entirely offline without any server dependency.

## Core features

- **Local AI governance** — [specsmith](https://github.com/BitConcepts/specsmith)
  runs preflight and verification checks locally before and after every governed action
- **BYOP** — any OpenAI Chat Completions-compatible endpoint works out of the box;
  6 native protocols via [genai](https://github.com/jeremychone/rust-genai)
- **Zero forced login** — `skip_login` is always active; no Warp account required
- **Zero telemetry** — all analytics, crash uploads, and experiment flags disabled
- **Bug reporting via GitHub Issues** — terminal bugs → [kairos](https://github.com/BitConcepts/kairos/issues),
  AI/governance bugs → [specsmith](https://github.com/BitConcepts/specsmith/issues)
- **Kairos Amber theme** — official brand theme (`#F5A623` amber on `#0D0D10`)
- **Full Warp UX preserved** — Blocks, Workflows, AI commands, Keymaps, SSH manager,
  themes, split panes, MCP client — all kept and working

## Verified AI providers

| Provider | Base URL | Notes |
| --- | --- | --- |
| **specsmith (local)** | `http://127.0.0.1:7700/v1` | Default — governance-aware local endpoint |
| **OpenAI** | `https://api.openai.com/v1` | Direct |
| **Anthropic** | via genai native | Claude family |
| **DeepSeek** | `https://api.deepseek.com/v1` | thinking + tool calling |
| **Gemini** | via genai native | Google AI Studio |
| **Ollama** | `http://localhost:11434/v1` | Local inference, no key |
| **OpenRouter** | `https://openrouter.ai/api/v1` | Aggregator gateway |

## Build from source

Requires: **Rust 1.92+** (via [rustup](https://rustup.rs)), **protoc** (for proto API crates).

```bash
git clone https://github.com/BitConcepts/kairos
cd kairos
./script/bootstrap   # platform-specific deps (macOS/Linux)
./script/run         # build & run
./script/presubmit   # fmt / clippy / tests
```

Windows: install deps via winget first:
```powershell
winget install Rustlang.Rustup
winget install Google.Protobuf
```

Always target the `kairos` binary explicitly:
```bash
cargo build --release --bin kairos
cargo run   --release --bin kairos
```

> Do not run `cargo build --release --bin {warp,stable,dev,preview}` — those
> entry points require Warp's private `warp-channel-config` binary and will
> panic at startup. Use `kairos` only.

See [DEVELOPMENT.md](DEVELOPMENT.md) for the full engineering guide.

## License

Kairos inherits the dual-license structure from Warp. See [LICENSE](LICENSE)
for the full breakdown:

- `crates/warpui_core` / `crates/warpui` — [MIT](LICENSE-MIT)
  (Copyright 2020-2026 Denver Technologies, Inc.)
- All other upstream Warp code — [AGPL-3.0](LICENSE-AGPL)
  (Copyright 2020-2026 Denver Technologies, Inc.)
- Kairos-specific additions (`crates/kairos-governance`, `specs/`, `.github/`,
  `themes/kairos_amber.yaml`, and files with a BitConcepts copyright header) — MIT
  (Copyright 2026 BitConcepts)

## Contributing

Community contributions welcome. See [CONTRIBUTING.md](CONTRIBUTING.md) for the full flow.

Before filing, please [search existing issues](https://github.com/BitConcepts/kairos/issues).
Security vulnerabilities should be reported privately per
[CONTRIBUTING.md#reporting-security-issues](CONTRIBUTING.md#reporting-security-issues).

## Acknowledgements

Kairos stands on the shoulders of many teams and open-source projects:

**Core foundation**
[Warp](https://github.com/warpdotdev/warp) — the terminal this is forked from, built by Warp, Inc.
[OpenWarp](https://github.com/zerx-lab/openwarp) — the community fork that first removed cloud dependencies, laying the groundwork for Kairos.

**Kairos governance**
[specsmith](https://github.com/BitConcepts/specsmith) — the local AI governance engine powering Kairos.

**Key dependencies**
[genai](https://github.com/jeremychone/rust-genai) · [Tokio](https://github.com/tokio-rs/tokio) · [minijinja](https://github.com/mitsuhiko/minijinja) · [cosmic-text](https://github.com/pop-os/cosmic-text) · [Alacritty VTE](https://github.com/alacritty/vte) · [Hyper](https://github.com/hyperium/hyper) · [reqwest](https://github.com/seanmonstar/reqwest) · [wgpu](https://github.com/gfx-rs/wgpu)
