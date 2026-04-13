---
doc_type: playbook
subsystem: telemetry
status: active
freshness: stale
preservation: preserve
summary: "OTel span naming, metrics taxonomy, and sampling strategy"
signals: "['otel', 'metrics', 'tracing']"
owners: Documentation Working Group
last_reviewed: 2025-11-12
---

# Organizational Observability Playbook

## 1. Purpose
Establish a cohesive, low-friction, cost-aware, multi-language observability standard spanning:
- Real‑time Rust (ColdVox family)
- Python LLM/agent services (colossus, the-Librarian, the-watchman)
- Worker/process pipelines (ComfyWatchman, ShortcutSage, coldwatch)
- Web/frontends (TS/React/Vite repos)
- Shared infra (OTel Collector/Alloy, self-hosted LGTM, Grafana Cloud, LangSmith, Logfire)

Primary goals:
1. Fast root cause isolation (p50 < 5 min triage start).
2. Predictive degradation warning (≥ 30 min early signals for memory/latency anomalies).
3. Cost governance (target: < 70% of allocated telemetry budget at steady state).
4. Developer ergonomics (instrumentation diffs < 10 lines typical; prebuilt helpers).
5. AI workflow introspection (agent reasoning traces + prompt/response spans with privacy filters).

## 2. Scope
Included data classes:
- Logs (structured + event logs)
- Metrics (system, pipeline, model usage, perf)
- Traces (distributed + agent cognitive spans)
- Derived events (error spikes, SLO burn alerts)
Excluded (initial phase):
- Full payload content archives
- Raw audio streams outside short-lived buffers
- Personally identifiable data (must be scrubbed before emission)

## 3. Guiding Principles
1. Minimal & focused by default (progressive elaboration when needed).
2. Single authoritative resource attribute schema.
3. Cost-aware sampling layered (head + tail + destination differentiation).
4. Explicit lifecycle (instrument → validate → promote → monitor → optimize).
5. Auto-validation via CI (lint + schema + frontmatter).
6. Tool neutrality—prefer OpenTelemetry APIs + thin adapters (LangSmith, Logfire integrated via exporters).
7. ColdVox pattern reuse: shared metrics handles (`Arc<PipelineMetrics>`) across pipeline stages.

## 4. Architecture Overview
High-level data flow:
Runtime apps → Local OTel SDK (language-specific) → Unified OTel Collector/Alloy → Routing/Sampling processors →
- Self-hosted LGTM stack (full fidelity internal analysis; higher retention for metrics/traces selective)
- Grafana Cloud (curated, cost-controlled subset)
- LangSmith (LLM traces, lineage, prompt quality)
- Logfire (structured Python logs enriched by Pydantic models)
- (Optional) Future ML cost analytics endpoint

Collector core pipelines:
- Receivers: otlp/http + otlp/grpc + prometheus (scrape) + filelog (optional)
- Processors: batch, memory_limiter, attributes (resource normalization), filter, spanmetrics, tail_sampling
- Exporters: loki, tempo, prometheusremote, otlp (Grafana Cloud), otlp (LangSmith gateway), http/json (Logfire), debug (dev only)
- Extensions: health_check, zpages (non-prod)

## 5. Roles & Responsibilities
- Observability Steward: Owns schema, sampling policy adjustments.
- Language Champions (Rust/Python/JS): Provide instrumentation helpers.
- Service Owners: Maintain SLO definitions & dashboards.
- Infra Team: Operates collector, retention policies, scale.
- AI/ML Lead: Curates LangSmith prompt/trace taxonomy.
- Docs Governance: Ensures frontmatter compliance & versioning.

## 6. Rollout Phases
Phase 0 (Prep):
- Define resource attribute schema.
- Introduce instrumentation helper libraries (Rust `coldvox-telemetry`, Python `observability_helpers`, JS wrapper).
- CI checks for doc frontmatter + basic span naming lint.

