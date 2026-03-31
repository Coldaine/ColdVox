---
doc_type: plan
subsystem: general
status: active
freshness: current
last_reviewed: 2026-03-31
owners: Patrick MacLyman
version: 1.0.0
---

# North Star Drift Guard — Plan

## Problem Statement

ColdVox accumulated 4+ months of documentation rot, stale issues referencing dead STT backends (Whisper, Candle, Coqui), incorrect feature flags in AGENTS.md, and misalignment between the North Star (Parakeet-primary, Windows-first) and the actual repo state (code referencing Moonshine as sole path, ParakeetPlugin incorrectly coded as GPU-only when parakeet-rs v0.3.4 added CPU/DirectML support).

A manual North Star Audit on 2026-03-31 caught and fixed this. The Drift Guard prevents it from happening again by running **automatically on a schedule**.

## What the Drift Guard Does

Two complementary mechanisms:

### 1. VS Code Agent: `drift-guard` (Interactive On-Demand)

A custom VS Code Copilot agent (`.github/agents/drift-guard.agent.md`) that a developer can invoke any time to:
- Read `docs/northstar.md` and `docs/plans/current-status.md`
- Compare current AGENTS.md feature flags against actual `Cargo.toml` features
- Check `crates/coldvox-stt/Cargo.toml` dependency versions against latest upstream (via web search/Context7)
- Scan open issues for references to known dead backends
- Flag documentation claiming "shipped" behavior that doesn't match code
- Produce a drift report

### 2. Jules GitHub Action: Weekly Drift Audit (Automated)

A GitHub Actions workflow (`.github/workflows/drift-guard.yml`) using Google Jules via `google-labs-code/jules-invoke@v1` that runs weekly to:
- Check upstream dependency versions (parakeet-rs, pyo3, tauri)
- Compare against current Cargo.toml pinned versions
- Scan for dead backend references in docs and issues
- Report findings as a GitHub issue or PR

## Architecture

```
┌─────────────────────────────────────┐
│ VS Code Agent (on-demand)           │
│ .github/agents/drift-guard.agent.md │
│                                     │
│ Tools: read, search, web, fetch     │
│ MCP: context7, exa, github          │
│                                     │
│ Reads: northstar.md, Cargo.toml,    │
│        current-status.md, issues    │
│ Output: Interactive drift report    │
└─────────────────────────────────────┘

┌─────────────────────────────────────┐
│ Jules Action (weekly cron)          │
│ .github/workflows/drift-guard.yml  │
│                                     │
│ Trigger: schedule (weekly Monday)   │
│          + manual workflow_dispatch  │
│                                     │
│ Reads: Cargo.toml, docs/northstar,  │
│        upstream crate versions      │
│ Output: GitHub issue with findings  │
└─────────────────────────────────────┘
```

## VS Code Agent Details

**File:** `.github/agents/drift-guard.agent.md`

The agent has access to:
- `read`, `search`, `web`, `fetch` tools (read-only + web research)
- Context7 MCP for library documentation lookups
- Exa MCP for code-aware web search
- GitHub MCP for issue/PR scanning

**Invocation:** Select "drift-guard" from the agents dropdown in VS Code Chat, or type `@drift-guard` in Agent mode.

**Key behaviors:**
1. Reads North Star goals and current status
2. Compares documented feature flags against `crates/*/Cargo.toml`
3. Checks upstream crate versions via web search
4. Scans for dead-backend references (whisper, candle, coqui, leopard, silero-stt, vosk, faster-whisper)
5. Validates architecture.md TieredSTTSystem matches northstar.md fallback chain
6. Reports drift with specific file:line references

## Jules GitHub Action Details

**File:** `.github/workflows/drift-guard.yml`

**Action:** `google-labs-code/jules-action@v1` (formerly `jules-invoke`, renamed).

**Trigger:** Weekly Monday 6 AM UTC + manual dispatch.

**Output mechanism:** Jules creates **Pull Requests**, not issues. The audit
results are written to `docs/drift-reports/latest.md` and delivered as a PR.

**Requirements:**
- `JULES_API_KEY` repository secret (from [jules.google.com/settings#api](https://jules.google.com/settings#api))
- Jules GitHub App installed on the Coldaine/ColdVox repository (done via the Jules web app)

**Jules prompt:** Instructs Jules to:
1. Read `docs/northstar.md` for canonical goals
2. Check `crates/coldvox-stt/Cargo.toml` for `parakeet-rs` version; compare against latest on crates.io using explicit `curl` commands
3. Check `crates/coldvox-gui/src-tauri/Cargo.toml` for Tauri version; same `curl` approach
4. Scan all `.md` files in `docs/` for references to dead backends
5. Scan `AGENTS.md`, `CLAUDE.md`, `GEMINI.md` for dead backend references
6. If drift found: create/update `docs/drift-reports/latest.md` (Jules auto-creates a PR)
7. If no drift: write "all clear" to the report file

**Security:**
- Jules API key stored as GitHub secret
- Jules only has read access + PR creation via its own GitHub App
- All Jules PRs require human review before merge

## Context7 MCP Integration

Context7 provides up-to-date library documentation. The drift-guard agent uses it to:
- Look up `parakeet-rs` API changes and migration notes
- Look up `tauri` migration guides between versions
- Look up `pyo3` compatibility notes

**Note:** Context7 is best for documentation lookups, not exact version numbers. For precise version checks, the agent should prefer `web/fetch` against crates.io.

**Setup:** Context7 MCP must be configured. If `.vscode/mcp.json` does not exist, create it with the Context7 server configuration. See the VS Code MCP documentation for format.

## Setup Steps

### Step 1: Create VS Code Agent
Create `.github/agents/drift-guard.agent.md` (done in this branch).

### Step 2: Install Jules GitHub App & Get API Key
1. Go to https://jules.google.com/
2. Authenticate with GitHub (Coldaine account)
3. In the Jules web app, ensure the Jules GitHub App is installed on the `Coldaine/ColdVox` repository
4. Go to [jules.google.com/settings#api](https://jules.google.com/settings#api) to generate an API key
5. Add as repository secret: Settings → Secrets → `JULES_API_KEY`

### Step 3: Create Jules Workflow
Create `.github/workflows/drift-guard.yml` (done in this branch).

### Step 4: Test
1. Run the VS Code agent manually: `@drift-guard Run a drift audit`
2. Trigger the Jules workflow manually via workflow_dispatch
3. Verify both produce useful output
4. Adjust prompts as needed

## Dead Backend Kill List

The following terms indicate dead/stale references that should be flagged:
- `whisper` (as STT backend — not the general word)
- `whisper_plugin.rs`
- `faster-whisper`
- `candle` (as STT backend)
- `coqui`
- `leopard`
- `silero-stt` (distinct from `silero` VAD which is alive)
- `vosk`

## Relationship to North Star Audit

Yes — this is the automated version of what I proposed as the "North Star Audit" in the previous conversation. The Guardian concept (per-PR reactive gate) is a separate concern that could be added later. The Drift Guard is the **proactive periodic sweep** that catches accumulated state drift.

## Success Criteria

1. Selecting `drift-guard` from the VS Code agents dropdown produces a correct drift report within 2 minutes
2. Jules weekly workflow runs without failure and produces a PR with `docs/drift-reports/latest.md` when drift is detected
3. The parakeet-rs version lag that went unnoticed for months would be caught within 1 week
4. Dead backend references in new docs would be flagged within 1 week
