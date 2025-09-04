# Repository Onboarding Prompt for Coding Agents

Your task is to "onboard" this repository to a coding agent by creating a `.github/copilot-instructions.md` file that contains comprehensive information describing how a coding agent seeing the repository for the first time can work most efficiently.

You will do this task only one time per repository. Doing a thorough job can SIGNIFICANTLY improve the quality of the agent's work, so take your time, explore comprehensively, and test everything before writing the instructions.

## Goals
- Minimize the likelihood of pull requests getting rejected due to:
  - Code that fails continuous integration builds
  - Code that fails validation pipelines
  - Code with unexpected behavior or side effects
- Reduce bash command and build failures
- Enable faster task completion by minimizing exploration needs (grep, find, code search)
- Provide architectural understanding to prevent breaking changes

## Limitations
- Instructions must be no longer than 3 pages
- Instructions must not be task-specific
- Focus on reusable knowledge that applies to any change

## What to Document

### 1. Repository Overview
<HighLevelDetails>
- **Purpose**: What the repository does and its primary use cases
- **Architecture**: Overall system architecture and design patterns
- **Technology Stack**: Languages, frameworks, runtimes, and their versions
- **Repository Metrics**: Size, number of packages/modules, complexity indicators
</HighLevelDetails>

### 2. System Architecture & Component Interactions
<SystemArchitecture>
Document how the system works as a cohesive whole:

- **Data Flow**: Trace how data moves through the system from input to output
- **Component Communication**:
  - What communication mechanisms are used (channels, buffers, queues, events)
  - Which components communicate with which
  - Synchronous vs asynchronous boundaries
- **Threading/Concurrency Model**:
  - Which components run in dedicated threads
  - Which use async/await patterns
  - Shared state and synchronization mechanisms
- **State Management**: How application state is managed and transitions
- **Error Handling Patterns**:
  - How errors propagate through the system
  - Recovery mechanisms and fallback strategies
  - Watchdogs and monitoring systems
</SystemArchitecture>

### 3. Build System & Development Workflow
<BuildInstructions>
For each development task (bootstrap, build, test, run, lint, format, deploy):

- **Exact Command Sequences**: Document the precise order of commands that work
- **Environment Setup**:
  - Required environment variables and their purposes
  - System library dependencies and installation methods
  - External resources (models, data files, services)
  - Platform-specific requirements
- **Build Variants**:
  - Feature flags and their combinations
  - Default vs optional features
  - Platform-specific build configurations
  - Common feature combinations used together
- **Validation Steps**:
  - How to verify builds succeeded
  - How to run tests (including ignored/special tests)
  - Linting and formatting commands
  - Pre-commit hooks and CI pipeline steps

**Testing Protocol**:
1. Clean the repository and environment
2. Run commands in different orders and document failures
3. Make a test change and document unexpected issues
4. Document workarounds for common problems
5. Note command execution times for long-running operations
6. Use imperative language: "ALWAYS run X before Y"
</BuildInstructions>

### 4. Project Structure & Key Components
<ProjectLayout>
Provide a mental map of the codebase:

- **Directory Structure**:
  - Purpose of each major directory
  - Where different types of code live
  - Configuration file locations and purposes
- **Key Files**:
  - Entry points and main execution paths
  - Core business logic locations
  - Configuration files (build, lint, test, CI/CD)
  - Interface definitions and contracts
- **Module/Package Organization**:
  - Dependencies between modules
  - Public vs internal APIs
  - Shared code and utilities
- **Development Utilities**:
  - Diagnostic tools and their purposes
  - Example programs and how to run them
  - Debugging utilities and helpers
  - Performance profiling tools
</ProjectLayout>

### 5. Runtime Behavior & Operations
<RuntimeBehavior>
- **Logging**:
  - Where logs are written
  - How to control log verbosity
  - Log rotation and management
- **Monitoring & Health**:
  - Health check endpoints or mechanisms
  - Performance metrics collection
  - Diagnostic commands or tools
- **Configuration**:
  - Configuration file formats and locations
  - Runtime vs compile-time configuration
  - Configuration precedence and overrides
- **Resource Management**:
  - Memory usage patterns
  - File handles and network connections
  - Cleanup and shutdown procedures
</RuntimeBehavior>

### 6. Platform & Environment Specifics
<PlatformSpecifics>
- **Platform Detection**: How the build system detects and adapts to platforms
- **OS-Specific Behavior**: Differences between Windows, Linux, macOS
- **Desktop Environment Support**: Special handling for Wayland, X11, KDE, GNOME, etc.
- **Hardware Requirements**: GPU, audio devices, special peripherals
- **Container/VM Considerations**: Special requirements or limitations
</PlatformSpecifics>

## Steps to Follow

1. **Initial Discovery**:
   - Read ALL documentation files (README, CONTRIBUTING, CHANGELOG, etc.)
   - Search for build scripts, makefiles, and project files
   - Examine CI/CD pipelines and workflows
   - Look for HACK, TODO, FIXME, WARNING comments

2. **Architecture Understanding**:
   - Trace main execution paths
   - Map component dependencies
   - Identify communication patterns
   - Document state management

3. **Build System Validation**:
   - Clean environment and test each build command
   - Document the working sequence of commands
   - Try different feature combinations
   - Record all errors and workarounds

4. **Runtime Testing**:
   - Run the application with different configurations
   - Test diagnostic and utility programs
   - Verify logging and monitoring
   - Check error recovery mechanisms

5. **Documentation Writing**:
   - Organize findings into clear sections
   - Use precise, actionable language
   - Include specific file paths and command examples
   - Highlight critical warnings and gotchas

6. **Final Instruction**:
   - End with: "Trust these instructions first. Only search the codebase if information here is incomplete or produces errors."

## Quality Checklist
Before finalizing, ensure the instructions answer:
- [ ] How do I build this project from scratch?
- [ ] What commands must I run before making changes?
- [ ] Where do I make changes for different types of features?
- [ ] How do components communicate with each other?
- [ ] What will break if I change X?
- [ ] How do I verify my changes work correctly?
- [ ] What platform-specific considerations exist?
- [ ] What external dependencies are required?
