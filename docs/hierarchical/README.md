# Hierarchical Documentation Structure

This directory contains the hierarchical documentation for the ColdVox project, organized according to the atomic node structure with typed edges.

## Vision and Pillars

The ColdVox project is built around a core vision of providing a robust, real-time voice AI pipeline with four foundational pillars:

1. **[Real-time Audio Processing](PIL1/COLDVOX-PIL1-001-realtime-audio-processing.md)** - Efficient, low-latency audio capture and processing with automatic recovery
2. **[Voice Activity Detection](PIL1/COLDVOX-PIL1-002-voice-activity-detection.md)** - Accurate detection of speech segments to gate processing
3. **[Speech-to-Text Transcription](PIL1/COLDVOX-PIL1-003-speech-to-text.md)** - High-quality offline-capable speech transcription
4. **[Cross-platform Text Injection](PIL1/COLDVOX-PIL1-004-text-injection.md)** - Reliable text injection with adaptive strategies

## Structure Overview

The documentation follows a streamlined hierarchy for conceptual/design levels with relaxed constraints for implementation artifacts. The hierarchy is VSN → PIL (optional) → DOM → SYS → SPEC, eliminating the SUB level to reduce overhead.

- **VSN** - Vision (top-level vision document)
- **PIL** - Pillars (core architectural pillars, skippable if no added value)
- **DOM** - Domains (major architectural domains)
- **SYS** - Systems (specific systems within domains)
- **SPEC** - Specifications (detailed interface specifications)
- **IMP** - Implementations (code implementations)
- **TST** - Tests (test specifications and implementations)
- **ADR** - Architectural Decision Records
- **RSK** - Risks
- **QST** - Open Questions
- **DOCS** - General Documentation

## Consolidation Rules

To maintain value while reducing overhead:
- Merge content from eliminated levels (e.g., former SUB subdomains) into the nearest meaningful parent (e.g., DOM or SYS).
- Eliminate placeholder documents that only exist to fill hierarchy slots; integrate substantial content or remove if redundant.
- Focus on meaningful, substantial documentation rather than strict structural compliance.
- PIL can be skipped when a domain directly stems from vision without needing a pillar intermediary.

## Node Types

Each document follows a standardized format with frontmatter metadata:

- **id**: Stable, unique identifier following the pattern `<AREA>-<TYPE><LEVEL>-<NNN>[-<slug>]` (adjust levels as hierarchy simplifies, e.g., SYS now follows DOM directly)
- **type**: Node type (VSN, PIL, DOM, SYS, SPEC, IMP, TST, ADR, RSK, QST, DOCS)
- **level**: Hierarchy depth (adjusted for simplified structure: VSN=0, PIL=1, DOM=2, SYS=3, SPEC=4)
- **title**: Human-readable title
- **status**: Current status (Draft|Approved|Deprecated)
- **owner**: Responsible team or individual (optional)
- **parent**: Parent node ID (required for VSN-PIL-DOM-SYS-SPEC levels)
- **links**: Typed relationships to other nodes
- **updated**: Last update timestamp
- **last_reviewed**: Date of last review (optional, for Approved/Deprecated)

## Link Types

Documents maintain typed relationships using the following link types:

- **satisfies**: This node satisfies a requirement from another node
- **implements**: This node implements a specification
- **verified_by**: This node is verified by a test
- **related_to**: This node is related to another node (flexible for peer or cross-hierarchy links)

(Other types like depends_on or supersedes can be used as related_to if needed, but focus on core types to reduce ceremony.)

## Implementation Status

This hierarchical documentation is being actively maintained alongside the codebase to ensure complete traceability from vision through implementation. The structure enables:

- Clear traceability from requirements to implementation
- Impact analysis when changes are made
- Automated validation through linters
- Graph visualization for design reviews
- Parallel development with clear ownership boundaries

## Key Architectural Decision Records

1. **[COLDVOX-ADR3-001-vosk-model-distribution](ADR3/COLDVOX-ADR3-001-vosk-model-distribution.md)** - Commit Vosk models directly to repository for deterministic CI
2. **[COLDVOX-ADR3-002-hybrid-threading-model](ADR3/COLDVOX-ADR3-002-hybrid-threading-model.md)** - Use dedicated real-time thread for audio capture
3. **[COLDVOX-ADR3-003-adaptive-injection-strategy](ADR3/COLDVOX-ADR3-003-adaptive-injection-strategy.md)** - Implement adaptive strategy for text injection

The documentation structure provides a solid foundation for the "docs-as-graph" approach, enabling comprehensive understanding of the ColdVox architecture and its implementation details.