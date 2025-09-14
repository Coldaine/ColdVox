# Link Strengths: Soft vs Hard

In the architecture framework, we use link attributes to allow flexible drafting while maintaining rigor where needed. Links can have two strengths: soft or hard.

## Definitions

**Soft Link** = Intent / design expectation (allowed to be unresolved or imprecise).
**Hard Link** = Machine-resolvable target (exact node ID, symbol path, file+lines, commit).

## Example

```yaml
links:
  depends_on:
    - id: PHYS-SPEC5-052-fluid-density-table
      strength: soft        # soft | hard
      rationale: "Likely density lookup"
      evidence: null        # optional proof now, required for hard
    - id: CODE:repo://game/Ocean/IWaterHeightProvider.cs
      strength: hard
      evidence:
        kind: "symbol"      # symbol | file | commit | url
        value: "IWaterHeightProvider.GetHeight(x,z,t)"
```

## Promotion Rules

The framework defines clear rules for when soft links must be promoted to hard links:

* When a node's `status` moves to **approved** → all **critical** edges must be hard.
* When a node's `status` moves to **implemented** → **all** outward edges must be hard (or waived with expiry).
* CI blocks merges if required edges aren't hard at these gates.

This approach allows for speed during drafting while ensuring enforcement at critical gates, preventing last-minute surprises.