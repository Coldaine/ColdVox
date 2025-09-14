# Drafting Flows with Soft Links

The architecture framework enables fast drafting through the use of soft links, allowing authors to express intent before all details are resolved.

## Early Drafting with Aspirational Edges

During early design phases, authors can write aspirational edges using soft links:

* "Buoyancy depends on *some* water-height API" → `depends_on: … strength: soft`
* When the interface is named → swap to `PHYS-SPEC5-060-water-height-interface` (hard).
* When code lands → add `implements → CODE:…` (hard) with evidence (commit/symbol).

## Benefits

This approach provides:
* **Speed while drafting** - Authors don't get blocked waiting for all details to be resolved
* **Enforcement at gates** - Soft links must be promoted to hard links at specific milestones
* **No whiplash** - Clear promotion rules prevent last-minute surprises

## Example Flow

```yaml
# Early draft with soft link
id: PHYS-SPEC5-044-buoyancy-spec
links:
  depends_on:
    - id: "TBD-water-height-api"
      strength: soft
      rationale: "Need water height for buoyancy calculations"

# Later refined with hard link
id: PHYS-SPEC5-044-buoyancy-spec
links:
  depends_on:
    - id: PHYS-SPEC5-060-water-height-interface
      strength: hard
      evidence: 
        kind: "commit"
        value: "a1b2c3d"
```

This drafting flow allows teams to move quickly in the early stages while ensuring rigor is applied at critical gates.