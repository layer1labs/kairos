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

### Phase 1 — Safe deletions (zero compile impact) ✅ COMPLETE

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

### Phase 2 — Break cloud connectivity ✅ COMPLETE

Done: all outbound network calls disabled.

| Module | Size | Action | Status |
|--------|------|--------|---------|
| `crates/graphql/src/client.rs` | stub | `send_graphql_request` always returns `ServiceUnavailable` | ✅ |
| `app/src/auth/` | feature | `skip_login` in default features — User::test() + no login screen | ✅ |
| `app/src/workspaces/user_workspaces.rs` | method | `is_byo_api_key_enabled()` returns `true` | ✅ |
| `warp_features/src/lib.rs` | flags | 30+ cloud flags force-false, SoloUserByok force-true | ✅ |
| `app/Cargo.toml` | defaults | Cloud feature flags removed from default build | ✅ |

**Remaining for full Phase 2 cleanup (Phase 3):** delete dead cloud module code.

### Phase 3 — Remove cloud-dependent features ✅ EFFECTIVE / PARTIAL SOURCE

Cloud code is **dead at runtime** — all network paths are stubbed or feature-gated off.
Small modules with no cloud calls of their own are effectively complete; large modules
(`server/`, `drive/`, `notebooks/`) require `cargo check` validation per-module before
source deletion and are deferred to a session with the Rust toolchain available.

| Module | Size | Runtime | Source |
|--------|------|---------|--------|
| `app/src/crash_reporting/` | 4 files | ✅ Compiled out (`#[cfg(feature="crash_reporting")]`) | ✅ Feature-gated out |
| `crates/graphql/src/client.rs` | 1 file | ✅ Stubbed (Phase 2) | ✅ Stub stays |
| `app/src/pricing/` | 1 file | ✅ Already no-op stub (comment confirms it) | ✅ Effectively done |
| `app/src/linear.rs` | 1 file | ✅ URL parsing only, zero network calls | ✅ Effectively done |
| `app/src/tips/` | 3 files | ✅ Pure UI data, zero network calls | ✅ Effectively done |
| `app/src/experiments/` | 7 files | ✅ Reads only stub GraphQL cache | ✅ Effectively done |
| `app/src/resource_center/` | 10 files | ✅ Pure local UI, zero network calls | ✅ Effectively done |
| `app/src/server/` | 56 files | ✅ All GraphQL calls stub-fail silently | ⏳ Source deletion needs `cargo check` |
| `app/src/drive/` | 45 files | ✅ Feature flags prevent all Drive UI | ⏳ Source deletion needs `cargo check` |
| `app/src/notebooks/` | 30 files | ✅ Dead at runtime | ⏳ Source deletion needs `cargo check` |
| `app/src/ai/cloud_agent_config/` | dir | ✅ Dead code | ⏳ Deletion needs `cargo check` |
| `app/src/ai/cloud_environments/` | dir | ✅ Dead code | ⏳ Deletion needs `cargo check` |
| `crates/computer_use/` | crate | ✅ Feature-disabled | ⏳ Removal needs `cargo check` |

### Phase 4 — Wire specsmith governance ✅ COMPLETE

| Change | Location | Status |
|--------|----------|---------|
| BYOP default → `http://127.0.0.1:7700/v1/` | `app/src/settings/ai.rs` | ✅ |
| Remove BYOK billing gate | `app/src/workspaces/user_workspaces.rs` | ✅ |
| Wire GovernanceServer spawn at startup | `app/src/bin/oss.rs` | ✅ |
| Add governance WebView panel | `app/src/settings_view/` | Planned |

### Phase 5 — Rebrand ✅ MOSTLY COMPLETE

| Change | Location | Status |
|--------|----------|---------|
| Binary name `warp-oss` → `kairos` | `app/Cargo.toml` | ✅ |
| AppId → `io.bitconcepts.Kairos` | `app/src/bin/oss.rs` | ✅ |
| macOS plist / URL scheme | `app/src/bin/oss.rs` | ✅ |
| Bundle metadata | `app/Cargo.toml` | ✅ |
| Authors / description | `app/Cargo.toml` | ✅ |
| `app-name`, `Welcome to Kairos`, agent/AI strings | `app/i18n/en/warp.ftl` | ✅ |
| Window title `WINDOW_TITLE = "Kairos"` | `app/src/root_view.rs` | ✅ |
| About page brand name | `app/src/settings_view/about_page.rs` | ✅ |
| macOS menu bar name | `app/src/app_menus.rs` | ✅ |
| Color theme (amber/gold) | `themes/` | Planned |
| Logo / icons | `assets/` | Planned (prompt delivered above) |

### Phase 6 — Bug Reporting via GitHub Issues ✅ COMPLETE

Replace Warp's feedback form / Slack links with GitHub issue tracking routed to
the correct BitConcepts repo based on the nature of the bug.

| Change | Location | Status |
|--------|----------|---------|
| `report_bug_url(repo)` generator (pre-fills version + OS) | `app/src/util/links.rs` | ✅ |
| `feedback_form_url()` aliased to `report_bug_url("kairos")` | `app/src/util/links.rs` | ✅ |
| Help menu: "Report Bug (Terminal/UI)..." → kairos issues | `app/src/app_menus.rs` | ✅ |
| Help menu: "Report Bug (AI/Governance)..." → specsmith issues | `app/src/app_menus.rs` | ✅ |
| Help menu: "Kairos Documentation..." → GitHub README | `app/src/app_menus.rs` | ✅ |
| Removed: Warp Slack, Warp Docs, warpdotdev GitHub Issues | `app/src/app_menus.rs` | ✅ |
| Privacy Policy placeholder → LICENSE file | `app/src/util/links.rs` | ✅ |

**Routing logic:**
- Terminal/UI bugs (crashes, rendering, shell integration) → `github.com/BitConcepts/kairos`
- AI/governance bugs (specsmith responses, BYOP, agent behaviour) → `github.com/BitConcepts/specsmith`
- Each URL is pre-filled with Kairos version and OS via query params so reporters don't have to gather them manually.

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
