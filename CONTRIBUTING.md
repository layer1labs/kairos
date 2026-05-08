# Contributing to Kairos

Thanks for helping improve Kairos! This guide explains how to open issues, propose changes, and get your work reviewed.

**Contact:** [info@bitconcepts.tech](mailto:info@bitconcepts.tech)
**Issues:** [github.com/BitConcepts/kairos/issues](https://github.com/BitConcepts/kairos/issues)

---

## Quick Start

```powershell
# Windows
winget install Rustlang.Rustup
winget install Google.Protobuf
.\Open-Kairos.ps1           # build and launch
```

```bash
# macOS / Linux
./script/bootstrap
./script/run
./script/presubmit          # fmt + clippy + tests
```

Always target the `kairos` binary:

```bash
cargo build --bin kairos
cargo run   --bin kairos
```

> Do not use `--bin warp`, `--bin stable`, `--bin dev`, or `--bin preview` —
> those require Warp's private channel-config binary and will panic at startup.

---

## How Contributing Works

- **Issues first.** Open an issue before a PR for anything non-trivial.
- **Bug fixes** can go straight to a PR — no pre-approval needed.
- **Features** need a brief issue discussion first so we can agree on scope.
- PRs are reviewed by BitConcepts maintainers.

## Routing Bugs Correctly

| Bug type | Where to file |
|----------|---------------|
| Terminal / UI / rendering / shell / keybindings | [BitConcepts/kairos](https://github.com/BitConcepts/kairos/issues/new?template=bug_report.md) |
| AI governance / specsmith / BYOE / preflight | [BitConcepts/specsmith](https://github.com/BitConcepts/specsmith/issues/new?template=bug_report.md) |

## Bug Report Checklist

- Clear title and one-paragraph summary
- Steps to reproduce (minimal example where possible)
- Expected vs. actual behavior
- Kairos version (`Settings → About`) and OS
- Logs: `%APPDATA%\kairos\logs\` (Windows) or `~/.local/share/kairos/logs/`

## Opening a PR

1. Branch from `main`
2. Implement the change and add tests
3. Run `./script/presubmit` (Windows: `cargo fmt && cargo clippy --workspace -- -D warnings && cargo test`)
4. Open a PR — describe **what** and **why**

## Testing

```bash
cargo test -p kairos-governance   # governance crate tests (fast, ~4s)
cargo check -p kairos --bin kairos # full app compile check (~2min cold)
```

- Bug fixes should include a regression test
- Logic changes need unit tests
- Governance integration tests live in `crates/kairos-governance/src/` (inline) and integration tests in the crate

## Code Style

- `cargo fmt` and `cargo clippy --workspace --all-targets -- -D warnings` must pass
- Follow existing patterns in the file you are editing
- Exhaustive `match` over `_` wildcards on enums
- Commit prefix: `feat:`, `fix:`, `build:`, `docs:`, `chore:`, `refactor:`
- For AI-assisted commits add: `Co-Authored-By: Oz <oz-agent@warp.dev>`

## Code of Conduct

See [`CODE_OF_CONDUCT.md`](CODE_OF_CONDUCT.md). Report violations to [info@bitconcepts.tech](mailto:info@bitconcepts.tech).

## Security

See [`SECURITY.md`](SECURITY.md). Report vulnerabilities privately — **do not open public issues for security bugs.**

## Getting Help

- [GitHub Issues](https://github.com/BitConcepts/kairos/issues)
- Email: [info@bitconcepts.tech](mailto:info@bitconcepts.tech)
- Full engineering guide: [`DEVELOPMENT.md`](DEVELOPMENT.md)
