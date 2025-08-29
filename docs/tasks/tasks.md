# Automated Text Injection Testing – Task Plan

This plan turns our strategy into concrete, trackable tasks to deliver robust, non-dummy E2E tests for text injection across X11 and Wayland. It’s organized by phases with acceptance criteria, file touchpoints, and CI wiring. Default target is hosted CI (X11) plus optional self-hosted Wayland jobs.

## Goals and scope

- Validate real keystroke delivery into a live window/buffer (no stubs).
- Cover X11 (hosted CI) and selected Wayland paths (self-hosted).
- Keep flakes low via focus checks, readiness gates, and bounded timing.
- Provide clear skip/gating for environments lacking permissions (uinput/portals).

Out of scope (for now): cross-distro packaging, macOS/Windows GUI automation.

## Success criteria

- X11 tests pass reliably (>=99% over 20 runs) on ubuntu-latest.
- Wayland wlroots headless tests pass on a self-hosted runner labelled `sway`.
- Optional KDE portal tests pass on a self-hosted runner labelled `plasma` with pre-seeded portal consent.
- CI matrix auto-skips non-available environments without failing the workflow.

---

## Phase 0 – Spike & toggles

- [ ] Create feature flags to gate GUI E2E tests:
  - [ ] Cargo features: `gui-x11`, `gui-wayland-wlroots`, `gui-wayland-kde-portal`.
  - [ ] Tests use `#[cfg(feature = "gui-x11")]` etc.
- [ ] Add runtime env toggles to skip tests gracefully:
  - [ ] `COLDVOX_GUI_E2E=1` to enable; otherwise skip.
  - [ ] Detect prerequisites (binaries, DISPLAY/WAYLAND_DISPLAY) at test start and `cargo test`-style skip with clear message.

Touchpoints:
- `crates/app/Cargo.toml` (features)
- `crates/app/tests/` (helpers and guards)

Acceptance:
- Running `cargo test` locally without any setup skips GUI tests with a helpful message; enabling flags runs them when prerequisites present.

---

## Phase 1 – Minimal test target app

- [ ] Add a tiny GTK text target app that prints buffer updates to stdout.
  - [ ] Crate path: `test-apps/gtk_text_target/` (binary).
  - [ ] Title: `ColdVox Test Target`.
  - [ ] Single `Entry` that prints `RECEIVED_TEXT:<text>` on change.
- [ ] Provide a minimal CLI opt to auto-close after inactivity (e.g., `--exit-after-ms 10000`).

Touchpoints:
- `test-apps/gtk_text_target/Cargo.toml`
- `test-apps/gtk_text_target/src/main.rs`

Acceptance:
- `cargo run -p gtk_text_target` launches a window; typing echoes to stdout.

---

## Phase 2 – X11 E2E (local + hosted CI)

Tasks:
- [ ] Write X11 integration test `tests/x11_integration.rs` (feature `gui-x11`).
  - [ ] Launch `gtk_text_target`.
  - [ ] Resolve X11 window id by name/class with `xdotool search --sync` (take first id).
  - [ ] `windowactivate --sync` and assert `getwindowfocus` equals target id.
  - [ ] Type `hello world` with `xdotool type --delay 50`.
  - [ ] Read child stdout and assert `RECEIVED_TEXT:hello world`.
  - [ ] Optional: set `setxkbmap us` inside the Xvfb session for deterministic keymap.
- [ ] Add helpers to spawn commands and capture stdout with timeout/retries.

Packages (CI): `xvfb xdotool openbox xprop xwininfo wmctrl imagemagick at-spi2-core` (OCR optional).

Touchpoints:
- `crates/app/tests/x11_integration.rs`
- `crates/app/tests/util/mod.rs` (process helpers)

Acceptance:
- Test passes under Xvfb locally; fails fast with skip if prerequisites missing.

---

## Phase 3 – CI job for X11 (ubuntu-latest)

Tasks:
- [ ] Add workflow `.github/workflows/gui.yml` with X11 job:
  - [ ] Install deps; start `Xvfb :99` and `openbox --sm-disable`.
  - [ ] `cargo test -p coldvox-app --features gui-x11 -- --include-ignored` (or similar).
  - [ ] Collect artifacts on failure (logs/screenshots optional).
- [ ] Add matrix slot `env: x11` and future-proof for Wayland jobs.

Acceptance:
- Workflow green on PRs, stable over retries.

---

## Phase 4 – Wayland (wlroots headless via sway + wtype) – self-hosted

Tasks:
- [ ] Provide a minimal sway config: `tests/fixtures/sway-minimal.conf`.
- [ ] Write `tests/wayland_wlroots.rs` (feature `gui-wayland-wlroots`).
  - [ ] Start sway in headless mode (`WLR_BACKENDS=headless`), export `WAYLAND_DISPLAY` for the test scope.
  - [ ] Launch `gtk_text_target` in same env.
  - [ ] Use `wtype` to inject `hello world` and `Return`.
  - [ ] Assert stdout contains `RECEIVED_TEXT:hello world`.
- [ ] Add self-hosted CI job `runs-on: [self-hosted, linux, sway]`.

Packages: `sway wtype` and necessary Wayland/GTK runtime libs.

Acceptance:
- Job passes on a prepared self-hosted runner; hosted CI auto-skips.

---

## Phase 5 – Wayland (KDE/KWin via xdg-desktop-portal) – self-hosted

Notes: Requires interactive consent or pre-seeded trust on `xdg-desktop-portal-kde`. Only feasible on persistent self-hosted runner.

