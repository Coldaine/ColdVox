# Handling Bottom-Up Reuse Cases

The architecture framework addresses the challenge of bottom-up reuse where implementation artifacts may serve multiple use cases, breaking the single-parent rule.

## The Challenge

At the implementation level, a utility may serve multiple use cases, so the single-parent constraint no longer holds. For example, a water height provider might be used by buoyancy calculations, floating debris simulation, and wave visualization.

## The Solution

Make implementations **off-tree** (or in a shallow *Code Areas* tree), and let many specifications point to them via `implements` or `depends_on` edges.

## Example

```yaml
# Buoyancy spec (design backbone)
id: PHYS-SPEC5-044-buoyancy-spec
parent: PHYS-SYS4-031-buoyancy
links:
  depends_on:
    - id: PHYS-SPEC5-060-water-height-interface    # interface contract
      strength: hard
  implements:
    - id: CODE:repo://game/Physics/Buoyancy.cs#L12-L190
      evidence: { kind: file, value: "commit 8b2c3e7" }
  verified_by: [ PHYS-TST6-310-buoyancy-acceptance ]

# Reusable code (off-tree, many-to-many)
id: CODE:repo://game/Ocean/IWaterHeightProvider.cs
type: IMP
area: Physics/Ocean
module: WaterHeight
owners: [@team-physics]
links:
  referenced_by:
    - PHYS-SPEC5-044-buoyancy-spec
    - PHYS-SPEC5-071-floating-debris-spec
    - PHYS-SPEC5-089-wave-visual-sync-spec
```

This approach ensures that:
* The *design* stays strictly hierarchical.
* The *implementation* is reusable and linked by many specs without violating hierarchy.