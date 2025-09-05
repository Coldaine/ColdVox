# AT-SPI Application Identification Enhancement

## Issue Type
Enhancement

## Priority
Medium

## Component
`crates/coldvox-text-injection`

## Description
The current AT-SPI text injection implementation uses placeholder logic for application identification. This needs to be replaced with real AT-SPI API calls to detect the currently focused application and select appropriate injection strategies based on the detected app.

## Current State
- Placeholder implementation returns "unknown" for all apps (manager.rs:272)
- App-specific strategy selection is stubbed out (manager.rs:476)
- No actual AT-SPI focus tracking implemented

## TODOs in Code
- `manager.rs:272`: "TODO: Implement real AT-SPI app identification once API is stable"
- `manager.rs:476`: "TODO: This should use actual app_id from get_current_app_id()"

## Proposed Solution
1. Implement `get_current_app_id()` using atspi crate:
   - Query currently focused window
   - Extract application name/class
   - Return standardized app identifier

2. Create app detection logic:
   - Terminal emulators (gnome-terminal, konsole, alacritty, etc.)
   - Text editors (vscode, vim, emacs, etc.)
   - Web browsers (firefox, chrome, etc.)
   - Chat applications (slack, discord, etc.)

3. Map apps to optimal injection strategies:
   - Terminal apps → prefer paste operations
   - Editors → check for vim mode, use appropriate method
   - Browsers → clipboard injection with fallback
   - Default → combo strategy with multiple fallbacks

## Technical Requirements
- Use atspi crate's accessibility API
- Cache app identification results (with TTL)
- Handle cases where AT-SPI is unavailable
- Provide fallback to current "unknown" behavior

## Testing Requirements
- Unit tests for app identification logic
- Integration tests with mock AT-SPI responses
- Manual testing across different desktop environments
- Performance benchmarks for app detection overhead

## Related Issues
- Platform-specific injection backend testing
- Regex caching optimizations (shares performance concerns)

## Acceptance Criteria
- [ ] Real AT-SPI app identification implemented
- [ ] App-specific strategy selection working
- [ ] Fallback behavior maintained
- [ ] Performance impact < 5ms per injection
- [ ] Documentation updated with supported apps
