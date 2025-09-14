---
mode: agent
model: GPT-5 mini
tools: [codebase, search, fs, terminal, apply_patch, create_file, create_directory, insert_edit_into_file]
description: "Universal repo inventory (no params): detect stack, traverse source, and write to docs/reference/fileInventory.md with public entry points & CLIs"
---

# Agent System Prompt — Universal File Inventory (No-Params)

## Constants (no inputs)
- **REPORT_PATH:** `docs/reference/fileInventory.md`  <!-- exact target path -->
- **IGNORE_GLOBS:**
  - `target/**, node_modules/**, .git/**, dist/**, build/**, out/**, .next/**, .nuxt/**, .turbo/**,
    .venv/**, venv/**, __pycache__/**, .pytest_cache/**, .tox/**, .mypy_cache/**,
    .idea/**, .vscode/**, vendor/**, third_party/**, coverage/**`
- **MAX_FILES:** `2000`

## Bootstrap (always do this first)
1. Resolve absolute path `<repo_root>/docs/reference/fileInventory.md`.
2. **Create parent folders if missing** (mkdir -p behavior).
3. **Open or create the report file immediately:**
   - If missing, create with:
     ```
     # File Inventory
     Updated by <MODEL_NAME> at <ISO8601_UTC_TIMESTAMP>

     This document lists publicly usable entry points (APIs & CLIs) discovered in the repository.
     ```
   - If present, **ensure the first two lines are exactly**:
     - Line 1: `# File Inventory`
     - Line 2: `Updated by <MODEL_NAME> at <ISO8601_UTC_TIMESTAMP>`
       - On every run, **replace** the entire “Updated by …” line with the current model name and current UTC timestamp (RFC 3339 / ISO-8601, e.g., `2025-09-02T18:45:12Z`).

> The “Updated by … at …” header is mandatory and must be refreshed on each run.

## Scope & traversal
1. Detect tech stack from extensions and manifests; handle monorepos/workspaces (Yarn/PNPM/Nx, Rust workspaces, Go modules, .NET solutions).
2. Traverse the repo root (respecting **IGNORE_GLOBS**), visiting at most **MAX_FILES** in **case-insensitive alphabetical order by relative path**.
3. Analyze these file families:
   - **Manifests:** `package.json`, `pyproject.toml`, `setup.cfg`, `setup.py`, `Cargo.toml`, `go.mod`, `pom.xml`, `build.gradle*`, `*.csproj`, `*.sln`, `Gemfile`, `*.gemspec`, `composer.json`
   - **Code:** `*.ts, *.tsx, *.js, *.mjs, *.cjs`, `*.py`, `*.rs`, `*.go`, `*.java`, `*.kt`, `*.cs`, `*.sh`, public headers `*.h, *.hpp` (and `*.c, *.cpp` only when they clearly define public APIs)
   - **Docs:** `*.md` **only** if they declare CLI/config/public API surfaces

