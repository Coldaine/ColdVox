---
doc_type: design
subsystem: gui
status: draft
freshness: current
preservation: preserve
domain_code: gui
last_reviewed: 2026-03-29
owners: Documentation Working Group
version: 1.1.0
---

# ColdVox GUI Design Overview

This document keeps the reusable GUI ideas from earlier ColdVox overlay work without treating any prior toolkit choice or prototype state as current implementation truth.

For current execution priorities, use [`../../plans/windows-multi-agent-recovery.md`](../../plans/windows-multi-agent-recovery.md).

## What This Document Is

- A collection of UI and interaction ideas worth preserving.
- A toolkit-agnostic design reference.
- A source of overlay behavior, visual feedback, and control ideas that can carry into the current Windows-first GUI path.

## What This Document Is Not

- Not a statement that a specific GUI toolkit is current.
- Not a claim that the described UI is already implemented.
- Not the current delivery plan.

## Core Design Intent

The ColdVox GUI should feel like a lightweight dictation overlay that stays visible enough to build trust without getting in the user’s way.

The UI should emphasize:

- immediate feedback while speaking
- clear system state
- low visual weight when idle
- simple interruption controls
- confidence that only finalized text gets injected

## Overlay States

### Collapsed State

The collapsed state should minimize screen footprint while still showing that ColdVox is present and ready.

Useful ideas worth preserving:

- compact horizontal bar
- always-on-top behavior
- microphone/status iconography
- single-click or equivalent quick expansion path
- visible but unobtrusive idle presence

### Expanded State

The expanded state should surface richer feedback and controls when the user is actively dictating or inspecting the pipeline.

Useful ideas worth preserving:

- larger transcript area
- dedicated activity/status region
- clearly grouped controls
- enough room for error or retry feedback
- sizing that remains readable without dominating the screen

## Transcript Presentation

The GUI should support live provisional text in the UI while speech is ongoing.

Useful ideas worth preserving:

- transcript region with readable line length
- smooth appearance of new text
- auto-scroll or equivalent “latest text stays visible” behavior
- clear distinction between ongoing/provisional display and finalized result

The UI may show text that changes, appears, or disappears while the utterance is still in progress. That provisional display is separate from final injection behavior.

## Visual State Feedback

The overlay should make system state legible at a glance.

Important states to communicate:

- idle
- listening / recording
- processing / transcribing
- completed / ready to inject
- failed / retry needed

Useful presentation ideas:

- color-coded state indication
- lightweight activity animation
- audio level visualization while capturing
- visible failure or retry messaging

## Controls

The UI should preserve a small set of high-value controls close to the transcript view.

Useful ideas worth preserving:

- stop
- pause / resume
- clear
- settings access

These controls should remain understandable even in a compact overlay form.

## Persistence and Configuration

Useful behavior worth preserving:

- window position persistence
- transparency / opacity preference
- user-facing settings access
- quick path to audio, hotkey, and appearance configuration

## Design Qualities To Preserve

- low-friction expansion and collapse
- readable transcript typography
- obvious current state
- visible but restrained animation
- quick recovery when STT or injection fails
- strong fit for a Windows-first floating overlay workflow

## Current Carry-Forward Guidance

The current GUI direction should carry forward relevant design and workflow ideas from prior ColdVox overlay explorations and from ColdVox_Mini, while remaining free to change the underlying implementation approach.
