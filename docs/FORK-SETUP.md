# Kairos Fork Setup

**How to set up the real Kairos terminal repository.**

The current `BitConcepts/kairos` repo is a governance module stub used during
architecture and requirements phases. The real Kairos terminal is a fork of
`zerx-lab/warp` (OpenWarp) with governance wired in and all Warp cloud services
removed.

---

## Step 1 — Fork OpenWarp into the Kairos repo

Two options:

**Option A (recommended): Replace the current kairos repo**
1. Archive `BitConcepts/kairos` on GitHub (Settings → Archive)
2. Go to https://github.com/zerx-lab/warp
3. Click **Fork** → set owner to `BitConcepts`, name to `kairos`
4. Clone the fork locally:
   ```bash
   git clone https://github.com/BitConcepts/kairos D:\Development\BitConcepts\kairos
   ```
5. Add specsmith remote for governance module reference:
   ```bash
   git remote add specsmith https://github.com/BitConcepts/specsmith
   ```

**Option B: Separate terminal repo**
- Keep `BitConcepts/kairos` as the governance library (current stub)
- Create `BitConcepts/kairos-terminal` as the fork of zerx-lab/warp
- Reference `kairos` as a Cargo dependency in `kairos-terminal`

Option A is simpler and reflects the intent: Kairos IS the terminal.

---

## Step 2 — Update the license

Change `README.md` header to note the AGPL license inheritance, and verify
`LICENSE-AGPL` is present (it will be, inherited from OpenWarp).

The key framing:
- Kairos (terminal) = **AGPL-3.0** (open source, community terminal)
- specsmith (governance backend) = **MIT / commercial** (the product)

---

## Step 3 — Copy governance module into the fork

Copy the governance integration code from the current stub into the Warp fork:

```bash
# In the kairos-terminal (Warp fork) root:
cp -r <kairos-stub>/src/governance ./src/governance_specsmith
cp <kairos-stub>/src/session.rs ./src/session.rs
```

Wire `GovernanceServer::spawn()` into the Warp startup sequence:
- Find Warp's `app/src/main.rs` or equivalent entry point
- Add `governance_specsmith::server::GovernanceServer::spawn(...)` before the
  event loop
- This replaces the Warp cloud agent initialization

---

## Step 4 — Set the BYOP default endpoint

In the OpenWarp fork, find the BYOP provider configuration (likely in
`app/src/ai/llms.rs` or `~/.config/openwarp.toml`):

Change the default `base_url` to point at specsmith:
```toml
[provider.kairos-governance]
name = "Kairos Governance (specsmith)"
base_url = "http://127.0.0.1:7700"
model = "kairos"
api_key = ""   # no key needed for local governance-serve
```

This makes `specsmith governance-serve` the BYOP endpoint out of the box.
Users who want a real AI model behind the governance gate set `KAIROS_AI_BASE_URL`.

---

## Step 5 — Remove Warp cloud services

Work through this removal checklist. Reference commit-by-commit so each
removal is traceable in the ledger:

### Phase 1: Account / Login (I6)
- [ ] Remove `app/src/auth/` or equivalent login flow
- [ ] Remove Warp account creation/login UI screens
- [ ] Remove workspace billing tier checks (e.g. `is_byo_api_key_enabled()`)
- [ ] Remove BYOK paywall — all users get BYOP by default
- [ ] Remove `app/src/workspaces/user_workspaces.rs` billing metadata checks

### Phase 2: Telemetry (I6)
- [ ] Remove `app/src/telemetry/` or equivalent tracking
- [ ] Remove Segment / analytics SDK dependencies
- [ ] Remove `analytics_event!` / `track!` macro calls throughout codebase
- [ ] Verify no outbound HTTP calls remain except to `127.0.0.1`

### Phase 3: Warp Drive (I6)
- [ ] Remove `app/src/drive/` or equivalent cloud sync
- [ ] Remove Drive UI panels and menu items
- [ ] Remove Drive sync triggers from file save hooks

### Phase 4: Warp AI cloud (I6, I7)
- [ ] Remove cloud agent orchestration (the non-BYOP path)
- [ ] Remove OpenAI-specific API key handling from settings UI
- [ ] Ensure all AI traffic routes through BYOP → `127.0.0.1:7700`
- [ ] Remove "Powered by OpenAI" references

### Phase 5: Branding
- [ ] Replace all "Warp" text with "Kairos" in UI strings
- [ ] Replace Warp logo/icon with Kairos brand assets
- [ ] Update color theme (amber/gold, NOT Warp blue/purple)
- [ ] Update app name in platform manifests (macOS .app, Windows installer)
- [ ] Update `Cargo.toml` package name to `kairos`

---

## Step 6 — Add Kairos governance WebView panel (REQ-005)

In the Warp fork's settings WebView system, add a Kairos governance panel:

- **Route**: `/governance` (accessible from Kairos menu)
- **Content**: Fetches from `http://127.0.0.1:7700/health` + governance state
- **Shows**: Current AEE phase, confidence score, open work items, recent preflight decisions
- **Playwright-testable**: Warp's WebView panels support Playwright (REQ-005)

Implementation uses Warp's existing `SettingsView` / WebView infrastructure.

---

## Step 7 — Verify governance integration

After the fork is set up and cloud services removed:

```bash
# Build the terminal
./script/bootstrap
cargo build --release

# In Terminal 1: start governance server
KAIROS_AI_BASE_URL=http://localhost:11434 \
specsmith governance-serve --port 7700 --project-dir .

# In Terminal 2: launch Kairos
./target/release/kairos
```

Expected flow:
1. Kairos starts, finds specsmith at `127.0.0.1:7700`
2. User types a request in Kairos terminal
3. Kairos sends `POST /v1/chat/completions` to specsmith (its BYOP endpoint)
4. specsmith runs preflight → if accepted, forwards to Ollama/real AI
5. Response returned to Kairos terminal

---

## Environment Variables

| Variable | Description |
|----------|-------------|
| `KAIROS_AI_BASE_URL` | Real AI provider base URL (e.g. `http://localhost:11434` for Ollama) |
| `KAIROS_AI_API_KEY` | Real AI provider API key (leave empty for local) |
| `KAIROS_AI_MODEL` | Real AI model name (e.g. `qwen2.5:14b`) |
| `SPECSMITH_CMD` | Override specsmith command detection |

---

## What's Already Done (Current Stub)

The `src/governance/` module (GovernanceClient, GovernanceServer) and
`src/session.rs` (SessionConfig, find_specsmith_cmd) are fully implemented and
tested. They just need to be transplanted into the Warp fork's source tree.

The `specsmith governance-serve` endpoint already has:
- `GET  /health` — liveness probe
- `POST /preflight` — governance gate
- `POST /verify` — post-change verification
- `POST /v1/chat/completions` — **Kairos BYOP gateway** (intercept → gate → forward → verify)
