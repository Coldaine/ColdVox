#!/usr/bin/env python3
"""Prepare and evaluate semantic documentation review packets for LLM CI checks.

Default behavior is provider-agnostic:
- gather changed docs between two git refs
- write a review packet JSON
- write a strict prompt text

Optional behavior:
- read an LLM JSON result and fail CI by severity threshold
"""

from __future__ import annotations

import argparse
from fnmatch import fnmatch
import json
import os
import re
import subprocess
import sys
import urllib.error
import urllib.request
from pathlib import Path
from typing import Dict, List, Tuple

DOCS_ROOT = Path("docs")

SEVERITY_ORDER = {
    "none": 0,
    "minor": 1,
    "major": 2,
    "critical": 3,
}

ALLOWED_SEVERITIES = set(SEVERITY_ORDER.keys())
ALLOWED_DECISIONS = {"keep", "revise", "archive", "delete"}
ALLOWED_INTENT_TYPES = {
    "northstar",
    "spec",
    "implementation",
    "research",
    "history",
    "playbook",
    "task",
    "reference",
}

# Normalize legacy/non-canonical labels to keep rollout flexible.
DOC_TYPE_ALIASES = {
    "implementation-plan": "plan",
    "dev-guide": "reference",
    "runbook": "playbook",
}

# Flexible path policy:
# - preferred: strongest fit, no finding
# - allowed: accepted but drift warning (minor)
# - neither: placement violation (major)
DOC_TYPE_PATH_POLICY = {
    "architecture": {
        "preferred": (
            "docs/architecture.md",
            "docs/northstar.md",
            "docs/architecture/**",
            "docs/domains/**",
            "docs/dev/CI/**",
        ),
        "allowed": (
            "docs/plans/**",
            "docs/archive/**",
        ),
    },
    "standard": {
        "preferred": (
            "docs/standards.md",
            "docs/todo.md",
            "docs/anchor-*.md",
            "docs/repo/**",
        ),
        "allowed": (
            "docs/*.md",
            "docs/dev/**",
        ),
    },
    "playbook": {
        "preferred": (
            "docs/playbooks/**",
            "docs/observability-playbook.md",
            "docs/MasterDocumentationPlaybook.md",
        ),
        "allowed": (
            "docs/*.md",
            "docs/domains/**",
            "docs/archive/**",
        ),
    },
    "reference": {
        "preferred": (
            "docs/reference/**",
            "docs/domains/**",
            "docs/repo/**",
            "docs/dependencies.md",
            "docs/logging.md",
        ),
        "allowed": (
            "docs/*.md",
            "docs/playbooks/**",
        ),
    },
    "research": {
        "preferred": (
            "docs/research/**",
            "docs/archive/research/**",
        ),
        "allowed": (
            "docs/plans/**",
            "docs/history/**",
            "docs/archive/**",
        ),
    },
    "plan": {
        "preferred": (
            "docs/plans/**",
            "docs/tasks/**",
            "docs/issues/**",
        ),
        "allowed": (
            "docs/research/**",
            "docs/archive/plans/**",
            "docs/domains/**",
        ),
    },
    "troubleshooting": {
        "preferred": (
            "docs/issues/**",
            "docs/domains/**/troubleshooting/**",
        ),
        "allowed": (
            "docs/playbooks/**",
            "docs/tasks/**",
            "docs/domains/**",
        ),
    },
    "index": {
        "preferred": (
            "docs/index.md",
            "docs/reference/**",
            "docs/archive/reference/**",
            "docs/**/index.md",
            "docs/**/overview.md",
            "docs/**/*-overview.md",
        ),
        "allowed": (
            "docs/domains/**",
            "docs/archive/**",
        ),
    },
    "history": {
        "preferred": (
            "docs/history/**",
            "docs/archive/**",
        ),
        "allowed": (
            "docs/research/logs/**",
        ),
    },
}


