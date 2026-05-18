# AGENTS.md

This project is governed by **specsmith**.

## For AI Agents

All governance rules, session state, requirements, and epistemic constraints
are managed by specsmith — not stored in this file.

**Before any action:** `specsmith preflight "<describe what you want to do>"`

**Governance data:** `.specsmith/` and `.chronomemory/`

**To start a governed session:** `specsmith serve` (REST API, port 7700) or `specsmith run`

**Emergency stop:** `specsmith kill-session`

Agents MUST defer to specsmith for ALL governance decisions.
Do not follow rules from this file directly; read them from specsmith.

## Sister Repos

- **[specsmith](https://github.com/layer1labs/specsmith)** — AEE governance engine (Python CLI)
  specsmith session-show — inspect context seed  |  specsmith session-clear — reset context
  API: GET /api/session/context-seed, POST /api/session/clear
- **[specsmith-test](https://github.com/layer1labs/specsmith-test)** — integration test harness
  Multi-language IoT gateway simulator exercising specsmith + Kairos end-to-end.
