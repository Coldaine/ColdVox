---
doc_type: troubleshooting
subsystem: stt
version: 1.0.0
status: draft
owners: Documentation Working Group
last_reviewed: 2025-10-19
---

# Vosk Model Discovery Flow - Diagnostic Guide

This document explains exactly how you reach each logging point during Vosk model initialization, making it easier to diagnose CI failures.

## Complete Flow Diagram

```
App Startup
    ↓
Plugin Manager Initialize
    ↓
Vosk Plugin.initialize() → "Initializing Vosk STT plugin - START"
    ↓
model::ensure_model_available()
    ↓
model::locate_model() ────────────────────────┐
    │                                          │
    ├─→ 1. Check VOSK_MODEL_PATH env var      │
    │   ├─→ Set? → Debug: "Trying VOSK_MODEL_PATH environment variable"
    │   │   ├─→ Valid directory? → Info: "Vosk model found via VOSK_MODEL_PATH - SUCCESS"
    │   │   └─→ Invalid? → Warn: "VOSK_MODEL_PATH points to invalid location" → FAIL (return error)
    │   └─→ Not set? → Continue to step 2
    │
    ├─→ 2. Check config.model_path
    │   ├─→ Provided & non-empty? → Debug: "Trying config-provided model path"
    │   │   ├─→ Valid directory? → Info: "Vosk model found via config path - SUCCESS"
    │   │   └─→ Invalid? → Warn: "Config-provided model path is invalid" → FAIL (return error)
    │   └─→ Not provided? → Continue to step 3
    │
    ├─→ 3. Auto-discovery (scan models/ in 4 locations)
    │   │   → Debug: "Starting auto-discovery for Vosk model"
    │   │   → find_model_candidates()
    │   │       → Scan CWD/models/, ../models/, ../../models/, ../../../models/
    │   │       → For each location:
    │   │           → Trace: "Checking models directory - SCANNING"
    │   │           → If exists & readable:
    │   │               → For each vosk-model-* subdirectory:
    │   │                   → Debug: "Found potential Vosk model directory"
    │   │               → Debug: "Completed scanning models directory - RESULT"
    │   │           → If unreadable:
    │   │               → Warn: "Failed to read models directory"
    │   │       → Debug: "Model discovery completed"
    │   │
    │   ├─→ Candidates found?
    │   │   ├─→ Yes → Debug: "Vosk model discovery candidates found"
    │   │   │   → pick_best_candidate() (prefers: small > en-us > highest version)
    │   │   │   └─→ Info: "Vosk model found via auto-discovery - SUCCESS"
    │   │   └─→ No → Debug: "No model candidates found during auto-discovery"
    │   │
    │   └─→ No valid candidate → Continue to step 4
    │
    └─→ 4. COMPLETE FAILURE - Build error message
        → Error: "Vosk model not found after exhaustive search - COMPLETE FAILURE"
        → Shows all paths checked (env, config, 4 auto-discovery locations)
        → Returns ModelError::NotFound
            ↓
        Plugin.initialize() catches error
        → Error: "Failed to locate or prepare Vosk model - CRITICAL"
        → Returns SttPluginError::InitializationFailed
            ↓
        Plugin Manager catches error
        → Error: "STT plugin initialization failed"
        → App fails to start OR STT is disabled
```

## Decision Tree: What Most Likely Happened

### Scenario 1: "Trying VOSK_MODEL_PATH environment variable"
**What happened:** User/CI explicitly set `VOSK_MODEL_PATH` environment variable

**Next steps:**
- ✅ **"Vosk model found via VOSK_MODEL_PATH - SUCCESS"**
  - **Cause:** Path exists and is a valid directory with model files
  - **Common in:** CI runners with cached models, production deployments
  
