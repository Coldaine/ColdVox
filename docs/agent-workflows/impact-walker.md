## Impact Walker Workflow

1. Detect changes to node with `status: approved` or higher
2. Determine change class (C0-C3) based on contracts diff:
   - C0: Documentation-only changes
   - C1: Non-critical breaking changes
   - C2: Critical breaking changes
   - C3: Breaking changes requiring migration
3. Traverse edges to identify affected components:
   - For SPEC changes: find all IMP and TST nodes
   - For IMP changes: find all SPEC and TST nodes
4. Generate PR comment with:
   - Change class
   - Affected components
   - Recommended actions