Phase 1 (Pilot – Rust & one Python agent service):
- ColdVox crates adopt OTel exporter (replace any ad-hoc non-OTel tagging).
- Single Python agent integrates tracing + LangSmith minimal spans.
- Validate cost baselines; adjust sampling parameters.

Phase 2 (Core Services):
- All Python LLM/agent repos instrument model invocation, prompt lifecycle, token usage metrics.
- Worker repos add queue latency, external API call spans, task outcome counters.

Phase 3 (Frontends & Cross-Cutting):
- JS apps emit Web Vitals and user action traces (correlated via traceparent).
- Introduce spanmetrics processor → RED metrics.

Phase 4 (Optimization & Governance):
- Tail sampling activated for error/latency anomalies.
- Adaptive sampling feedback loop (error rate & cost + dynamic throttle).
- Dashboard & SLO burn alerts integrated.

Abort Thresholds / Guardrails:
- > 125% telemetry budget for 2 consecutive weeks.
- p95 ingestion latency > 5s sustained 12h.
- Unbounded high-cardinality metric creation (auto quarantine via filter processor).

Phase Exit Criteria:
- Each phase requires: 95% resource attribute compliance + baseline dashboards + sampling config stable for 1 week.

## 7. Instrumentation Standards
Resource Attributes (mandatory):
- service.name
- service.version (semver/git sha short)
- service.namespace (domain: agent-llm, realtime-audio, worker-batch, web-ui)
- deployment.environment (dev/staging/prod)
- telemetry.sdk.language
- infra.region / zone (if multi-region)
- runtime.version (Python/Rust/Node)
- build.commit / build.branch (non-prod)

Span Naming Conventions:
- Rust pipeline: audio.capture, vad.evaluate, stt.decode, text.inject, telemetry.flush
- Python agents: agent.session, agent.prompt.build, llm.call, llm.stream.chunk, tool.invoke, memory.fetch
- Worker tasks: task.dequeue, task.execute.<type>, external.http.<service>, artifact.store
- JS frontend: ui.interaction.<component>, api.call.<endpoint>, page.load

Span Attributes:
- latency_ms (explicit if not auto captured)
- attempt (retry count)
- success (bool)
- model.name / model.provider / model.token.count.input / model.token.count.output
- error.type / error.message (scrubbed)
- cost.estimated_usd (if available)
- audio.frame.size / audio.sample.rate / vad.triggered (Rust)
- queue.name / queue.wait_ms (workers)
- user.action.id (frontend hashed)

Metrics Naming Format: `<domain>.<subsystem>.<metric>`
Examples:
- agent.llm.tokens.input_total (counter)
- agent.llm.tokens.output_total (counter)
- agent.llm.call_duration_ms (histogram)
- realtime.audio.frames_processed_total (counter)
- realtime.audio.vad_activation_ratio (gauge)
- worker.task.execution_duration_ms (histogram)
- worker.task.failures_total (counter)
- web.ui.interaction_latency_ms (histogram)
- system.cpu.utilization_pct (gauge)
- cost.llm.estimated_per_minute_usd (gauge)

Logs:
- Structured JSON (or key=value fallback for Loki).
- Mandatory fields: timestamp, level, service.name, trace_id (if span context), event.category, message
- Optional: span_id, thread, component, error.stack (scrub PII)
- Python: use Logfire to auto-capture Pydantic model fields (ensure whitelist).

## 8. Metrics & Logs Taxonomy
Core Categories:
- RED metrics per critical path
- Resource metrics (CPU, memory rss, GPU util)
- Throughput (requests/sec, frames/sec)
- Quality (STT accuracy proxy: partial→final ratio; agent hallucination flags)
- Cost (token usage, model call cost)
- Reliability (error rates segmented by error.type)
- Queueing (wait time, backlog depth)
Retention Strategy (initial):
- High-cardinality metrics: 7d (promote if stable)
- Core KPI & SLO metrics: 90d
- Full-fidelity logs internal: 14d; aggregated logs cloud: 7d
- Traces: 7d full (internal), 3d sampled (cloud)

