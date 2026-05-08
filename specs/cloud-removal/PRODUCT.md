# Cloud Removal — Kairos Clean Base

## Summary

Strip all Warp cloud/account/telemetry/billing infrastructure from Kairos to
produce a self-contained terminal that makes zero network calls to Warp servers.
The specsmith governance layer replaces the cloud AI.

## Behavior

### After removal the user sees

1. Terminal launches without a login screen. No account required.
2. No BYOE/BYOE paywall. AI provider is freely configurable.
3. BYOE default endpoint is `http://127.0.0.1:7700` (specsmith governance-serve).
4. No Warp Drive panel, no cloud sync, no shared notebooks.
5. No telemetry, analytics, crash uploads, or server-driven experiment flags.
6. No pricing pages, upgrade CTAs, or billing tier checks anywhere.
7. All remaining functionality works fully offline and locally.

### What is kept unchanged

- Terminal core: shell, blocks, input, completions, themes, settings
- BYOE infrastructure (openai_compatible.rs, genai adapter) — rewired to specsmith
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
| `warp_features/src/lib.rs` | flags | 30+ cloud flags force-false, SoloUserBYOE force-true | ✅ |
| `app/Cargo.toml` | defaults | Cloud feature flags removed from default build | ✅ |

**Remaining for full Phase 2 cleanup (Phase 3):** delete dead cloud module code.

### Phase 3 — Remove cloud-dependent features ✅ COMPLETE

Cloud code is **dead at runtime AND at the module level**. All cloud operations
have been stubbed: get_all/get_by_id return empty, send_create/update return Err,
new_from_server returns None. 18,633 lines of dead cloud test code gutted.

| Module | Size | Runtime | Cloud Ops | Tests |
|--------|------|---------|-----------|-------|
| `app/src/crash_reporting/` | 4 files | ✅ Feature-gated out | ✅ N/A | ✅ N/A |
| `crates/graphql/src/client.rs` | 1 file | ✅ Stubbed (Phase 2) | ✅ Returns ServiceUnavailable | ✅ N/A |
| `app/src/pricing/` | 1 file | ✅ No-op stub | ✅ N/A | ✅ N/A |
| `app/src/linear.rs` | 1 file | ✅ URL parsing only | ✅ N/A | ✅ N/A |
| `app/src/tips/` | 3 files | ✅ Pure UI data | ✅ N/A | ✅ N/A |
| `app/src/experiments/` | 7 files | ✅ Reads stub cache | ✅ N/A | ✅ N/A |
| `app/src/resource_center/` | 10 files | ✅ Pure local UI | ✅ N/A | ✅ N/A |
| `app/src/server/` | 56 files | ✅ GraphQL stub-fails | ✅ All ops dead | ✅ 14 test files gutted |
| `app/src/drive/` | 45 files | ✅ Feature flags off | ✅ send_create/update→Err | ✅ 6 test files gutted |
| `app/src/notebooks/` | 30 files | ✅ Dead at runtime | ✅ send_create/update→Err | ✅ 8 test files gutted |
| `app/src/ai/cloud_agent_config/` | 1 file | ✅ Dead | ✅ get_all→[], get_by_id→None | ✅ N/A |
| `app/src/ai/cloud_environments/` | 2 files | ✅ Dead | ✅ get_all→[], owner→None | ✅ 1 test file gutted |
| `crates/computer_use/` | crate | ✅ Feature-disabled | ✅ N/A | ✅ N/A |

**Note on source retention:** The implementation files in server/ (56), drive/ (45),
and notebooks/ (30) retain their source because they export type definitions used by
170+ other files via deeply nested import graphs. The types are compile-time
dependencies only — no cloud operations execute at runtime. The `#![allow(dead_code)]`
directive in `lib.rs` suppresses warnings for this retained-but-dead code.

### Phase 4 — Wire specsmith governance ✅ COMPLETE

| Change | Location | Status |
|--------|----------|---------|
| BYOE default → `http://127.0.0.1:7700/v1/` | `app/src/settings/ai.rs` | ✅ |
| Remove BYOE billing gate | `app/src/workspaces/user_workspaces.rs` | ✅ |
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
|| `app-name`, `Welcome to Kairos`, agent/AI strings | `app/i18n/en/kairos.ftl` | ✅ |
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
- AI/governance bugs (specsmith responses, BYOE, agent behaviour) → `github.com/BitConcepts/specsmith`
- Each URL is pre-filled with Kairos version and OS via query params so reporters don't have to gather them manually.

---

## Removal Rules

1. **Stub before delete** — never delete a module with widespread imports until
   everything that depends on it compiles against a stub.
2. **One phase per commit** — each phase produces a buildable commit.
3. **Test after each phase** — `cargo check -p app` must pass.

---

## Success Criteria

- [x] `cargo check -p kairos --bin kairos` passes (verified 2026-05-07)
- [x] `grep -r "warp.dev" app/src/` (non-test files) returns empty
- [x] Terminal launches without login (`skip_login` in default features)
- [x] BYOE default is `http://127.0.0.1:7700/v1/` (OpenAI + OpenAIResp protocols)
- [x] `specsmith governance-serve` spawns at start via `GovernanceServer::spawn()`
- [x] Zero runtime calls to Warp servers (GraphQL stubbed, all cloud flags off)

## Phase 3 Source Deletion — Final Status (2026-05-08)

All cloud operations are dead at both runtime and module level:
- **GraphQL**: `send_graphql_request` returns `ServiceUnavailable` (Phase 2)
- **cloud_agent_config**: `get_all()→[]`, `get_by_id()→None`
- **cloud_environments**: `get_all()→[]`, `get_by_id()→None`, `owner_for_new_*()→None`
- **notebooks**: `send_create_request()→Err`, `send_update_request()→Err`, `new_from_server_update()→None`
- **drive/folders**: `send_create_request()→Err`, `send_update_request()→Err`, `new_from_server_update()→None`
- **computer_use**: Feature-gated off in default features
- **30 test files gutted**: 18,633 lines of dead cloud test code removed
- `cargo check -p kairos --bin kairos` passes with 0 errors

Type definitions are retained in implementation files because they are
imported by 170+ other files. Physical file deletion would require extracting
type shells into minimal stubs and updating every importer — a mechanical
refactor with zero runtime impact. The codebase uses `#![allow(dead_code)]`
to suppress warnings for this retained code.