- ⚠️ **"VOSK_MODEL_PATH points to invalid location"**
  - **Cause:** Environment variable set but path is wrong
  - **Common reasons:**
    - Typo in CI configuration
    - Model extraction step failed silently
    - Cache was cleared but variable still set
    - Wrong directory specified (file instead of directory)
  - **Check:** `exists=false` (doesn't exist) or `is_dir=false` (is a file)

---

### Scenario 2: "Trying config-provided model path"
**What happened:** No env var, but `model_path` was provided in `TranscriptionConfig`

**Next steps:**
- ✅ **"Vosk model found via config path - SUCCESS"**
  - **Cause:** Config path is valid
  - **Common in:** Users with custom installations, --model-path CLI flag
  
- ⚠️ **"Config-provided model path is invalid"**
  - **Cause:** Config value is wrong
  - **Common reasons:**
    - Wrong path in config.toml or CLI argument
    - Model was moved/deleted after config was written
    - Relative path interpreted from wrong working directory

---

### Scenario 3: "Starting auto-discovery for Vosk model"
**What happened:** No env var AND no config path provided (normal case)

**Sub-scenarios:**

#### 3a. "Vosk model discovery candidates found"
- **What happened:** Found one or more `vosk-model-*` directories
- **Where found:** In `models/` directory at one of these locations:
  - `$CWD/models/` (current working directory)
  - `$CWD/../models/` (parent)
  - `$CWD/../../models/` (grandparent)
  - `$CWD/../../../models/` (great-grandparent)
- **Common in:** Development environment, properly installed releases
- **Next:** Picks best candidate (prefers: small, en-us, highest version)

#### 3b. "No model candidates found during auto-discovery"
- **What happened:** Scanned all 4 locations, no `vosk-model-*` directories exist
- **Common reasons:**
  - Fresh clone/install, model never extracted
  - Running from unexpected directory (too deep in subdirectories)
  - Model directory named incorrectly (doesn't start with `vosk-model-`)
  - CI: Model download/extraction step didn't run
- **Next:** Proceeds to final error

#### 3c. "Found potential Vosk model directory"
- **What happened:** Found a specific candidate directory
- **Shows:** Full path and ancestor level (0=cwd, 1=parent, etc.)
- **Common in:** Normal operation

#### 3d. "Checking models directory - SCANNING"
- **Trace-level:** Shows each location being checked
- **Fields:**
  - `exists=true`: `models/` directory exists at this level
  - `is_dir=true`: It's actually a directory (not a file)
  - `ancestor_level=N`: How many levels up (0-3)

#### 3e. "Failed to read models directory"
- **What happened:** `models/` exists but can't be read
- **Common reasons:**
  - Permission denied (chmod issues)
  - Network filesystem timeout
  - Very rare: filesystem corruption

---

### Scenario 4: "Vosk model not found after exhaustive search - COMPLETE FAILURE"
**What happened:** All attempts failed - this is the END of the line

**Common causes by environment:**

**CI/Runners:**
- Model download script didn't run (workflow step missing/failed)
- Download succeeded but extraction failed (disk full, corrupt zip)
- Cache key mismatch (expected cache not restored)
- Wrong working directory (changed directory before running app)

**Local Development:**
- User forgot to download/extract model
- Running from wrong directory (e.g., `crates/app/` instead of project root)
- Model directory renamed or deleted

**Production:**
- Deployment didn't include models directory
- Filesystem permissions changed
- Disk full prevented extraction

**Check the error fields:**
- `checked_paths`: Lists every path that was tried
- `env_var_set=true`: Shows if VOSK_MODEL_PATH was set (even if invalid)
- `config_path_provided=true`: Shows if config had a path (even if invalid)
- `cwd`: Current working directory (may be unexpected)

---

## After Model Found: Transcriber Creation

### "Creating Vosk transcriber - NEXT"
**What happened:** Model directory was found, now attempting to load it

**Next steps:**

#### ✅ "VoskTranscriber created successfully"
- **Cause:** Model loaded, recognizer created
- **Means:** Everything is working correctly

#### ❌ "Failed to create Vosk transcriber - REASON: Model corrupted or incompatible"
**What happened:** Directory exists but Vosk library couldn't load files

**Common causes:**
- **Corrupted model:** Incomplete download, extraction failed partway
- **Missing files:** Required files like `am/`, `graph/`, `conf/` missing or empty
- **Version mismatch:** Model format incompatible with Vosk library version
- **Wrong model type:** Tried to load incompatible model (wrong language engine)

**The error shows:**
- `model_path`: Path that failed to load
- `directory_contents`: First 10 files in the directory (helps diagnose what's missing)
- `exists/is_dir`: Confirms the path is actually a directory

---

## Quick Diagnostic Checklist

When you see a Vosk model error in CI, check logs for:

1. **Was VOSK_MODEL_PATH set?**
   - Look for: `"Trying VOSK_MODEL_PATH environment variable"`
   - If yes, check `exists=` and `is_dir=` fields

2. **What was the working directory?**
   - Look for: `cwd=` field in error message
   - Compare to where you expect models to be

3. **Were any candidates found?**
   - Look for: `"Vosk model discovery candidates found"`
   - If yes, but still failed, model files are probably corrupted

4. **What paths were checked?**
   - Look at: `checked_paths=` in final error
   - Verify these are the paths you expected

5. **Can the models directory be read?**
   - Look for: `"Failed to read models directory"`
   - If present, it's a permission issue

6. **Did extraction run?**
   - Look for: `auto_extract_enabled=true` in logs
   - Look for: `"Attempting to extract model from zip"`
   - If no zip found, download step probably failed

---

## Example Log Patterns

### Pattern 1: Fresh Install (no model)
```
DEBUG Starting auto-discovery for Vosk model
DEBUG No model candidates found during auto-discovery
ERROR Vosk model not found after exhaustive search checked_paths="auto_discovery=/path/to/models"
```
**Diagnosis:** Model was never installed. User needs to download/extract.

---

### Pattern 2: Wrong Working Directory
```
DEBUG Starting auto-discovery cwd="/path/to/crates/app"
DEBUG Checking models directory search_path="/path/to/crates/app/models" exists=false
DEBUG Checking models directory search_path="/path/to/crates/models" exists=false
DEBUG Checking models directory search_path="/path/to/models" exists=false
DEBUG Checking models directory search_path="/path/models" exists=true
DEBUG Found potential Vosk model directory candidate="/path/models/vosk-model-small-en-us-0.15"
INFO Vosk model found via auto-discovery - SUCCESS
```
**Diagnosis:** Running from deep subdirectory, but auto-discovery still found it (good design!).

---

### Pattern 3: Env Var Set But Invalid
```
DEBUG Trying VOSK_MODEL_PATH environment variable env_path="/nonexistent"
WARN VOSK_MODEL_PATH points to invalid location env_path="/nonexistent" exists=false is_dir=false
ERROR Failed to locate or prepare Vosk model env_var_set=true
```
**Diagnosis:** CI set wrong path. Check CI configuration for typos.

---

### Pattern 4: Model Directory Empty
```
INFO Vosk model found via auto-discovery path="/path/to/models/vosk-model-small-en-us-0.15"
DEBUG Creating Vosk transcriber
ERROR Vosk library failed to load model directory_contents="" path_exists=true is_directory=true
```
**Diagnosis:** Directory exists but is empty. Extraction failed or files were deleted.

---

### Pattern 5: Corrupt Model
```
INFO Vosk model found via auto-discovery
ERROR Failed to load Vosk model from: /path/models/vosk-model-small-en-us-0.15 
      exists=true, is_dir=true, contents=README, conf/, incomplete_file
```
**Diagnosis:** Partial extraction or corrupted download. Re-extract model.

---

## Related Files

- `crates/coldvox-stt-vosk/src/model.rs` - Model discovery logic
- `crates/coldvox-stt-vosk/src/plugin.rs` - Plugin initialization
- `crates/coldvox-stt-vosk/src/vosk_transcriber.rs` - Model loading
- `crates/app/src/stt/plugin_manager.rs` - Plugin selection and error handling
