# Minimal Linter Rules

The architecture framework includes a set of linter rules to enforce the defined constraints and ensure consistency across the architecture documentation.

## Backbone Rules

* Types `{VSN,PIL,DOM,SUB,SYS,SPEC}` require exactly one `parent` one level up.
* Types `{IMP,TST,DATA,DOCS}` must NOT have more than one parent (0 or 1).

## Traceability Rules

* Any node in the backbone must have a parent chain to a `PIL` (pillar).

## Cardinality by Status

* `SPEC.status ∈ {approved, implemented}` ⇒ `satisfies ≥1 REQ` (hard by approved).
* `SPEC.status = implemented` ⇒ `implements ≥1 IMP/CODE` (hard).
* `REQ.status = ready` ⇒ `verified_by ≥1 TST` (hard by release).

## Edge Hygiene Rules

* Disallow `depends_on` to ancestor/descendant.
* `interacts_with` must be same-level.

## Hardness Gates

* At `approved`: all `critical` links hard (tag critical edges in schema or infer by type).
* At `implemented`: all outgoing links hard or waived (waiver has `expires_on`).

These rules can be enforced through a linter that validates architecture documents against these constraints, ensuring that the architecture remains consistent and follows the defined patterns.