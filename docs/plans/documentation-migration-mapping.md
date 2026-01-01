---
doc_type: plan
subsystem: general
version: 1.0.0
status: draft
owners: Documentation Working Group
last_reviewed: 2025-10-19
---

# Documentation Migration Mapping

> **Note (current-state override)**: This plan predates the current agent-instruction policy.
> `AGENTS.md` at the repo root is the canonical agent instruction file.
> Tool-specific entrypoints (`.github/copilot-instructions.md`, `.kilocode/rules/agents.md`) are kept in sync with `AGENTS.md` (and are locally hardlinked where possible).
> **Do not create** `docs/agents.md`.

This plan catalogs existing Markdown files and maps each one to the canonical target location (or disposition) required by the Master Documentation Playbook v1.0.0. The mapping will guide the relocation work in Phase 3 and ensures no markdown remains outside `/docs` except approved exceptions.

## Status Update

Phase 1 and the majority of Phase 2–3 migrations have been executed. Remaining items focus on metadata normalization and README governance updates.

## Legend

- **Target** — Canonical destination path under `/docs`.
- **Action** — `move`, `delete`, `archive`, or `retain-exception`.
- **Notes** — Additional steps (e.g., add retention banner, normalize filename, requires new stub).

## Outside `/docs`

| Current Path | Target | Action | Notes |
| --- | --- | --- | --- |
| crates/coldvox-gui/docs/design.md | docs/domains/gui/design-overview.md | move | Add required frontmatter; integrate with GUI domain; rename to kebab-case. |
| crates/coldvox-gui/docs/implementation-plan.md | docs/plans/gui-implementation-plan.md | move | Ensure plan references docs/todo.md item. |
| crates/coldvox-gui/docs/components.md | docs/domains/gui/components.md | move | Add frontmatter; cross-link with architecture overview. |
| crates/coldvox-gui/docs/tasks.md | docs/tasks/gui-integration.md | move | Link to docs/todo.md. |
| crates/coldvox-gui/docs/architecture.md | docs/domains/gui/architecture.md | move | Frontmatter + link to architecture.md. |
| crates/coldvox-gui/docs/bridge.md | docs/domains/gui/bridge-integration.md | move | Add troubleshooting references if applicable. |
| crates/coldvox-gui/README.md | docs/reference/crates/coldvox-gui.md | move | Replace with thin index linking to crate README. |
| crates/app/docs/updated_architecture_diagram.md | docs/domains/gui/troubleshooting/updated-architecture-diagram.md | move | Confirm appropriate domain; include retention guidance if exploratory. |
| crates/app/test_data/README.md | docs/reference/crates/app-test-data.md | move | Determine if this should stay with crate README or become troubleshooting note. |
| crates/voice-activity-detector/MODIFICATIONS.md | docs/domains/vad/modifications.md | move | Normalize filename + frontmatter. |
| crates/coldvox-telemetry/README.md | docs/reference/crates/coldvox-telemetry.md | move | Thin index. |
| crates/coldvox-text-injection/TESTING.md | docs/domains/text-injection/ti-testing.md | move | Add frontmatter and align with standards. |
| crates/coldvox-text-injection/README.md | docs/reference/crates/coldvox-text-injection.md | move | Thin index. |
| crates/coldvox-stt/README.md | docs/reference/crates/coldvox-stt.md | move | Thin index. |
| crates/coldvox-audio/docs/design-pipewire.md | docs/domains/audio/pipewire-design.md | move | Add frontmatter; ensure domain placement. |
| crates/coldvox-audio/docs/design-user-config.md | docs/domains/audio/user-config-design.md | move | Add frontmatter. |
| crates/coldvox-audio/README.md | docs/reference/crates/coldvox-audio.md | move | Thin index. |
| crates/coldvox-foundation/README.md | docs/reference/crates/coldvox-foundation.md | move | Thin index. |
| .github/copilot-instructions.md | docs/repo/copilot-instructions.md | move | Add frontmatter; document exception removal. |
| .github/prompts/FileInventory.prompt.md | docs/research/logs/file-inventory-prompt.md | move | Add retention banner (ephemeral). |
| .github/SETUP_RELEASE_TOKEN.md | docs/repo/setup-release-token.md | move | Add frontmatter; categorize under repo meta. |
| CHANGELOG.md | CHANGELOG.md | retain-exception | Approved root exception. Add reference to docs/standards.md exceptions. |
| CLAUDE.md | CLAUDE.md | retain-exception | Must reference/import `AGENTS.md` (canonical). |
| README.md | README.md | retain-exception | Update contributing links to new docs. |

## Inside `/docs`