Tasks:
- [ ] Document runner prep: enable portal services and pre-grant RemoteDesktop/Input permissions for the test user.
- [ ] Implement a tiny helper (Rust or script) to request portal session and send keys, or integrate an existing tool if available.
- [ ] Write `tests/wayland_kde_portal.rs` (feature `gui-wayland-kde-portal`).
  - [ ] Launch `gtk_text_target`.
  - [ ] Start portal session; send `hello world`.
  - [ ] Assert stdout.
- [ ] Add CI job `runs-on: [self-hosted, linux, plasma]` and gate.

Acceptance:
- Test passes consistently on the Plasma self-hosted runner; skips elsewhere.

---

## Phase 6 – ydotool fallback (uinput) – self-hosted/privileged only

Tasks:
- [ ] Document uinput setup (udev rule, group membership, `ydotoold` service).
- [ ] Write `tests/wayland_ydotool.rs` (feature `gui-wayland-ydotool`).
  - [ ] Ensure `/dev/uinput` accessible; start `ydotoold`.
  - [ ] Inject text with `ydotool type` and assert stdout.
- [ ] Add CI job `runs-on: [self-hosted, linux, privileged]`.

Acceptance:
- Passes on privileged runner; auto-skipped otherwise.

---

## Phase 7 – Flake reduction & verification extras

- [ ] Add readiness waits: poll window mapped/visible, verify focus equality.
- [ ] Add small jitter sleeps (50–150ms) after focus changes.
- [ ] Optional AT‑SPI verification utility (Python or Rust via atk): only when bus present.
- [ ] Optional screenshot+OCR fallback (documented as off-by-default).
- [ ] Keymap stabilization: call `setxkbmap us` within Xvfb job.

Acceptance:
- X11 test variance < 1% across 20 CI retries.

---

## Phase 8 – Documentation & developer UX

- [ ] Add `docs/end_to_end_testing.md` updates for GUI tests.
- [ ] Add `README` snippets to run locally:
  - [ ] X11 with Xvfb instructions.
  - [ ] Wayland wlroots self-hosted notes.
  - [ ] Flags and env toggles.
- [ ] Add `Makefile`/cargo alias convenience targets (optional).

Acceptance:
- New dev can run X11 E2E locally in <5 min following docs.

---

## CI wiring (summary)

Workflow: `.github/workflows/gui.yml`

- Job: `x11-tests` (hosted)
  - `DISPLAY=:99` -> start Xvfb + openbox
  - Install toolchain + deps
  - Run: `cargo test -p coldvox-app --features gui-x11 -- --nocapture`
- Job: `wayland-wlroots` (self-hosted label `sway`)
  - Start sway headless; export `WAYLAND_DISPLAY`
  - Run: `cargo test -p coldvox-app --features gui-wayland-wlroots -- --nocapture`
- Job: `wayland-kde-portal` (self-hosted label `plasma`)
  - Ensure portal services/consent
  - Run: `cargo test -p coldvox-app --features gui-wayland-kde-portal -- --nocapture`
- Job: `wayland-ydotool` (self-hosted `privileged`)
  - Ensure `/dev/uinput` access and `ydotoold`
  - Run: `cargo test -p coldvox-app --features gui-wayland-ydotool -- --nocapture`

Gating & skip rules:
- Tests check for required binaries (xdotool, wtype, ydotool) and env (DISPLAY/WAYLAND_DISPLAY), otherwise `cargo test` skip.

---

## File creation/edit map

- New:
  - `test-apps/gtk_text_target/Cargo.toml`
  - `test-apps/gtk_text_target/src/main.rs`
  - `crates/app/tests/x11_integration.rs`
  - `crates/app/tests/wayland_wlroots.rs`
  - `crates/app/tests/wayland_kde_portal.rs` (optional)
  - `crates/app/tests/wayland_ydotool.rs` (optional)
  - `crates/app/tests/util/mod.rs`
  - `tests/fixtures/sway-minimal.conf`
  - `.github/workflows/gui.yml`
- Edit:
  - `crates/app/Cargo.toml` (features)
  - `docs/end_to_end_testing.md` (add GUI section)

---

## Risks & mitigations

- Hosted CI permissions: `/dev/uinput` and portals not available → keep Wayland to self-hosted; auto-skip.
- Focus & timing: assert focus equals target, add small jitter, use `--sync` modes.
- Keymaps: force `us` layout in Xvfb job when needed.
- Portal consent: pre-seed on persistent runner; document steps; otherwise skip.
- Environment drift: pin base images/versions for self-hosted.

---

## Estimates (rough)

- Phase 0–2 (X11 local + CI): 1–2 days.
- Phase 4 (wlroots self-hosted): 1 day after runner ready.
- Phase 5 (KDE portal): 2–4 days incl. runner prep.
- Phase 6 (ydotool): 0.5–1 day on privileged runner.
- Hardening/docs: 0.5–1 day.

---

## Acceptance checklist (roll-up)

- [ ] X11 tests run and pass in hosted CI with high reliability.
- [ ] Wayland wlroots tests run and pass on self-hosted runner.
- [ ] Optional KDE portal tests pass on Plasma self-hosted runner.
- [ ] Clear docs + toggles; tests skip cleanly when prerequisites missing.

---

## Nice-to-haves (later)

- Record short video/gif from Xvfb session for failures.
- Add AT‑SPI Rust-based verifier to avoid Python dependency.
- Build a small Rust “input driver” abstraction for Wayland portal calls.