def run_git(args: List[str]) -> str:
    try:
        result = subprocess.run(
            ["git"] + args,
            capture_output=True,
            text=True,
            check=True,
        )
        return result.stdout
    except subprocess.CalledProcessError as exc:
        stderr = exc.stderr.strip() if exc.stderr else "(no stderr)"
        raise RuntimeError(
            f"git {' '.join(args)} failed (exit {exc.returncode}): {stderr}"
        ) from exc


def git_changed_docs(base: str, head: str) -> List[Tuple[str, Path]]:
    output = run_git(["diff", "--name-status", "--diff-filter=AMR", base, head])
    rows: List[Tuple[str, Path]] = []
    for line in output.splitlines():
        if not line.strip():
            continue
        parts = line.split("\t")
        status = parts[0]
        if status.startswith("R"):
            path = Path(parts[-1])
            code = "R"
        else:
            code = status[0]
            path = Path(parts[1])
        if path.suffix == ".md" and path.parts and path.parts[0] == "docs":
            rows.append((code, path))
    return rows


def parse_frontmatter(text: str) -> Tuple[Dict[str, str], str]:
    if not text.startswith("---\n"):
        return {}, text
    try:
        closing = text.index("\n---", 4)
    except ValueError:
        return {}, text

    header = text[4:closing]
    body = text[closing + 4 :].lstrip("\n")
    values: Dict[str, str] = {}
    for line in header.splitlines():
        stripped = line.strip()
        if not stripped or stripped.startswith("#") or ":" not in stripped:
            continue
        key, raw = stripped.split(":", 1)
        values[key.strip()] = raw.strip().strip("'\"")
    return values, body


def path_matches(path: str, patterns: Tuple[str, ...]) -> bool:
    return any(fnmatch(path, pattern) for pattern in patterns)


def normalize_doc_type(raw: str) -> str:
    cleaned = raw.strip().lower()
    return DOC_TYPE_ALIASES.get(cleaned, cleaned)


def evaluate_path_policy(path: str, frontmatter: Dict[str, str]) -> Dict[str, object]:
    raw_doc_type = frontmatter.get("doc_type", "").strip()
    if not raw_doc_type:
        # Missing doc_type makes placement intent unknowable, so treat as hard policy gap.
        return {
            "status": "missing-doc-type",
            "severity": "major",
            "raw_doc_type": "",
            "normalized_doc_type": "",
            "reason": "frontmatter doc_type is missing",
            "preferred_paths": [],
            "allowed_paths": [],
        }

    normalized_doc_type = normalize_doc_type(raw_doc_type)
    policy = DOC_TYPE_PATH_POLICY.get(normalized_doc_type)
    if not policy:
        # Unknown types create silent taxonomy drift; fail loudly so we extend policy deliberately.
        return {
            "status": "unknown-doc-type",
            "severity": "major",
            "raw_doc_type": raw_doc_type,
            "normalized_doc_type": normalized_doc_type,
            "reason": f"doc_type '{raw_doc_type}' is not recognized by policy",
            "preferred_paths": [],
            "allowed_paths": [],
        }

    preferred_paths = tuple(policy.get("preferred", ()))
    allowed_paths = tuple(policy.get("allowed", ()))

    if path_matches(path, preferred_paths):
        return {
            "status": "ok",
            "severity": "none",
            "raw_doc_type": raw_doc_type,
            "normalized_doc_type": normalized_doc_type,
            "reason": "path matches preferred placement policy",
            "preferred_paths": list(preferred_paths),
            "allowed_paths": list(allowed_paths),
        }

    if path_matches(path, allowed_paths):
        # Allowed-but-not-preferred is intentional flexibility: warn without blocking by default.
        return {
            "status": "placement-drift",
            "severity": "minor",
            "raw_doc_type": raw_doc_type,
            "normalized_doc_type": normalized_doc_type,
            "reason": "path is allowed but not preferred for this doc_type",
            "preferred_paths": list(preferred_paths),
            "allowed_paths": list(allowed_paths),
        }

    return {
        "status": "placement-invalid",
        "severity": "major",
        "raw_doc_type": raw_doc_type,
        "normalized_doc_type": normalized_doc_type,
        "reason": "path does not match preferred or allowed placement policy",
        "preferred_paths": list(preferred_paths),
        "allowed_paths": list(allowed_paths),
    }


