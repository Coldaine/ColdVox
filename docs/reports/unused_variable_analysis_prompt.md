### **Prompt for AI Agent: Rust Codebase Unused Variable Analysis**

**Objective:**

Your primary goal is to perform a comprehensive review of the entire Rust codebase in this workspace to identify all "unused variable" warnings. For each warning, you must determine if the variable is truly **abandoned** (i.e., dead code that should be removed) or if it is **conditionally compiled** (i.e., used only when specific features or build configurations are active).

**Core Concepts:**

*   **Abandoned Variable:** A variable that is declared but never used in any compilation scenario. Its existence is likely a bug, a remnant of old refactoring, or incomplete code. It should be safely removed.
*   **Conditionally Compiled Variable:** A variable that appears unused in the default build configuration but is required when compiling with specific features (e.g., `#[cfg(feature = "my-feature")]`), for a specific operating system (e.g., `#[cfg(target_os = "windows")]`), or during tests (e.g., `#[cfg(test)]`). These are not bugs.

**Execution Plan:**

You are to follow this procedure step-by-step:

**1. Initial Code Scan:**
   - Execute `cargo clippy --workspace` to get a complete list of lints across all crates.
   - Parse the output and create a list of every "unused variable" warning.

**2. In-Depth Analysis per Variable:**
   For each unused variable warning you identified:
   a. **Locate the Code:** Identify the exact file, line number, and function where the variable is declared.
   b. **Analyze the Context:** Read the surrounding code to understand the variable's purpose.
   c. **Search for Conditional Compilation (`#[cfg]`)**: This is the most critical step.
      - Check if the **usage** of the variable occurs inside a block controlled by a `#[cfg(...)]` attribute.
      - Check if the variable's **declaration** itself is inside a `#[cfg]` block.
      - Check if the entire **enclosing function or module** is gated by a `#[cfg]` attribute.
   d. **Cross-Reference with `Cargo.toml`:**
      - If you find a feature flag (e.g., `#[cfg(feature = "some-feature")]`), open the relevant `Cargo.toml` file and examine the `[features]` section to understand what that feature enables. This confirms the conditional nature of the code.
   e. **Classify the Variable:**
      - **Classify as `Conditionally Compiled` if:** You find any `#[cfg]` attribute that explains why the variable is not used in the current compilation context.
      - **Classify as `Abandoned` if:** You find no evidence of conditional compilation related to the variable's use. It appears to be genuinely unused in all scenarios.

**3. Take Corrective Action:**
   Based on your classification:
   - For a **`Conditionally Compiled`** variable:
     1.  Apply the standard Rust idiom by prefixing the variable name with an underscore (e.g., `my_var` becomes `_my_var`).
     2.  Add a brief comment explaining *why* it's conditionally used, if the reason is not immediately obvious from the code. For example: `// Only used on Windows builds`.
   - For an **`Abandoned`** variable:
     1.  Safely remove the variable declaration.
     2.  Ensure that removing it does not cause any new compilation errors. If it does, the variable was not truly abandoned; re-evaluate your analysis.

**4. Verification:**
   - After applying fixes for all identified variables, run `cargo clippy --workspace` one final time to confirm that all the original "unused variable" warnings have been resolved and no new errors have been introduced.

**5. Final Report:**
   Present your findings in a clear, structured report. For each variable you investigated, provide the following details:
   - **File & Line:** The location of the variable declaration.
   - **Variable Name:** The name of the variable.
   - **Classification:** `Abandoned` or `Conditionally Compiled`.
   - **Justification:** A brief explanation for your classification. (e.g., "Used only when `target_os = 'linux'`" or "No usage found in any build configuration.").
   - **Action Taken:** The specific action you took (e.g., "Prefixed with underscore" or "Removed variable declaration.").

Your ultimate goal is to improve code quality by removing dead code and correctly handling variables used in conditional builds, thereby making the codebase cleaner and easier to maintain.
