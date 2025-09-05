# Regex Caching for Text Injection Performance

## Issue Type
Performance Optimization

## Priority
Low-Medium

## Component
`crates/coldvox-text-injection`

## Description
The StrategyManager currently compiles regex patterns on every use for app filtering. This should be optimized by caching compiled regex objects to avoid repeated compilation overhead.

## Current State
- Regex patterns compiled fresh each time in filtering logic
- No caching mechanism for compiled patterns
- TODO comment at manager.rs:335 indicates known optimization opportunity

## TODOs in Code
- `manager.rs:335`: "TODO: Store compiled regexes in the manager state for performance"

## Performance Impact
- Current: ~0.5-2ms per regex compilation
- With multiple patterns: Can add 5-10ms to injection latency
- Expected improvement: <0.1ms for cached pattern matching

## Proposed Solution
1. Add regex cache to StrategyManager state:
   ```rust
   struct StrategyManager {
       strategies: Vec<InjectionStrategy>,
       regex_cache: HashMap<String, Regex>,
       // existing fields...
   }
   ```

2. Implement cache management:
   - Lazy compilation on first use
   - LRU eviction for memory management
   - Thread-safe access if needed

3. Update filter methods:
   - Check cache before compilation
   - Store compiled patterns in cache
   - Handle invalid regex patterns gracefully

## Implementation Details
```rust
fn get_or_compile_regex(&mut self, pattern: &str) -> Result<&Regex> {
    match self.regex_cache.entry(pattern.to_string()) {
        Entry::Occupied(e) => Ok(e.into_mut()),
        Entry::Vacant(e) => {
            let regex = Regex::new(pattern)?;
            Ok(e.insert(regex))
        }
    }
}
```

## Testing Requirements
- Benchmark before/after performance
- Unit tests for cache behavior
- Test cache eviction with many patterns
- Verify thread safety if concurrent access

## Considerations
- Memory usage vs performance tradeoff
- Cache invalidation strategy
- Maximum cache size limits
- Pattern validation and error handling

## Related Issues
- AT-SPI app identification (shares manager.rs modifications)
- Platform-specific backend testing (performance validation)

## Acceptance Criteria
- [ ] Regex cache implemented in StrategyManager
- [ ] Performance improvement measurable (>50% reduction)
- [ ] Memory usage reasonable (<1MB for typical usage)
- [ ] No regression in functionality
- [ ] Cache metrics exposed for monitoring
