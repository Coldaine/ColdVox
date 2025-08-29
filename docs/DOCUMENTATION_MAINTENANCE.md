# Documentation Maintenance Checklist

## Regular Maintenance (Monthly)

### ✅ File Reference Verification
```bash
# Check for broken .md file references
grep -r "\.md" docs/ --include="*.md" | grep -v "github.com"

# Verify referenced files exist
find docs/ -name "*.md" -exec grep -l "docs/" {} \; | xargs grep -h "docs/[^)]*\.md" | sort -u
```

### ✅ Implementation Status Audit
- [ ] Review PROJECT_STATUS.md phase completion claims
- [ ] Verify ✅ IMPLEMENTED vs 📋 PLANNED markers match actual code
- [ ] Update phase status based on recent commits
- [ ] Check if any "IN PROGRESS" items are now complete

### ✅ Architecture Validation  
- [ ] Compare documented threading model with `src/main.rs`
- [ ] Verify data flow diagrams match pipeline implementation
- [ ] Check component interfaces match actual APIs
- [ ] Validate configuration examples work

## Before Major Releases

### ✅ Comprehensive Review
- [ ] Read all documentation from user perspective
- [ ] Test all example commands and code snippets
- [ ] Verify build/run instructions work on fresh checkout
- [ ] Check for outdated version numbers or paths

### ✅ Status Marker Update
- [ ] Mark completed features as ✅ IMPLEMENTED
- [ ] Move finished items from 📋 PLANNED to ✅ IMPLEMENTED  
- [ ] Update 🔄 IN PROGRESS items based on current development
- [ ] Remove or archive obsolete planning documents

## After Code Changes

### ✅ Immediate Updates (per PR)
- [ ] Update docs if public APIs change
- [ ] Fix any broken references introduced
- [ ] Update configuration examples if config changes
- [ ] Maintain CLI command documentation

### ✅ Architectural Changes
- [ ] Update component diagrams if structure changes
- [ ] Revise threading model docs if concurrency changes
- [ ] Update data flow if pipeline architecture changes
- [ ] Refresh performance characteristics if optimizations made

## Documentation Quality Standards

### ✅ Accuracy Requirements
- **Implementation Claims**: Only mark ✅ IMPLEMENTED if fully working in main branch
- **File References**: All `docs/path/file.md` references must resolve to existing files
- **Code Examples**: All code snippets must compile and run
- **Command Examples**: All CLI examples must work with current build

### ✅ Status Marker Standards
| Marker | Meaning | Requirements |
|--------|---------|--------------|
| ✅ IMPLEMENTED | Feature complete and tested | Code exists, tests pass, documented |
| 🔄 IN PROGRESS | Actively being developed | Partial implementation, known next steps |
| 📋 PLANNED | Designed but not started | Clear specification, no implementation yet |

### ✅ Content Organization
- **Current Status**: Use PROJECT_STATUS.md as single source of truth
- **Detailed Plans**: Individual planning docs (with status markers)  
- **Implementation Details**: Focus on architecture, not exhaustive code details
- **Archival**: Move obsolete detailed designs to git history, keep simple summaries

## Tools and Automation

### ✅ Validation Scripts
```bash
# Find broken internal references
./scripts/check_doc_references.sh

# Validate status markers consistency
./scripts/audit_implementation_status.sh

# Check for stale planning documents
find docs/ -name "*.md" -exec grep -l "PLANNED\|TODO\|TBD" {} \;
```

### ✅ Pre-commit Hooks (Optional)
- Validate markdown syntax
- Check internal link integrity
- Flag TODO/TBD markers in non-planning documents

## Warning Signs of Stale Documentation

### 🚨 Critical Issues
- ✅ IMPLEMENTED features that don't exist in code
- Broken references to moved/deleted files
- Example commands that fail to run
- Architecture diagrams that don't match implementation

### ⚠️ Quality Issues  
- Vague status markers (e.g., "mostly complete")
- Over-detailed implementation docs for simple library usage
- Planning documents presented as current implementation
- Outdated performance claims or benchmarks

## Maintenance History

- **2025-08-29**: Initial pruning completed
  - Fixed broken references in CLAUDE.md, Phase3.md
  - Simplified ring buffer documentation (rtrb library vs custom implementation)
  - Added implementation status markers to STT integration plan
  - Created this maintenance checklist

---

## Quick Commands Reference

```bash
# Check current documentation health
grep -r "IMPLEMENTED\|PLANNED\|IN PROGRESS" docs/ | wc -l

# Find all status markers
grep -r "✅\|🔄\|📋" docs/

# Validate example commands
cd crates/app && cargo check --all-targets

# Check build commands from documentation
cargo build --features vosk
cargo run --bin tui_dashboard --help
```

Keep this checklist updated and follow it regularly to maintain high documentation quality.

### New Maintenance Tasks
- [ ] Add task to keep metrics fields and dashboard displays synchronized in Live_Test_Dashboard.md