def first_heading(body: str) -> str:
    for line in body.splitlines():
        if line.startswith("# "):
            return line[2:].strip()
    return ""


def summarize_body(body: str, max_chars: int) -> str:
    compact = re.sub(r"\s+", " ", body).strip()
    if len(compact) <= max_chars:
        return compact
    return compact[: max_chars - 3] + "..."


def extract_claim_lines(body: str, max_claims: int = 8) -> List[str]:
    pattern = re.compile(
        r"\b("
        r"must|should|required|canonical|source of truth|works|supported|"
        r"broken|planned|deprecated|removed|archive|delete"
        r")\b",
        flags=re.IGNORECASE,
    )
    claims: List[str] = []
    for raw in body.splitlines():
        line = raw.strip()
        if not line:
            continue
        if line.startswith("#") or line.startswith("```"):
            continue
        if pattern.search(line):
            claims.append(line[:220])
        if len(claims) >= max_claims:
            break
    return claims


def load_northstar_goals(path: Path = Path("docs/northstar.md")) -> List[str]:
    if not path.exists():
        return []
    text = path.read_text(encoding="utf-8")
    _, body = parse_frontmatter(text)
    goals: List[str] = []
    in_target = False
    for raw in body.splitlines():
        line = raw.strip()
        if line.startswith("## "):
            section = line[3:].strip().lower()
            in_target = section in {"core goals", "execution priority (current)"}
            continue
        if in_target and line.startswith("- "):
            goals.append(line[2:].strip())
    return goals


def collect_docs(
    changed: List[Tuple[str, Path]],
    max_docs: int,
    max_chars: int,
) -> List[Dict]:
    docs: List[Dict] = []
    for status, path in changed[:max_docs]:
        try:
            text = path.read_text(encoding="utf-8")
        except FileNotFoundError:
            continue
        frontmatter, body = parse_frontmatter(text)
        docs.append(
            {
                "path": path.as_posix(),
                "change_type": status,
                "has_frontmatter": bool(frontmatter),
                "frontmatter": frontmatter,
                "policy": evaluate_path_policy(path.as_posix(), frontmatter),
                "title": first_heading(body),
                "claims": extract_claim_lines(body),
                "body_excerpt": summarize_body(body, max_chars=max_chars),
                "body_char_count": len(body),
            }
        )
    return docs


def collect_policy_findings(docs: List[Dict]) -> List[Dict]:
    findings: List[Dict] = []
    for doc in docs:
        policy = doc.get("policy", {})
        severity = policy.get("severity", "none")
        if severity == "none":
            continue
        findings.append(
            {
                "path": doc.get("path", ""),
                "severity": severity,
                "status": policy.get("status", ""),
                "reason": policy.get("reason", ""),
                "raw_doc_type": policy.get("raw_doc_type", ""),
                "normalized_doc_type": policy.get("normalized_doc_type", ""),
                "preferred_paths": policy.get("preferred_paths", []),
            }
        )
    return findings


def render_path_policy() -> str:
    lines: List[str] = []
    for doc_type in sorted(DOC_TYPE_PATH_POLICY.keys()):
        policy = DOC_TYPE_PATH_POLICY[doc_type]
        preferred = ", ".join(policy.get("preferred", ()))
        allowed = ", ".join(policy.get("allowed", ()))
        lines.append(f"- {doc_type}")
        lines.append(f"  - preferred: {preferred}")
        lines.append(f"  - allowed: {allowed}")
    return "\n".join(lines)