| Current Path | Target | Action | Notes |
| docs/research/comprehensive_gui_plan.md | docs/plans/gui/comprehensive-gui-plan.md | move | Convert folder structure under plans/; add retention banner if exploratory. |
| docs/research/self-hosted-runner-current-status.md | docs/research/logs/2025-??-self-hosted-runner-status.md | move | Normalize filename with date; add retention banner. |
| docs/research/parakeet_technical_solutions.md | docs/research/logs/2025-??-parakeet-technical-solutions.md | move | Normalize to kebab-case; add retention banner. |
| docs/research/aspirational_gui_plan.md | docs/plans/gui/aspirational-gui-plan.md | move | Ensure plan vs research classification. |
| docs/research/raw_gui_plan.md | docs/plans/gui/raw-gui-plan.md | move | Confirm retention policy. |
| docs/research/parakeet_onnx_integration_plan.md | docs/plans/foundation/parakeet-onnx-integration-plan.md | move | Determine domain; likely foundation. |
| docs/research/ParakeetResearch.md | docs/research/logs/2025-??-parakeet-research.md | move | Kebab-case + frontmatter + retention banner. |
| docs/tasks/ci-runner-readiness-proposal.md | docs/tasks/ci-runner-readiness-proposal.md | retain | Add frontmatter, link to todo. |
| docs/self-hosted-runner-complete-setup.md | docs/playbooks/organizational/runner_setup.md | move | Already matches; ensure naming & frontmatter. |
| docs/plans/OpusCodeInject.md | docs/plans/text-injection/opus-code-inject.md | move | Kebab-case + domain subfolder. |
| docs/plans/Testing/OpusTestInject2.md | docs/plans/text-injection/opus-test-inject-2.md | move | Flatten testing folder. |
| docs/plans/Testing/QwenTestMerge.md | docs/plans/text-injection/qwen-test-merge.md | move | Same. |
| docs/plans/OpusTestInject2.md | docs/plans/text-injection/opus-test-inject-2-notes.md | move | De-duplicate with above; evaluate for deletion vs merge. |
| docs/plans/roadmap.md | docs/architecture/roadmap.md | move | Replace old plan file with canonical roadmap. |
| docs/plans/QwenTestMerge.md | docs/plans/text-injection/qwen-test-merge.md | move | Merge with duplicate. |
| docs/checkpointvalidation/ColdVox_2_0_0/* | docs/research/checkpoints/coldvox-2-0-0/* | move | Ensure directory rename to kebab-case; add retention metadata. |
| docs/future-documentation-architecture.md | docs/plans/documentation/future-documentation-architecture.md | move | Link to todo epic. |
| docs/dev/pr-checklist.md | docs/playbooks/organizational/pr_playbook.md | move | Align content; ensure stub matches policy. |
| docs/dev/COMMIT_HISTORY_REWRITE_PLAN.md | docs/research/logs/2025-??-commit-history-rewrite.md | move | Possibly archive; add retention banner. |
| docs/dev/comprehensive-testing-report.md | docs/research/pr-reports/PR-???-comprehensive-testing-report.md | move | Identify PR number; add retention banner. |
| docs/dev/TESTING.md | docs/domains/testing/overview.md | move | Determine if separate domain needed or integrate into standards. |
| docs/dev/clipboard-test-timeout-fixes.md | docs/research/pr-reports/PR-???-clipboard-test-timeout-fixes.md | move | Determine PR and retention. |
| docs/dev/pr152-testing-summary.md | docs/research/pr-reports/PR-152-testing-summary.md | move | Add retention banner. |
| docs/architecture.md | docs/architecture.md | retain | Update to include roadmap link and ADR index. |
| docs/reference/README.md | docs/reference/index.md | move | Evaluate need; convert to index with frontmatter. |
| docs/reference/coldvox_audio.md | docs/domains/audio/index.md | move | Determine if domain index. |
| docs/domains/README.md | docs/domains/index.md | move | Add frontmatter, restructure domain landing page. |
| docs/domains/text_injection.md | docs/domains/text-injection/index.md | move | Normalize naming. |
| docs/domains/streaming_text_injection.md | docs/domains/text-injection/streaming.md | move | rename. |
| docs/domains/text-injection/silero_audio_stream_injection.md | docs/domains/text-injection/silero-audio-stream-injection.md | retain | Add frontmatter. |
| docs/domains/text-injection/architecture-overview.md | docs/domains/text-injection/architecture-overview.md | retain | Add frontmatter. |
| docs/domains/text-injection/research_plan.md | docs/research/logs/2025-??-text-injection-research-plan.md | move | Add retention banner. |
| docs/domains/text-injection/TEXT_INJECTION_DOMAIN_SUMMARY.md | docs/domains/text-injection/summary.md | move | rename. |
| docs/domains/text-injection/quick-reference.md | docs/domains/text-injection/quick-reference.md | retain | Add frontmatter. |
| docs/domains/audio.md | docs/domains/audio/index.md | move | rename + restructure. |
| docs/domains/audio_audio_pipeline.md | docs/domains/audio/pipeline.md | move | rename. |
| docs/domains/audio_telemetry.md | docs/domains/audio/telemetry.md | move | rename. |
| docs/domains/TTS.md | docs/domains/audio/tts.md | move | rename. |
| docs/domains/audio/README.md | docs/domains/audio/index.md | move | consolidate duplicates. |
| docs/domains/audio/devices.md | docs/domains/audio/devices.md | retain | Add frontmatter. |
| docs/domains/telemetry.md | docs/domains/telemetry/index.md | move | rename. |
| docs/domains/telemetry/README.md | docs/domains/telemetry/index.md | consolidate | unify. |
| docs/domains/telemetry/event_bus.md | docs/domains/telemetry/event-bus.md | retain | Add frontmatter. |
| docs/domains/gui.md | docs/domains/gui/index.md | move | rename. |
| docs/domains/gui/README.md | docs/domains/gui/index.md | consolidate | unify. |
| docs/domains/gui/gui_architecture.md | docs/domains/gui/architecture.md | retain | rename to kebab-case. |
| docs/domains/gui/windows_architecture.md | docs/domains/gui/windows-architecture.md | retain | rename. |
| docs/domains/gui/gui-components.md | docs/domains/gui/components.md | consolidate | unify with crate docs. |
| docs/domains/vad.md | docs/domains/vad/index.md | move | rename. |
| docs/domains/stt.md | docs/domains/stt/index.md | move | rename. |
| docs/domains/stt/README.md | docs/domains/stt/index.md | consolidate | unify. |
| docs/domains/stt/whisper/README.md | docs/domains/stt/whisper/index.md | move | rename. |
| docs/domains/stt/whisper/implementation-checklist.md | docs/domains/stt/whisper/implementation-checklist.md | retain | Add frontmatter. |
| docs/domains/stt/whisper/windows-testing.md | docs/domains/stt/whisper/windows-testing.md | retain | Add frontmatter. |
| docs/domains/foundation.md | docs/domains/foundation/index.md | move | rename. |
| docs/domains/foundation/README.md | docs/domains/foundation/index.md | consolidate | unify. |
| docs/domains/foundation/runtime_vision.md | docs/domains/foundation/runtime-vision.md | retain | rename + frontmatter. |
| docs/domains/foundation/core_summary.md | docs/domains/foundation/core-summary.md | retain | rename + frontmatter. |
| docs/domains/foundation/modules.md | docs/domains/foundation/modules.md | retain | add frontmatter. |
| docs/domains/text-injection/detections.md | docs/domains/text-injection/detections.md | retain | add frontmatter. |
| docs/domains/text-injection/tracing.md | docs/domains/text-injection/tracing.md | retain | add frontmatter. |
| docs/domains/text-injection/injection_states.md | docs/domains/text-injection/injection-states.md | retain | rename + frontmatter. |
| docs/domains/text-injection/voice_selection.md | docs/domains/text-injection/voice-selection.md | retain | rename + frontmatter. |
| docs/domains/text-injection/silero_audio_stream_injection.md | docs/domains/text-injection/silero-audio-stream-injection.md | retain | rename + frontmatter. |
| docs/domains/text-injection/docs-review-roadmap.md | docs/domains/text-injection/docs-review-roadmap.md | retain | ensure alignment with architecture roadmap. |
| docs/review/README.md | docs/research/pr-reports/index.md | move | convert to index for review history or archive. |
| docs/review/testing-review.md | docs/research/pr-reports/testing-review.md | move | add retention banner. |
| docs/research/README.md | docs/research/index.md | move | add frontmatter. |
| docs/tasks/README.md | docs/tasks/index.md | move | add frontmatter; link to todo. |
| docs/dev/README.md | docs/playbooks/organizational/index.md | move | restructure. |
| docs/adr/??? | docs/architecture/adr-XXXX.md | move | ensure naming (none currently). |

## Required New Files/Stubs

- docs/standards.md — Document metadata schema, approved exceptions, changelog rubric, watcher spec.
- (Removed) `docs/agents.md` — superseded by root `AGENTS.md`.
- docs/dependencies.md — Dependency overview.
- docs/repo/gitignore.md — Explain .gitignore conventions.
- docs/repo/editor.md — Editor/IDE guidelines.
- docs/todo.md — Canonical task backlog (seed with Documentation migration epic).
- docs/revision_log.csv — File watcher log.
- docs/playbooks/organizational/pr_playbook.md — Organizational PR process stub.
- docs/playbooks/organizational/ci_cd_playbook.md — CI/CD conventions stub.
- docs/playbooks/organizational/runner_setup.md — Move existing content + align.
- docs/playbooks/organizational/github_governance.md — Branch governance stub.
- docs/reference/crates/*.md — Thin indexes for each workspace crate linking to README.
- docs/architecture/roadmap.md — Canonical roadmap destination.
- docs/architecture/adr-index.md — Optional index referencing ADRs (create if absent).

This mapping will be updated as we validate each file during the migration.