## Public API capture rules (externally usable only)
- **TypeScript/JavaScript:** `export`/`export default`, named exports, `module.exports`, CommonJS `exports.*`; note conditional exports in `package.json#exports`.
- **Python:** top-level `def`/`class` not starting with `_`; respect `__all__`; console entry points from `pyproject.toml [project.scripts]`, `[tool.poetry.scripts]`, or `setup.cfg [options.entry_points.console_scripts]`.
- **Rust:** `pub fn/struct/enum/trait/mod/type`; public impl methods; record `#[cfg(feature="...")]`.
- **Go:** exported (Capitalized) identifiers; packages intended for import; mark binaries where `package main` + `func main()`; record `//go:build` tags.
- **Java/Kotlin:** `public` classes/interfaces/enums and `public` methods; CLI via `public static void main(String[] args)` or picocli annotations.
- **.NET (C#):** `public` classes/structs/interfaces/enums and `public` methods; CLI via `static void Main`/`static Task Main`; note `.csproj` `OutputType`.
- **Shell:** executable scripts (shebang), flag parsing (`getopts`/`argparse`-style).
- **C/C++ (headers):** functions/types/macros in public headers (prefer `include/` or installed headers).

## Binary / CLI detection (summarize args/flags if evident)
- **Node:** `package.json#bin`, shebang + common CLI libs (`yargs`, `commander`, `oclif`).
- **Python:** `pyproject/setup.cfg` scripts, `if __name__ == "__main__":`, `argparse`/`click`.
- **Rust:** `src/bin/*.rs`, `[[bin]]` in `Cargo.toml` (e.g., `clap`).
- **Go:** `package main` using `cobra`/`urfave/cli`.
- **Java:** `main` + picocli annotations.
- **.NET:** `System.CommandLine` patterns.

## Manifest extraction (public-facing only)
- **package.json:** `name`, `type`, `exports`, `bin`, `workspaces`, public `scripts`.
- **pyproject.toml / setup.cfg:** project name; entry points under scripts.
- **Cargo.toml:** crate `name`; `lib`/`bin` targets; `[[example]]`; `features`.
- **go.mod:** module path (note `replace` only as context).
- **pom.xml / build.gradle*:** group/artifact; application plugin/mainClass if present.
- **.csproj:** `OutputType`; target frameworks; `PackageId` if library.

## Report format (write/update in `docs/reference/fileInventory.md`)
Ensure the file begins with:

File Inventory

Updated by <MODEL_NAME> at <ISO8601_UTC_TIMESTAMP>


For each file that exposes a public surface, write **one** section keyed by its **relative path**, keeping alphabetical order by section key.

### Section heading

path/to/file.ext

### Section body (strict template; omit truly N/A keys, keep this key order)
- **Role:** _library_ | _binary/CLI_ | _module_ | _header_ | _script_
- **Language/Stack:** e.g., TypeScript (Node ESM), Python, Rust, Go, Java, C#, Shell, C/C++
- **Public API:**
  - `name(signature)` — one bullet per export/public symbol (group logically if numerous)
- **CLI Entrypoints:**
  - `command` — short synopsis (flags/args if evident)
- **Manifest Links:**
  - references like `package.json#exports`, `pyproject [project.scripts]`, `Cargo.toml [[bin]]`
- **Notes:** build tags, features, cfgs, deprecations

## Deterministic, idempotent update algorithm
1. **Header:** Ensure line 1 is `# File Inventory`. **Replace** line 2 with `Updated by <MODEL_NAME> at <ISO8601_UTC_TIMESTAMP>` every run.
2. Compute the complete desired set of sections (all qualifying files), sorted **case-insensitively by relative path**.
3. For each section:
   - If a heading `## <relative-path>` exists, **replace its body** up to the next `## ` or EOF using the template above.
   - If absent, **insert** a new section at the correct sorted position (do not duplicate).
4. Remove sections for files that no longer expose a public surface.
5. Write changes atomically to `docs/reference/fileInventory.md`. Parent directories must already exist (created during Bootstrap).

## Output requirements
- Respect **IGNORE_GLOBS** and **MAX_FILES**.
- Only capture public/CLI surfaces; exclude private/internal items.
- Keep formatting exactly as specified (headings, key order, bold labels).
- Re-running with no code changes must produce **no diff**.

## Self-check before finishing
- [ ] Created/read `docs/reference/fileInventory.md` **first**; parent dirs ensured.
- [ ] Header updated with correct `MODEL_NAME` and current UTC timestamp.
- [ ] Sections sorted; no duplicate headings.
- [ ] Only public/CLI surfaces captured.
- [ ] Idempotent (no diff on repeat run).

---

## Post-pass: High-Level Grouping Proposal & Assignment

**Goal:** After writing/updating all file sections, propose a concise set of top-level **Groups** and assign each documented file section to exactly one group (or **Unassigned** if no clear fit). Then append (or replace) a single grouped summary block at the end of the report.

### Group proposal
- Infer groups from directory names, filenames, imports/dependencies, frameworks, and each section’s **Role**/**Language/Stack**.
- Prefer ≤ **12** Title-Case group names; avoid overlapping meanings.
- If the repository resembles a speech/voice/STT app, consider these canonical examples (only if they truly apply):
  - **Interface/GUI**
  - **Audio Capture**
  - **VAD** (Voice Activity Detection)
  - **STT Engine**
  - **Text Injection**
- Optional additional buckets (only if they cover multiple files): **CLI/Binaries**, **Config**, **Networking**, **Data Models**, **Persistence/Storage**, **Utilities**, **Build/CI**, **Docs**.

### Heuristics (signals; use judgment)
- **Interface/GUI:** `ui/`, `gui/`, `view/`, `electron`, `qt`, `react`, `flutter`, `wpf`, `swiftui`.
- **Audio Capture:** `audio`, `mic`, `microphone`, `portaudio`, `pyaudio`, `sounddevice`, `coreaudio`, `avfoundation`, WebRTC audio.
- **VAD:** `vad`, `webrtcvad`, `silero`, `rnnoise`, “silence/energy threshold”.
- **STT Engine:** `stt`, `asr`, `whisper`, `whisper.cpp`, `vosk`, `deepspeech`, `kaldi`, `nemo`, cloud STT SDKs.
- **Text Injection:** `keyboard`, `keystroke`, `SendInput`, `xdotool`, `AutoHotkey`, `robotjs`, accessibility APIs.

### Assignment pass
- For **every** `## path/to/file.ext` section in this report, compute a best-fit group.
- Choose the **most specific** applicable group; assign **exactly one** group per file.
- If no reasonable fit, assign **Unassigned**.
- Do **not** duplicate a file across multiple groups.

### Respect existing groupings
- If a **Groupings** block already exists (see markers below), treat its **group names** as the authoritative set for the next run. Reuse names, maintain their order, and only add new groups if necessary.
- Preserve ordering: existing groups (original order) → any new groups (case-insensitive alphabetical) → **Unassigned** last.

### Output block (append/replace, with stable markers)
Write (or fully replace) a single block at the **end** of `docs/reference/fileInventory.md`, delimited exactly as below:
