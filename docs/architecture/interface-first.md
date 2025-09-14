# Interface-First Approach for Reuse

The architecture framework promotes an interface-first approach to handle reuse scenarios, particularly when multiple consumers need the same functionality.

## The Pattern

Model the shared dependency as an **interface spec** at the specification level (L5), then point many implementations at it.

## Example

```
PHYS-SPEC5-060 Water Height Interface  <-- backbone
   ↑ satisfies REQ4-… (accuracy/latency)
   ↓ implemented by:
      CODE:…/GersterHeightProvider.cs
      CODE:…/FFTHeightProvider.cs
Used by:
   PHYS-SPEC5-044 Buoyancy Spec
   PHYS-SPEC5-071 Floating Debris Spec
   PHYS-SPEC5-089 Wave-Visual Sync Spec
```

## Benefits

This approach provides several advantages:

1. Each consumer has a **hard** `depends_on` to the interface (stable)
2. Freedom to swap implementations without breaking upstream specs
3. Clear separation between interface (design) and implementation (code)
4. Multiple implementations can coexist and be tested independently

## Implementation

When using this pattern:

* Define the interface as a specification in the backbone
* Ensure the interface satisfies relevant requirements
* Link implementations to the interface via `implements` edges
* Have consumer specifications depend on the interface rather than specific implementations

This approach enables flexible architecture while maintaining the rigor of the design backbone.