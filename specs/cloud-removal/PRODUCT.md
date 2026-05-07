# Cloud Removal — Kairos Clean Base

## Summary

Strip all Warp cloud/account/telemetry/billing infrastructure from Kairos to
produce a self-contained terminal that makes zero network calls to Warp servers.
The specsmith governance layer replaces the cloud AI.

## Behavior

### After removal the user sees

1. Terminal launches without a login screen. No account required.
2. No BYOK/BYOP paywall. AI provider is freely configurable.
3. BYOP default endpoint is `http://127.0.0.1:7700` (specsmith governance-serve).
4. No Warp Drive panel, no cloud sync, no shared notebooks.
5. No telemetry, analytics, crash uploads, or server-driven experiment flags.
6. No pricing pages, upgrade CTAs, or billing tier checks anywhere.
7. All remaining functionality works fully offline and locally.

### What is kept unchanged

- Terminal core: shell, blocks, input, completions, themes, settings
- BYOP infrastructure (openai_compatible.rs, genai adapter) — rewired to specsmith
- Skills, workflows, tab configs — local features
- SSH, remote server — local feature
- MCP client — local feature
- Vim, search, clipboard, split panes — local features

---

## Removal Inventory

### Phase 1 — Safe deletions (zero compile impact) ✅ IN PROGRESS

| Item | Status |
|------|--------|
| `.claude/`, `.deepseek/`, `CLAUDE.md` | ✅ Done |
| `.mcp.json` | ✅ Done |
| `.agents/skills/add-telemetry/` | ✅ Done |
| `website/` | ✅ Done |
| `FAQ.md` (Warp cloud FAQ) | ✅ Done |
| OpenWarp `specs/` | ✅ Done (Kairos specs replace them) |
| `WARP.md` → renamed `DEVELOPMENT.md` | ✅ Done |
| `.warpindexingignore` | ✅ Done |
| `README.zh-CN.md` | ✅ Done |
| `.zed/` | ✅ Done |
| `about.hbs`, `about.toml` | ✅ Done (will create Kairos About) |
| `diesel.toml` | ✅ Done |

### Phase 2 — Break cloud connectivity (requires compile fixes)

Strategy: stub entry points first, then delete dead code.

| Module | Size | Action |
|--------|------|--------|
| `crates/graphql/` | crate | Stub → delete |
| `crates/warp_server_client/` | crate | Stub → delete |
| `app/src/auth/` | 21 files | Stub to anonymous → delete |
| `app/src/workspaces/` | 10 files | Stub billing gates OFF → delete |

**Target:** `grep -r "warp\.dev" app/src/ \| grep -v test` returns empty.

### Phase 3 — Remove cloud-dependent features

| Module | Size | Action |
|--------|------|--------|
| `app/src/drive/` | 45 files | Delete |
| `app/src/notebooks/` | 30 files | Delete |
| `app/src/ai/cloud_agent_config/` | dir | Delete |
| `app/src/ai/cloud_environments/` | dir | Delete |
| `app/src/ai/ambient_agents/` | dir | Delete |
| `crates/computer_use/` | crate | Delete |
| `app/src/server/` | 56 files | Delete |
| `app/src/pricing/` | 1 file | Delete |
| `app/src/resource_center/` | 10 files | Delete |
| `app/src/experiments/` | 7 files | Delete |
| `app/src/crash_reporting/` | 4 files | Remove upload, keep local log |
| `app/src/linear.rs` | 1 file | Delete |
| `app/src/tips/` | 3 files | Delete |

### Phase 4 — Wire specsmith governance

| Change | Location |
|--------|----------|
| BYOP default → `http://127.0.0.1:7700` | `app/src/ai/llms.rs` |
| Remove OpenAI hardcoded provider | `app/src/ai/llms.rs` |
| Remove BYOK billing gate | `app/src/workspaces/user_workspaces.rs` |
| Wire GovernanceServer spawn | startup / `app/src/main.rs` |
| Add governance WebView panel | `app/src/settings_view/` |

### Phase 5 — Rebrand

| Change | Location |
|--------|----------|
| App name Warp → Kairos | manifests, metadata |
| Color theme (amber/gold) | `themes/` |
| Logo / icons | `assets/` |
| Cargo package name | `app/Cargo.toml` |
| UI strings | `i18n/` |

---

## Removal Rules

1. **Stub before delete** — never delete a module with widespread imports until
   everything that depends on it compiles against a stub.
2. **One phase per commit** — each phase produces a buildable commit.
3. **Test after each phase** — `cargo check -p app` must pass.

---

## Success Criteria

- [ ] `cargo build` passes
- [ ] `grep -r "warp\.dev" app/src/ | grep -v test` returns empty
- [ ] Terminal launches without login
- [ ] BYOP default is `http://127.0.0.1:7700`
- [ ] `specsmith governance-serve` spawns at start
- [ ] Zero runtime calls to Warp servers
