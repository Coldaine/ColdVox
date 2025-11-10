---
doc_type: playbook
subsystem: testing
version: 1.0.0
status: draft
owners: AI Strategy Team
last_reviewed: 2025-11-10
---

# Playbook: Debugging Test Failures with LLMs

This playbook provides a structured approach for developers and AI assistants to collaboratively debug test failures using Large Language Models (LLMs). It aligns with our philosophy of using comprehensive, real-world tests by equipping the team with tools to rapidly diagnose failures in complex integration scenarios.

The goal is not to have the LLM solve the problem autonomously, but to use it as an intelligent assistant to accelerate the human's debugging workflow.

## Guiding Principles

1.  **Context is King**: The quality of the LLM's assistance is directly proportional to the quality of the context provided. Garbage in, garbage out.
2.  **LLM as a Diagnostician, Not a Fixer**: Use the LLM to generate hypotheses, identify potential root causes, and suggest diagnostic steps. The developer's role is to verify these hypotheses and implement the fix.
3.  **Iterative Refinement**: Start with a high-level overview of the failure and progressively add more context as you narrow down the problem.
4.  **Trust but Verify**: Never blindly apply LLM-suggested code changes. Always understand the reasoning behind the suggestion and validate it against the project's standards and your own expertise.

## The 4-Step LLM Debugging Workflow

### Step 1: Initial Triage (High-Level Context)

The first step is to get a quick, high-level assessment of the failure. Provide the LLM with the essential "what, where, and when" of the problem.

**Prompt Template: Initial Triage**

```
I am debugging a test failure in the ColdVox project. Here is the initial context:

**1. Test Failure Log:**
```
<Paste the complete, unfiltered test failure output here. Include the test name, failure message, and any stack traces.>
```

**2. Test Source Code:**
Filepath: `<path/to/the/failing/test_file.rs>`
```rust
<Paste the full source code of the failing test function and any relevant setup functions.>
```

**3. Code Under Test:**
Filepath: `<path/to/the/source_file_being_tested.rs>`
```rust
<Paste the primary function or module being exercised by the test.>
```

**Your Task:**
Based on this initial information, please provide:
1.  A brief, one-sentence summary of the failure.
2.  A list of 3-5 potential root causes, ordered from most to least likely.
3.  A list of immediate next steps I should take to investigate the most likely cause. What specific information should I gather?
```

### Step 2: Contextual Deep Dive (Narrowing the Scope)

Based on the initial triage, you likely have a primary hypothesis. Now, provide the LLM with more specific, targeted context to explore that hypothesis.

**Prompt Template: Contextual Deep Dive**

```
Following up on the previous analysis of the test failure in `test_some_functionality`. My primary hypothesis is that the failure is related to `<Your Hypothesis, e.g., "a race condition in the audio chunker">`.

Here is additional context to help investigate this:

**1. Relevant Logs (with increased verbosity if possible):**
```
<Paste new, more detailed logs. For example, logs with `RUST_LOG=debug` enabled for the relevant modules.>
```

**2. Related Modules/Functions:**
- **Module A:** `<path/to/related_module_A.rs>`
  ```rust
  <Paste code for a function that interacts with the code under test.>
  ```
- **Module B:** `<path/to/related_module_B.rs>`
  ```rust
  <Paste code for another relevant data structure or utility.>
  ```

**3. Key Configuration:**
Filepath: `config/default.toml` (or relevant test config)
```toml
<Paste any configuration values that might influence the behavior of the components involved.>
```

**Your Task:**
1.  Re-evaluate the potential root causes based on this new information. Does the initial hypothesis still hold?
2.  Suggest specific `log::debug!` or `println!` statements to insert into the code to prove or disprove the hypothesis. Provide the file path, line number, and the exact code to insert.
3.  Provide a small, self-contained code snippet that could be run to reproduce the suspected issue in isolation, if possible.
```

### Step 3: Root Cause Analysis (Pinpointing the Error)

By now, you should be very close to the root cause. This step is about confirming the exact line of code and logic error that is causing the failure.

**Prompt Template: Root Cause Analysis**

```
We have confirmed that the test failure is caused by `<The confirmed root cause, e.g., "an off-by-one error when calculating the ring buffer's readable region">`.

The key evidence was the output from the debug statements you suggested:
```
<Paste the specific log output that confirms the root cause.>
```

Here is the precise code block that contains the error:
Filepath: `<path/to/source_file.rs>`
```rust
// Lines 100-115
<Paste the 10-15 lines of code directly surrounding the bug.>
```

**Your Task:**
1.  Explain the logical error in the code block above in plain English.
2.  Provide the corrected code block.
3.  Explain *why* the correction fixes the bug.
4.  Describe any potential side effects or regressions that I should be aware of and suggest how to test for them.
```

### Step 4: Solution & Verification (Writing the Fix and New Tests)

The final step is to implement the fix and ensure it's covered by a new, specific test case.

**Prompt Template: Solution & Verification**

```
The fix you provided for `<the specific bug>` was successful and all existing tests now pass.

**Corrected Code (for your reference):**
Filepath: `<path/to/source_file.rs>`
```rust
<Paste the final, corrected code that is now committed.>
```

**Your Task:**
1.  Write a new unit or integration test that specifically targets the bug we just fixed. This test should have failed *before* the fix was applied.
2.  The new test should be placed in `<path/to/test_file.rs>`.
3.  Provide a descriptive name for the test, such as `test_ring_buffer_handles_edge_case_on_wrap_around`.
4.  Explain why this new test effectively prevents a regression of this specific bug.
```

## Best Practices

-   **Use Code Blocks with Syntax Highlighting**: Always format logs, code, and config snippets in Markdown code blocks (e.g., ` ```rust `) for readability.
-   **Be Specific with File Paths**: Providing accurate file paths helps the LLM build a mental model of the codebase.
-   **Anonymize Sensitive Data**: Before pasting logs or code, scrub any proprietary or sensitive information.
-   **Correct the LLM**: If the LLM misunderstands something or provides an incorrect suggestion, correct it. This helps refine its context for subsequent prompts.
-   **Separate Concerns**: Don't dump the entire codebase on the LLM. Focus each prompt on a specific aspect of the failure.
