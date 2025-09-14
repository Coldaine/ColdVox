# Strict Hierarchy and Relaxation

The architecture framework maintains a strict hierarchy for conceptual and design levels while relaxing constraints for implementation artifacts.

## Where the Strict Hierarchy Applies

The backbone applies to these levels with exactly-one-parent relationship (PIL optional and can be skipped if a pillar doesn't add value):
* **VSN** (Vision) → PIL (Pillar) → DOM (Domain) → SYS (System) → SPEC (Specification)

## Where the Hierarchy is Relaxed

For implementation artifacts, the hierarchy is relaxed because these artifacts are naturally reusable and many-to-many:
* **IMP** (Implementation/Code)
* **TST** (Tests)
* **DATA** (Assets)
* **DOCS** (Documentation/How-tos)

## Practical Rule

* `parent` is **required** for VSN…SPEC (one parent).
* `parent` is **optional** for IMP/TST/DATA/DOCS. Instead, they must carry (optional for discoverability):

  * `area`: e.g., `Physics/Ocean`
  * `module`: e.g., `WaterHeight`
  * `owners`: team or codeowner

These attributes let you group code for ownership without pretending there's a single "parent feature."

## Consolidation Rules

To reduce overhead:
- Merge content from eliminated levels (e.g., former SUB subdomains) into the nearest meaningful parent (e.g., DOM or SYS).
- Eliminate placeholder documents that only fill hierarchy slots; integrate substantial content or remove if redundant.
- Focus on meaningful documentation over strict structural compliance.

This approach keeps the design strictly hierarchical for clarity while allowing implementation artifacts to be reused across multiple features without violating the hierarchy.