def build_output_schema() -> Dict:
    return {
        "type": "object",
        "required": ["global", "documents", "cross_doc_conflicts", "in_scope_issues"],
        "additionalProperties": False,
        "properties": {
            "global": {
                "type": "object",
                "required": ["overall", "summary", "max_severity", "blocking_reasons"],
                "additionalProperties": False,
                "properties": {
                    "overall": {"type": "string", "enum": ["pass", "fail"]},
                    "summary": {"type": "string"},
                    "max_severity": {
                        "type": "string",
                        "enum": sorted(ALLOWED_SEVERITIES),
                    },
                    "blocking_reasons": {"type": "array", "items": {"type": "string"}},
                },
            },
            "documents": {
                "type": "array",
                "items": {
                    "type": "object",
                    "required": [
                        "path",
                        "decision",
                        "intent_type",
                        "severity",
                        "confidence",
                        "reason",
                        "required_actions",
                        "frontmatter_patch",
                        "similar_docs",
                    ],
                    "additionalProperties": False,
                    "properties": {
                        "path": {"type": "string"},
                        "decision": {"type": "string", "enum": sorted(ALLOWED_DECISIONS)},
                        "intent_type": {
                            "type": "string",
                            "enum": sorted(ALLOWED_INTENT_TYPES),
                        },
                        "severity": {
                            "type": "string",
                            "enum": sorted(ALLOWED_SEVERITIES),
                        },
                        "confidence": {"type": "number", "minimum": 0, "maximum": 1},
                        "reason": {"type": "string"},
                        "required_actions": {
                            "type": "array",
                            "items": {"type": "string"},
                        },
                        "frontmatter_patch": {
                            "type": "object",
                            "additionalProperties": {"type": "string"},
                        },
                        "similar_docs": {"type": "array", "items": {"type": "string"}},
                    },
                },
            },
            "cross_doc_conflicts": {
                "type": "array",
                "items": {
                    "type": "object",
                    "required": [
                        "docs",
                        "severity",
                        "issue",
                        "recommended_source_of_truth",
                    ],
                    "additionalProperties": False,
                    "properties": {
                        "docs": {"type": "array", "items": {"type": "string"}},
                        "severity": {
                            "type": "string",
                            "enum": sorted(ALLOWED_SEVERITIES),
                        },
                        "issue": {"type": "string"},
                        "recommended_source_of_truth": {"type": "string"},
                    },
                },
            },
            "in_scope_issues": {"type": "array", "items": {"type": "string"}},
        },
    }


def build_prompt(packet: Dict) -> str:
    schema = json.dumps(build_output_schema(), indent=2)
    packet_json = json.dumps(packet, indent=2)

    return f"""# ColdVox Semantic Documentation Review Prompt

You are reviewing documentation quality for ColdVox.

Primary objective:
- Keep documentation useful for decision-making and execution against North Star.
- Allow aspirational and research docs, but they must be labeled and non-misleading.
- Be aggressive about archiving low-value docs; only delete when there is no salvage value.

Hard rules:
1. If a doc claims shipped behavior, check for evidence references to code/tests/config paths.
2. If intent is aspirational/research, do not punish implementation mismatch by default.
3. Prefer `archive` over `delete` when ideas may still be useful.
4. Flag cross-document contradictions and propose one source of truth.
5. Enforce doc placement policy using doc_type and path map. Treat placement-invalid as at least major unless strong justification exists.
6. Output must be strictly valid JSON matching the schema below.

Output JSON schema:
```json
{schema}
```

Severity guidance:
- `none`: no action.
- `minor`: clarity or metadata cleanup.
- `major`: likely to mislead implementation or planning.
- `critical`: blocks execution or directly contradicts canonical direction.

Decision guidance:
- `keep`: acceptable as-is.
- `revise`: valuable but needs edits.
- `archive`: not active guidance, but preserve history/ideas.
- `delete`: redundant/no salvage.

North Star anchors:
{chr(10).join(f"- {goal}" for goal in packet.get("northstar_goals", [])) or "- (no northstar goals found)"}

Doc type aliases:
{json.dumps(DOC_TYPE_ALIASES, indent=2)}

Doc placement policy:
{render_path_policy()}

Input packet:
```json
{packet_json}
```

Return only JSON, no prose outside JSON.
"""


