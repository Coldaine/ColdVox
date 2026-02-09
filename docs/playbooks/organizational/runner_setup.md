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

# Self-Hosted Runner Setup

## Authority

`docs/dev/CI/architecture.md` is the canonical source for CI runner policy.
This document contains only current setup and operational expectations.

## Runner Profile

- OS: Fedora/Nobara Linux
- Session: live KDE Plasma desktop session
- Role: hardware-dependent CI jobs only
- Labels: `[self-hosted, Linux, X64, Fedora, Nobara]`

## Required Environment

- Real desktop display available (`DISPLAY=:0` expected in active session).
- Wayland session variables available when running Wayland tests.
- Tooling installed via Fedora package management (`dnf`), not `apt-get`.
- Rust toolchain, `cargo`, and CI dependencies installed and available to runner user.

## Explicit Non-Goals

- No Xvfb-based runner operation.
- No `DISPLAY=:99` workflow assumptions.
- No Ubuntu-specific package management instructions.

## Health Checks

- Runner service is active and online in GitHub Actions.
- Desktop session is alive before hardware jobs run.
- Input/output dependencies for hardware tests are available (display/audio/clipboard paths).

## Job Routing Rule

- GitHub-hosted: lint/build/general tests/security checks.
- Self-hosted: only jobs that require real display/audio/clipboard/hardware.

## Maintenance

- Keep runner labels and this document aligned with `docs/dev/CI/architecture.md`.
- If hardware capabilities change, update architecture doc first, then this file.
