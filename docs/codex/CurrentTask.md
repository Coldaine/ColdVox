# Text Injection Workflow and `kdotool` Integration Plan

This document outlines the current understanding of the ColdVox text injection system and the plan for integrating `kdotool` to enhance its window awareness, particularly within KDE Plasma environments.

## Current Text Injection Workflow

The text injection system, managed by the `StrategyManager`, employs a prioritized, multi-stage fallback mechanism to deliver text to the active application. My analysis of the `manager.rs` and related injector modules confirms the following injection pipeline:

1.  **AT-SPI Direct Injection (`AtspiInsert`)**: This is the primary and most preferred method. It uses the Accessibility Technology Service Provider Interface (AT-SPI) to directly insert text into a focused, editable field. This method is fast, reliable, and avoids interfering with the system clipboard.

2.  **AT-SPI with Paste (`ClipboardPasteFallback`)**: When direct insertion via AT-SPI is not possible or fails, the system falls back to a clipboard-based paste operation that is still mediated by AT-SPI. The workflow is designed for safety and transparency:
    *   The current content of the clipboard is backed up.
    *   The text to be injected is placed into the clipboard.
    *   An AT-SPI command is used to trigger a "paste" action in the target application.
    *   Crucially, the original clipboard content is restored immediately after the paste action.
    This ensures that the user's clipboard is not permanently overwritten, a key requirement.

3.  **Command-Line Tool Fallback (`kdotool`, `ydotool`, `enigo`)**: If both AT-SPI methods fail, the system resorts to lower-level command-line utilities as a final fallback. The user specifically mentioned `kdotool` and `ydotool`. The current implementation in `manager.rs` includes `KdoToolAssist` and `EnigoText` as configurable fallbacks before the final `ClipboardPasteFallback`. `ydotool` is not directly in the injection chain but is mentioned in documentation as a potential tool used by other injectors.

## Plan for `kdotool` Integration and Scoped Testing

The current task is to enhance the system's "window awareness" by leveraging `kdotool` and to add comprehensive, behaviorally-scoped tests to validate its functionality. The integration will focus on using `kdotool` to reliably identify the active window, which is a prerequisite for selecting the correct injection strategy.

### 1. Enhance `kdotool` Injector for Window Details
I will add a new function, `get_active_window_details()`, to the `KdotoolInjector`. This function will be responsible for calling the `kdotool` command-line tool to retrieve not just the active window ID, but also its PID and, most importantly, its class name (`WM_CLASS`). This consolidates all `kdotool` interactions for window information into a single, testable module.

### 2. Integrate `kdotool` into the `StrategyManager`
I will modify the `get_current_app_id()` function within the `StrategyManager`. The new logic will first check if the current desktop environment is KDE Plasma. If it is, it will use the newly created `kdotool_injector.get_active_window_details()` to get the window's class name, which is a more reliable identifier (`app_id`) than generic `xprop` queries in this environment. The existing `xprop` and `swaymsg` logic will be preserved as fallbacks for X11 and Sway/wlroots environments, respectively.

### 3. Implement Comprehensive Behavioral Testing
I will create a new test suite specifically for `kdotool` integration. The tests will be scoped to verify each piece of functionality:
*   **Unit/Mock Tests**: I will write tests for the `get_active_window_details()` function that mock the standard output of the `kdotool` command. This will allow for validating the parsing logic for window ID, PID, and class name without requiring a live desktop environment, making the tests suitable for CI.
*   **Integration Tests**: I will add tests that execute actual `kdotool` commands (`getactivewindow`, `windowfocus`, `windowactivate`). These tests will be designed to run in a live KDE environment to confirm that the tool behaves as expected. They will be marked appropriately to be skipped in headless CI environments where a display server is not available.

By implementing these changes, the text injection system will be more robust and reliable, especially for users on KDE Plasma, and the new test suite will ensure that `kdotool` functionality remains stable and correct.