def extract_json_from_text(text: str) -> Dict:
    """Best-effort JSON extraction for providers that wrap JSON in prose/code fences."""
    cleaned = text.strip()
    if cleaned.startswith("```"):
        cleaned = re.sub(r"^```[a-zA-Z]*\s*", "", cleaned)
        cleaned = re.sub(r"\s*```$", "", cleaned)
    try:
        return json.loads(cleaned)
    except json.JSONDecodeError:
        pass

    start = cleaned.find("{")
    end = cleaned.rfind("}")
    if start != -1 and end != -1 and end > start:
        return json.loads(cleaned[start : end + 1])
    raise ValueError("provider response did not contain valid JSON")


def call_openai_compatible(
    prompt: str,
    schema: Dict,
    model: str,
    base_url: str,
    api_key: str,
    timeout_sec: int = 90,
) -> Dict:
    """Call an OpenAI-compatible chat completions endpoint with strict JSON schema output."""
    url = base_url.rstrip("/") + "/chat/completions"
    payload = {
        "model": model,
        "messages": [
            {
                "role": "system",
                "content": (
                    "You are a strict documentation quality judge. "
                    "Return only valid JSON matching the provided schema."
                ),
            },
            {
                "role": "user",
                "content": prompt,
            },
        ],
        "temperature": 0,
        "response_format": {
            "type": "json_schema",
            "json_schema": {
                "name": "coldvox_docs_semantic_review",
                "strict": True,
                "schema": schema,
            },
        },
    }
    data = json.dumps(payload).encode("utf-8")
    req = urllib.request.Request(
        url,
        data=data,
        method="POST",
        headers={
            "Content-Type": "application/json",
            "Authorization": f"Bearer {api_key}",
        },
    )
    try:
        with urllib.request.urlopen(req, timeout=timeout_sec) as response:
            raw = response.read().decode("utf-8")
    except urllib.error.HTTPError as err:
        body = err.read().decode("utf-8", errors="replace")
        raise RuntimeError(f"provider HTTP {err.code}: {body}") from err
    except urllib.error.URLError as err:
        raise RuntimeError(f"provider connection failed: {err}") from err

    parsed = json.loads(raw)
    choices = parsed.get("choices", [])
    if not choices:
        raise RuntimeError("provider response missing choices")

    message = choices[0].get("message", {})
    content = message.get("content")
    if isinstance(content, str):
        return extract_json_from_text(content)

    if isinstance(content, list):
        text_parts = []
        for part in content:
            if isinstance(part, dict) and part.get("type") in ("text", "output_text"):
                text_parts.append(part.get("text", ""))
        joined = "\n".join(p for p in text_parts if p).strip()
        if joined:
            return extract_json_from_text(joined)

    raise RuntimeError("provider response missing parseable content")


def validate_model_output(model_output: Dict) -> List[str]:
    errors: List[str] = []

    for key in ("global", "documents", "cross_doc_conflicts", "in_scope_issues"):
        if key not in model_output:
            errors.append(f"missing top-level key: {key}")

    global_block = model_output.get("global", {})
    max_severity = global_block.get("max_severity")
    if max_severity not in ALLOWED_SEVERITIES:
        errors.append(f"invalid global.max_severity: {max_severity}")

    for idx, doc in enumerate(model_output.get("documents", []), start=1):
        sev = doc.get("severity")
        if sev not in ALLOWED_SEVERITIES:
            errors.append(f"documents[{idx}] invalid severity: {sev}")
        decision = doc.get("decision")
        if decision not in ALLOWED_DECISIONS:
            errors.append(f"documents[{idx}] invalid decision: {decision}")
        intent = doc.get("intent_type")
        if intent not in ALLOWED_INTENT_TYPES:
            errors.append(f"documents[{idx}] invalid intent_type: {intent}")
        conf = doc.get("confidence")
        if not isinstance(conf, (int, float)) or conf < 0 or conf > 1:
            errors.append(f"documents[{idx}] invalid confidence: {conf}")

    for idx, conflict in enumerate(model_output.get("cross_doc_conflicts", []), start=1):
        sev = conflict.get("severity")
        if sev not in ALLOWED_SEVERITIES:
            errors.append(f"cross_doc_conflicts[{idx}] invalid severity: {sev}")

    return errors


