# ColdVox vNext: Quick Start Guide

> **New to this project?** Start here for a rapid overview of the vNext text injection initiative.

---

## 🎯 What is vNext?

**vNext** is a comprehensive redesign of ColdVox's text injection system to support multiple desktop environments with <200ms latency and 85%+ success rates across all applications.

### The Problem
Current injection methods are fragile, slow, and platform-specific. Users experience failures, delays, and poor error messages.

### The Solution
Multi-method injection with intelligent fallbacks, event-based confirmation, and deep observability.

---

## 📚 Documentation Structure

```
docs/plans/
├── README.md          ← Navigation guide (start here for exploration)
├── QUICK_START.md     ← This file (start here for quick overview)
├── MASTER_PLAN.md     ← Complete synthesis (read for full context)
└── [Source docs]      ← Detailed designs (reference as needed)
```

### Reading Path by Goal

**I want to understand the big picture:**
1. Read this Quick Start
2. Read [MASTER_PLAN.md - Executive Summary](MASTER_PLAN.md#executive-summary)
3. Skim [Section 2: Architecture](MASTER_PLAN.md#2-text-injection-architecture)

**I'm implementing injection methods:**
1. Read [MASTER_PLAN.md - Section 2](MASTER_PLAN.md#2-text-injection-architecture)
2. Deep dive: [OpusCodeInject.md](OpusCodeInject.md)
3. Reference: Implementation Roadmap Phases 1-4

**I'm writing tests:**
1. Read [MASTER_PLAN.md - Section 3](MASTER_PLAN.md#3-testing-strategy)
2. Deep dive: [InjectionTest1008.md](InjectionTest1008.md)
3. Reference: [OpusTestInject2.md](OpusTestInject2.md) for hardware tests

**I'm managing the project:**
1. Read [MASTER_PLAN.md - Section 5](MASTER_PLAN.md#5-implementation-roadmap)
2. Review [Section 7: Risk Management](MASTER_PLAN.md#7-risk-management)
3. Track against [Section 6: Success Criteria](MASTER_PLAN.md#6-success-criteria)

---

## 🏗️ Architecture at a Glance

### Three Core Methods

1. **AT-SPI Insert/Paste** - Direct accessibility API (fastest, most reliable)
2. **Virtual Keyboard** - Protocol-level input synthesis (Wayland/wlroots)
3. **Portal/EIS** - Authorized remote desktop input (secure, needs consent)

### Fallback Chain Example (Hyprland)

```
Speech detected
    ↓
Try AT-SPI Insert (30ms)
    ↓ [fails - no AT-SPI support]
Try AT-SPI Paste (40ms)
    ↓ [fails - no caret]
Try Virtual Keyboard (50ms)
    ↓ [success!]
Text appears in app (confirmed via events)
    ↓
Total: 120ms ✓
```

### Key Innovation: Pre-Warming

Instead of discovering capabilities at injection time, we **pre-warm** when the user starts speaking:

```
User starts speaking
    ↓
[Pre-warm phase - 50ms]
├── Ping AT-SPI bus
├── Snapshot focus
├── Backup clipboard
└── Connect virtual keyboard
    ↓
User finishes speaking
    ↓
[Injection phase - <100ms]
└── Use pre-warmed connection
    ↓
Text appears fast ✓
```

---

## 📊 Success Metrics (at a Glance)

| Metric | Target | Current | Status |
|--------|--------|---------|--------|
| AT-SPI apps (Kate, Firefox) | ≥95% | TBD | 🔄 In Progress |
| Non-AT-SPI apps (terminals) | ≥80% | TBD | 🔄 In Progress |
| End-to-end latency (p95) | <200ms | TBD | 🔄 In Progress |
| Pre-warm overhead | <50ms | TBD | 🔄 In Progress |
| Test suite time (pre-commit) | <3s | TBD | 🔄 In Progress |

---

## 🗓️ Timeline (12 Weeks)

```
Phase 1: Foundation          [Weeks 1-2]   ████░░░░░░░░░
  └── AT-SPI refinement, telemetry, test framework

Phase 2: Virtual Keyboard    [Weeks 3-4]   ░░░░████░░░░░
  └── Wayland protocol implementation

Phase 3: Portal/EIS          [Weeks 5-6]   ░░░░░░░░████░░
  └── xdg-desktop-portal integration

Phase 4: KWin Fake Input     [Weeks 7-8]   ░░░░░░░░░░████
  └── KDE-specific privileged input

Phase 5: Integration         [Weeks 9-10]  ░░░░░░░░░░░░██
  └── E2E testing, hardware validation

Phase 6: Polish & Release    [Weeks 11-12] ░░░░░░░░░░░░░░
  └── Performance tuning, documentation
```

**Current Phase:** Planning ✓ → Phase 1 (Next)

---

## 🧪 Testing Philosophy

### Three-Tier Strategy

**Tier 1: Fast (Pre-Commit)**
- <3s execution
- Deterministic logic tests
- Blocks commits on failure
- Uses behavioral fakes

**Tier 2: Hardware (Continuous)**
- Non-blocking CI runs
- Real devices and compositors
- Reports to telemetry
- Alerts on critical failures

**Tier 3: Release Gate**
- Full platform matrix
- Must pass before release
- Includes performance benchmarks
- Real users, real apps

### Test Pyramid

```
        /\
       /E2\        10% - Real apps, real audio
      /____\
     /      \
    / Integ  \     20% - Mock protocols, real timing
   /__________\
  /            \
 /    Unit      \  70% - Logic, algorithms, fast fakes
/________________\
```

---

## 🚀 Getting Started (Developer)

### Prerequisites
```bash
# Nobara Linux (or similar)
sudo dnf install at-spi2-core xdg-desktop-portal wayland-devel

# Add user to input group (for fallbacks)
sudo usermod -aG input $USER

# Install Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### Build and Test
```bash
# Clone and build
git clone https://github.com/Coldaine/ColdVox.git
cd ColdVox

# Run fast tests
make test-fast

# Run with text injection enabled
cargo run --features text-injection

# Run TUI dashboard
cargo run --bin tui_dashboard
```

### Pre-Commit Setup
```bash
# Install git hooks
make install-hooks

# Now every commit runs fast tests (<3s)
git commit -m "Your changes"
```

---

## 📖 Key Concepts

### Event-Based Confirmation
Don't sleep and hope text appeared. Listen for platform events:
- AT-SPI: `object:text-changed:inserted`
- Windows UIA: `TextChanged` event
- Timeout: 75ms max

### Clipboard Hygiene
Never leave user's clipboard polluted:
1. Backup current clipboard
2. Set text for injection
3. Perform paste action
4. Restore original clipboard
5. Clear from clipboard manager history (optional)

### Graceful Degradation
Every method can fail. Chain them intelligently:
```
Primary method fails
    ↓
Log structured diagnostic
    ↓
Try next method
    ↓
All methods exhausted?
    ↓
Return actionable error message
```

### Observable Behavior
Every injection attempt generates telemetry:
- Which method tried
- How long each stage took
- Success/failure with reason
- Privacy-safe (no text content logged)

---

## 🔗 Quick Links

### Essential Reading
- [Master Plan](MASTER_PLAN.md) - Complete reference
- [Architecture Section](MASTER_PLAN.md#2-text-injection-architecture)
- [Testing Strategy](MASTER_PLAN.md#3-testing-strategy)

### Implementation Details
- [OpusCodeInject.md](OpusCodeInject.md) - Complete code examples
- [InjectionMaster.md](InjectionMaster.md) - Design rationale

### Testing Details
- [InjectionTest1008.md](InjectionTest1008.md) - Test philosophy
- [OpusTestInject2.md](OpusTestInject2.md) - Hardware framework
- [QwenTestMerge.md](QwenTestMerge.md) - vNext test plan

### Project Resources
- [GitHub Repository](https://github.com/Coldaine/ColdVox)
- [Issue Tracker](https://github.com/Coldaine/ColdVox/issues)
- [CLAUDE.md](../../CLAUDE.md) - Workspace overview

---

## ❓ FAQ

**Q: Why so many injection methods?**  
A: Different desktop environments and applications support different APIs. Having multiple methods with intelligent fallbacks ensures wide compatibility.

**Q: Won't this be slow with all the fallbacks?**  
A: Pre-warming eliminates discovery overhead. Each method has a 50ms timeout, so worst case is 200ms even with 4 methods.

**Q: What about privacy?**  
A: Text content is never logged. Telemetry captures length, timing, and success/failure but not actual words.

**Q: Which platforms are supported?**  
A: KDE Plasma (Wayland), Hyprland, and Windows. X11 support via existing methods. Other platforms can be added following the same pattern.

**Q: How do I contribute?**  
A: Check the [Implementation Roadmap](MASTER_PLAN.md#5-implementation-roadmap) for current phase. Pick an unclaimed deliverable and open a PR.

---

## 📞 Support

**Questions?** Open an issue with the `planning` or `question` label.

**Found a bug?** Open an issue with the `bug` label and reproduction steps.

**Want to contribute?** See [MASTER_PLAN.md - Section 5](MASTER_PLAN.md#5-implementation-roadmap) for current priorities.

---

**Last Updated:** 2025-10-08  
**Status:** Planning Phase Complete → Phase 1 Starting Soon
