---
doc_type: playbook
subsystem: general
status: draft
freshness: stale
preservation: preserve
last_reviewed: 2026-02-09
last_reviewer: Documentation Working Group
owners: Documentation Working Group
review_due: 2026-05-10
version: 2.0.0
---

# CI/CD Playbook

## Authority

`docs/dev/CI/architecture.md` is the canonical CI policy.
If this playbook conflicts with that document, the architecture document wins.

## Purpose

This playbook provides day-to-day operational guidance for CI/CD decisions.
Historical setup content has been intentionally removed to avoid policy drift.

## Current Operating Rules

- Use GitHub-hosted runners for general CI (`fmt`, `clippy`, build, workspace tests, security checks).
- Use the self-hosted Fedora/Nobara runner only for hardware-dependent tests.
- Do not use Xvfb on self-hosted jobs.
- Do not use `apt-get` for self-hosted runner setup.
- Do not force `DISPLAY=:99` on self-hosted jobs.
- Keep self-hosted jobs independent from hosted jobs unless a hard dependency exists.

## STT/Feature Reality Gate

- Current reliable STT path: Moonshine.
- Parakeet is planned work, not current baseline.
- CI examples and jobs should not assume Whisper as an active backend.

## Workflow Hygiene

- Prefer crate-scoped commands for local verification before pushing.
- Keep docs in sync when CI behavior or runner assumptions change.
- Treat docs drift as a CI failure cause, not an afterthought.

## Change Process

When CI policy changes:

1. Update `docs/dev/CI/architecture.md` first.
2. Update this playbook to match.
3. Update related agent docs (`AGENTS.md`, mirrors) if contributor workflow changes.