## 9. Tracing & Sampling Strategy
Layers:
1. Head Sampling: baseline 20–30% for low-volume services; 5–10% for high-volume realtime frames.
2. Tail Sampling: capture spans with: error=true; latency_ms > p95*1.25; cost spikes; premium provider.
3. Destination Differentiation: internal Tempo keeps broad set; Grafana Cloud minimal tail + thin head slice; LangSmith all LLM spans; Logfire logs only.

Adaptive Adjustment Inputs: error rate delta, cost spike, latency skew.
Feedback Loop: weekly job analyzes metrics and writes new sampling config (versioned).
SpanMetrics: derives RED automatically from operation names.

## 10. Collector / Alloy Config Pattern (Outline)
Receivers: otlp(grpc/http), prometheus, filelog(optional)
Processors: batch, memory_limiter, attributes(normalize), filter(drop noisy), spanmetrics, tail_sampling
Exporters: loki, tempo, prometheusremote, otlp(Grafana Cloud), otlp(LangSmith), http(Logfire), debug(dev)
Extensions: health_check, zpages(non-prod)

## 11. Destination Routing Policy
- Loki (internal): info+warn+error (prod), debug allowed dev/staging via feature flag.
- Cloud Loki: warn+error + curated release/info events.
- Tempo internal: head + tail + agent cognitive spans.
- Cloud traces: tail + critical subset.
- LangSmith: all model invocation graph spans.
- Logfire: Python structured logs correlated via trace_id.

## 12. Governance & Quality Gates
CI:
- Resource attribute tests
- Span name lint
- Metrics cardinality guard
- Doc frontmatter schema validation
- PR label `observability-impact` auto if > threshold instrumentation changes.
Runtime:
- Collector rejection counters dashboarded
- Alert if sampling config >7d old
- SLO burn alerts (error rate, latency, cost)
Frontmatter required keys: title, service.name, instrumentation.version, last_reviewed, owner, stability.

## 13. ColdVox-Derived Patterns (Appendix)
- Shared metrics handle → standard TelemetryContext across languages.
- Pipeline stage naming → consistent parent-child spans.
- Performance-sensitive logging flags.
- VAD gating events become domain events.
- Text injection counters pattern reused for agent message injection.

## 14. Instrumentation Cheat Sheet
Rust: tracing + opentelemetry; global subscriber; avoid debug in hot loops.
Python: opentelemetry-sdk + langsmith; decorators for llm.call; Logfire structured.
JS/TS: @opentelemetry/api; traceparent propagation; Web Vitals metrics exporter.

## 15. Rollout Plan Matrix
Phase success metrics & abort guards summarized (see section 6 & 9).

## 16. Governance Automation
- Lint script for spans/metrics
- Pre-commit resource attribute injection
- PR bot cardinality risk detection
- Doc watcher ensures last_reviewed freshness
- Sampling config version check

## 17. Next-Step Enhancements
- Adaptive cost sampler
- Cross-repo end-to-end SLO dashboard
- GPU telemetry integration (if needed)
- Prompt quality scoring metrics (LangSmith + Grafana)
- Rust macro DSL for boilerplate reduction
- Privacy scrubbing processor
- AI anomaly detection on trace structures

## 18. Success Metrics & KPIs
Monthly tracked: MTTR, telemetry cost, attribute completeness %, alert false-positive rate, incident triage start time.

## 19. Risks & Mitigations
- Cardinality explosion → label allowlist
- Over-sampling expensive spans → cost throttle
- PII leakage → scrubbing processor + denylist
- Collector overload → memory_limiter + horizontal scale

## 20. Maintenance Cadence
Weekly sampling review; monthly cost/cardinality audit; quarterly schema evolution; semi-annual toolchain upgrades.

---
Feedback welcome. Initial status: experimental. Promote to beta after Phase 1 success metrics achieved.
