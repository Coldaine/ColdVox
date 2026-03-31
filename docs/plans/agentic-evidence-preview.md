# Specification: Agentic Evidence Assessor (Preview Mode)

## Objective
Stand up a "shadow mode" implementation of the Portable Agentic Evidence Standard for ColdVox. This system will analyze Pull Requests, evaluate evidence against claims, and detect semantic drift. 

It will **not** block merges or force the developer to change their workflow. It will compile its findings and post them to the GitHub Actions Run Summary.

## Core Philosophy: Native Agentic Execution
This architecture relies on **Native Agentic Execution**. We install the `gemini-cli` directly into the GitHub Actions runner, grant it full file access, and give it a comprehensive system prompt. The agent uses its own native tools (file reading, grepping, shell commands) to investigate the repository.

To avoid diluting the complex reasoning required, the agent's instructions are stored in a dedicated, comprehensive prompt file rather than a fragile inline string.

---

## Architecture Overview

### 1. The Prompt (`.github/prompts/evidence-assessor.md`)
We will create a robust, multi-page instruction set for the agent that actually codifies the "Portable Agentic Evidence Standard". It will instruct the agent exactly how to identify claims, what counts as valid artifact evidence, and how to execute a semantic drift check against the repository's authoritative documentation.

### 2. The Workflow (`.github/workflows/agentic-evidence-preview.yml`)
The workflow installs Node.js, fetches the PR details, and executes the Gemini CLI in "YOLO" mode, feeding it the comprehensive prompt.

```yaml
name: Agentic Evidence Assessor (Shadow Mode)

on:
  pull_request:
    types: [opened, synchronize, ready_for_review]

jobs:
  assess:
    runs-on: ubuntu-latest
    permissions:
      contents: read
      pull-requests: read
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0 # Need full history for git diff
          
      - name: Setup Node
        uses: actions/setup-node@v4
        with:
          node-version: '20'
          
      - name: Fetch PR Body
        id: pr_body
        env:
          GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        run: |
          gh pr view ${{ github.event.pull_request.number }} --json body --jq .body > pr_body.txt
          
      - name: Run Evidence Assessor
        env:
          GEMINI_API_KEY: ${{ secrets.GEMINI_API_KEY }}
          BASE_REF: origin/${{ github.base_ref }}
        run: |
          npx --yes @google/gemini-cli "
            Your objective is defined in '.github/prompts/evidence-assessor.md'. 
            You are operating autonomously. Read the prompt, execute your investigation using your available tools, and write your final assessment to the \$GITHUB_STEP_SUMMARY path.
          " -y
```

### 3. The Output Destination (GitHub Step Summary)
The agent writes its final synthesis to the `$GITHUB_STEP_SUMMARY` environment variable. GitHub natively renders this on the Actions workflow page. This provides a frictionless "Message Queue" that is easy to check but doesn't clutter the main PR comment thread.

---

## Implementation Plan

1. **Draft the Comprehensive Prompt:** Create `.github/prompts/evidence-assessor.md` containing the full behavioral instructions based on the Evidence Standard.
2. **Create the Workflow File:** Commit `.github/workflows/agentic-evidence-preview.yml`.
3. **Deploy:** Commit to `main` and let it run out-of-band on the next PR.