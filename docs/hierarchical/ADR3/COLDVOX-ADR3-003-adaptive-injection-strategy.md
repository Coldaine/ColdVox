---
id: COLDVOX-ADR3-003-adaptive-injection-strategy
type: ADR
level: 3
title: Adaptive Text Injection Strategy
status: accepted
owner: @team-injection
updated: 2025-09-11
parent: COLDVOX-DOM2-005-text-injection
links:
  satisfies: [COLDVOX-DOM2-005-text-injection]
  depends_on: []
  supersedes: []
  related_to: [COLDVOX-SYS4-006-injection-manager]
---

## Context
Text injection reliability varies significantly across applications, desktop environments, and user configurations. A static approach to injection backends results in suboptimal success rates.

## Decision
Implement an adaptive strategy manager that learns from past injection attempts to optimize backend selection per application and environment.

## Status
Accepted

## Consequences
### Positive
- Improves overall injection success rates by learning from past attempts
- Reduces manual configuration burden on users
- Adapts to changing environment conditions automatically
- Enables data-driven optimization of injection strategies

### Negative
- Adds complexity to the injection system
- Requires storing and analyzing injection attempt data
- Initial learning period may have lower success rates
- Increased memory usage for tracking success metrics

## Implementation
- StrategyManager tracks success rates per application/backend combination
- Uses exponential moving average for responsive learning
- Implements fallback chains when primary backends fail
- Provides cache warming for faster application-specific strategy selection

## Related Documents
- `crates/coldvox-text-injection/src/manager.rs`
- `COLDVOX-SYS4-006-injection-manager.md`

---
satisfies: COLDVOX-DOM2-005-text-injection  
depends_on:  
supersedes:  
related_to: COLDVOX-SYS4-006-injection-manager