# Issue: Benchmarking Harness & Performance Alignment (#222)

**Status:** Existing (needs body refresh)  
**Related Issues:** #42, #44, #45, #47, #173, #221, Testing Infra Phase 2  
**Tags:** `benchmarking`, `performance`, `testing`, `infrastructure`

## Summary

Issue #222 is the canonical tracker for building and maintaining the benchmarking harness that watches STT, VAD, and text-injection performance. The goal is to unify the scattered performance tickets (#42, #44, #45, #47) under one actionable roadmap so we can wire metrics into CI and tie them to the Testing Infrastructure Phase 2 efforts.

## Body Template (for GitHub)

> **Title:** Benchmarking Harness & Performance Alignment  
>  
> This issue consolidates the performance tracking items from #42, #44, #45, and #47. It also feeds Testing Infrastructure Phase 2 so we have a single benchmarking harness that runs inside CI and locally.  
>  
> **Scope:**  
> - Build a reusable benchmarking harness (CLI + CI job) that measures STT latency, VAD throughput, and clipboard/text-injection timings.  
> - Produce a baseline data set checked into `test/benchmarks/` (or similar) so regressions can be detected.  
> - Wire the harness into CI (nightly + PR opt-in) and surface results in PR comments.  
> - Tag each legacy performance issue as complete once their specific metrics are represented in this harness.  
>  
> **Linked issues:** #42, #44, #45, #47, #173 (VM matrix alignment), #221 (golden master coverage).  
>  
> **Acceptance:**  
> - `benchmarking` label present on all of the linked issues.  
> - CI job `benchmarking-harness` (or equivalent) runs and publishes metrics artifacts.  
> - Documentation added under `docs/testing/` describing how to run the harness locally and how to interpret regressions.

## Implementation Checklist

- [ ] Update the GitHub issue body using the template above (include the Linked issues section).
- [ ] Add `benchmarking` + `status:roll-forward` labels to #42/#44/#45/#47/#222.
- [ ] Reference this document from `docs/tasks/issue-triage-2025-11-10.md` (done).
- [ ] Link the harness work back into Testing Infrastructure Phase 2 once the GH issue is updated.
