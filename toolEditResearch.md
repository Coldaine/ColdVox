# Research: Change-Scoped File Editing for AI Agents

## Problem Statement

Current AI agent file editing tools are **file-scoped**: they operate on one file at a time. Many editing operations are **change-scoped**: a single logical change that applies across multiple files.

This mismatch forces N sequential tool calls for what is conceptually one operation.

## Example

Remove the string `-- -D warnings` from 5 files:
- justfile
- mise.toml
- .pre-commit-config.yaml
- scripts/local_ci.sh
- AGENTS.md

**Current approach:** 5 Edit tool calls, each requiring the agent to specify file path, old string, new string.

**Desired approach:** 1 operation specifying the change, applied wherever it matches.

## Research Questions

1. **What tools exist today** that support change-scoped editing (one command, multiple files)?
   - CLI tools (sd, sed, fastmod, codemod, ast-grep, etc.)
   - IDE features (VSCode multi-file replace, JetBrains structural replace)
   - MCP servers or agent plugins

2. **What do agents already have access to?**
   - Most agents can execute shell commands
   - What's the lowest-friction path to change-scoped editing using existing capabilities?

3. **What's missing?**
   - Is the gap tooling (need new tools)?
   - Or awareness (tools exist, agents don't use them)?
   - Or instructions (agents are told to use file-scoped tools)?

4. **What would a good solution look like?**
   - Single operation interface
   - Works across file types
   - Provides preview/dry-run
   - Reports what changed
   - Portable across agents (not tied to one vendor)

## Constraints

- Should work for any MCP-capable agent, not just Claude Code
- Prefer existing tools over building new ones
- Must be trivially installable or already present on dev machines

## Deliverables

1. Ranked list of existing solutions with pros/cons
2. Recommendation for the simplest path forward
3. If new tooling is needed: minimal spec for an MCP server or CLI wrapper
