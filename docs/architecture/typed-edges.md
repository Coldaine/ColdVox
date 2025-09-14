# Typed Edges with Direction and Cardinality

The architecture framework uses typed edges with specific direction and cardinality rules to maintain graph integrity and ensure proper relationships between components.

## Edge Types and Rules

| Edge             | From → To                 | Notes / gates                            |
| ---------------- | ------------------------- | ---------------------------------------- |
| `satisfies`      | FEA/SPEC → REQ            | Spec must satisfy ≥1 REQ before approval |
| `depends_on`     | any → any (not ancestor)  | Soft allowed in draft; hard before impl  |
| `implements`     | SPEC → IMP/CODE           | Hard before "implemented"                |
| `verified_by`    | REQ/FEA/SPEC → TST        | Must exist before release                |
| `interacts_with` | peer↔peer (SYS↔SYS, etc.) | For modeling runtime interactions        |
| `supersedes`     | any → any (same type)     | Deprecation/ADR evolution                |

## CI Validation Rules

To maintain graph integrity, the framework enforces these CI checks:

* No `depends_on` to your own `parent`/`child` (that's hierarchy).
* `interacts_with` only between peers (same level).
* Missing required edges → fail.

These typed edges ensure that relationships between components are explicit, validated, and meaningful, preventing ambiguous or incorrect connections in the architecture.