def max_observed_severity(model_output: Dict) -> str:
    seen = [model_output.get("global", {}).get("max_severity", "none")]
    seen.extend(doc.get("severity", "none") for doc in model_output.get("documents", []))
    seen.extend(
        conflict.get("severity", "none")
        for conflict in model_output.get("cross_doc_conflicts", [])
    )
    seen = [s for s in seen if s in SEVERITY_ORDER]
    if not seen:
        return "none"
    return max(seen, key=lambda x: SEVERITY_ORDER[x])


def should_fail(worst: str, threshold: str) -> bool:
    if threshold == "none":
        return False
    # Use a monotonic numeric map so policy and model gates share the exact same semantics.
    return SEVERITY_ORDER.get(worst, 0) >= SEVERITY_ORDER[threshold]


def worst_policy_severity(findings: List[Dict]) -> str:
    if not findings:
        return "none"
    return max(
        [f.get("severity", "none") for f in findings if f.get("severity") in SEVERITY_ORDER],
        key=lambda x: SEVERITY_ORDER[x],
        default="none",
    )


def max_observed_severity_with_confidence(model_output: Dict, min_confidence: float) -> str:
    seen = [model_output.get("global", {}).get("max_severity", "none")]
    for doc in model_output.get("documents", []):
        severity = doc.get("severity", "none")
        confidence = doc.get("confidence", 0.0)
        if severity in SEVERITY_ORDER and isinstance(confidence, (int, float)):
            if confidence >= min_confidence:
                seen.append(severity)

    # Conflicts do not carry confidence in this schema, so include as-is.
    for conflict in model_output.get("cross_doc_conflicts", []):
        severity = conflict.get("severity", "none")
        if severity in SEVERITY_ORDER:
            seen.append(severity)

    seen = [s for s in seen if s in SEVERITY_ORDER]
    if not seen:
        return "none"
    return max(seen, key=lambda x: SEVERITY_ORDER[x])


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("base")
    parser.add_argument("head")
    parser.add_argument("--max-docs", type=int, default=30)
    parser.add_argument("--max-body-chars", type=int, default=2500)
    parser.add_argument(
        "--packet-out",
        default=".artifacts/docs_semantic_review_packet.json",
    )
    parser.add_argument(
        "--prompt-out",
        default=".artifacts/docs_semantic_review_prompt.md",
    )
    parser.add_argument(
        "--llm-output",
        default="",
        help="Path to LLM JSON output to validate and evaluate.",
    )
    parser.add_argument(
        "--fail-on",
        choices=["none", "minor", "major", "critical"],
        default="major",
    )
    parser.add_argument(
        "--enforce-policy",
        action="store_true",
        help="Fail when deterministic path policy findings meet threshold.",
    )
    parser.add_argument(
        "--policy-fail-on",
        choices=["none", "minor", "major", "critical"],
        default="major",
    )
    parser.add_argument(
        "--run-openai",
        action="store_true",
        help="Run OpenAI-compatible semantic judging and write --llm-output.",
    )
    parser.add_argument(
        "--openai-base-url",
        default=os.getenv("OPENAI_BASE_URL", "https://api.openai.com/v1"),
    )
    parser.add_argument(
        "--openai-model",
        default=os.getenv("OPENAI_MODEL", "gpt-5-mini"),
    )
    parser.add_argument(
        "--openai-api-key",
        default=os.getenv("OPENAI_API_KEY", ""),
    )
    parser.add_argument(
        "--allow-missing-openai-key",
        action="store_true",
        help="If --run-openai is set but key is missing, emit warning and continue.",
    )
    parser.add_argument(
        "--min-confidence",
        type=float,
        default=0.8,
        help="Minimum confidence for doc-level findings to count toward blocking.",
    )
    args = parser.parse_args()

    changed = git_changed_docs(args.base, args.head)
    docs = collect_docs(changed, max_docs=args.max_docs, max_chars=args.max_body_chars)
    policy_findings = collect_policy_findings(docs)

    packet = {
        "repo": "ColdVox",
        "base": args.base,
        "head": args.head,
        "northstar_path": "docs/northstar.md",
        "northstar_goals": load_northstar_goals(),
        "changed_docs_count": len(changed),
        "review_docs_count": len(docs),
        "doc_type_aliases": DOC_TYPE_ALIASES,
        "doc_type_path_policy": DOC_TYPE_PATH_POLICY,
        "policy_findings": policy_findings,
        "docs": docs,
    }

    packet_out = Path(args.packet_out)
    prompt_out = Path(args.prompt_out)
    packet_out.parent.mkdir(parents=True, exist_ok=True)
    prompt_out.parent.mkdir(parents=True, exist_ok=True)
    packet_out.write_text(json.dumps(packet, indent=2) + "\n", encoding="utf-8")
    prompt_text = build_prompt(packet)
    prompt_out.write_text(prompt_text, encoding="utf-8")

    print(f"[OK] Wrote packet: {packet_out}")
    print(f"[OK] Wrote prompt: {prompt_out}")
    print(f"[INFO] Changed docs detected: {len(changed)}")
    print(f"[INFO] Docs included in packet: {len(docs)}")
    if policy_findings:
        print(f"[WARN] Policy findings: {len(policy_findings)}")
        for finding in policy_findings:
            print(
                f"::warning ::{finding['path']}: [{finding['severity']}] "
                f"{finding['status']} - {finding['reason']}"
            )

    if args.enforce_policy:
        # Deterministic placement checks can block independently of LLM availability/reliability.
        worst_policy = worst_policy_severity(policy_findings)
        print(f"[INFO] Worst policy severity: {worst_policy}")
        if should_fail(worst_policy, args.policy_fail_on):
            print(
                f"::error ::Doc placement policy failed at threshold "
                f"{args.policy_fail_on} (worst={worst_policy})"
            )
            sys.exit(1)

    llm_output_path = args.llm_output
    if args.run_openai:
        if not args.openai_api_key:
            message = (
                "OPENAI_API_KEY not set; skipping provider-backed semantic review."
                if args.allow_missing_openai_key
                else "OPENAI_API_KEY not set and --run-openai requested."
            )
            if args.allow_missing_openai_key:
                print(f"::warning ::{message}")
                return
            print(f"::error ::{message}")
            sys.exit(1)

        schema = build_output_schema()
        try:
            model_output = call_openai_compatible(
                prompt=prompt_text,
                schema=schema,
                model=args.openai_model,
                base_url=args.openai_base_url,
                api_key=args.openai_api_key,
            )
        except Exception as err:  # noqa: BLE001
            print(f"::error ::OpenAI-compatible review call failed: {err}")
            sys.exit(1)

        if not llm_output_path:
            llm_output_path = ".artifacts/docs_semantic_review_result.json"
        out_path = Path(llm_output_path)
        out_path.parent.mkdir(parents=True, exist_ok=True)
        out_path.write_text(json.dumps(model_output, indent=2) + "\n", encoding="utf-8")
        print(f"[OK] Wrote LLM output: {out_path}")

    if not llm_output_path:
        return

    llm_path = Path(llm_output_path)
    try:
        model_output = json.loads(llm_path.read_text(encoding="utf-8"))
    except FileNotFoundError:
        print(f"::error ::LLM output file not found: {llm_path}")
        sys.exit(1)
    except json.JSONDecodeError as exc:
        print(f"::error ::LLM output is invalid JSON: {exc}")
        sys.exit(1)
    validation_errors = validate_model_output(model_output)
    if validation_errors:
        for err in validation_errors:
            print(f"::error ::Invalid LLM output: {err}")
        sys.exit(1)

    worst = max_observed_severity_with_confidence(
        model_output, min_confidence=args.min_confidence
    )
    print(f"[INFO] LLM max severity (confidence >= {args.min_confidence}): {worst}")
    if should_fail(worst, args.fail_on):
        summary = model_output.get("global", {}).get("summary", "semantic docs review failed")
        print(f"::error ::{summary}")
        for reason in model_output.get("global", {}).get("blocking_reasons", []):
            print(f"::error ::{reason}")
        sys.exit(1)


if __name__ == "__main__":
    main()
