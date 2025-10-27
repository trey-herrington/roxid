# Architectural Decisions Record

## ADR Template

Use this skeleton for new architectural decisions:

```markdown
# ADR-[NUMBER]: [Title of Decision]

**Date:** YYYY-MM-DD

**Status:** [Proposed | Accepted | Deprecated | Superseded]

## Context

What is the issue we're facing? What factors are driving this decision?
Describe the forces at play (technical, political, social, project-related).

## Decision

What is the change we're proposing/have agreed to?
State the decision clearly and concisely.

## Consequences

What becomes easier or harder as a result of this decision?

### Positive
- Benefit 1
- Benefit 2

### Negative
- Drawback 1
- Drawback 2

### Neutral
- Trade-off 1
- Trade-off 2

## Alternatives Considered

What other options did we evaluate?

### Option 1: [Name]
- Description
- Pros/Cons
- Why rejected

### Option 2: [Name]
- Description
- Pros/Cons
- Why rejected

## References

- Links to relevant discussions, RFCs, documentation
- Related ADRs
```

---

## ADR-001: Use Ratatui as Core TUI Framework

**Date:** 2025-10-23

**Status:** Accepted

### Context

We needed a Terminal User Interface (TUI) framework for building an interactive
console application in Rust. The application requires dynamic rendering,
event handling, and cross-platform terminal support. Key requirements included:

- Modern, actively maintained library
- Good widget ecosystem for building complex UIs
- Strong community and documentation
- Performance and reliability
- Rust-native solution

### Decision

We chose Ratatui as the core TUI framework for this application.

### Consequences

#### Positive

- Immediate mode rendering model makes state management straightforward
- Rich set of built-in widgets (lists, paragraphs, blocks, charts, etc.)
- Excellent documentation and examples
- Active community and regular updates
- Backend-agnostic design (works with crossterm, termion, or termwiz)
- No unsafe code in the core library

#### Negative

- Steeper learning curve compared to simpler print-based approaches
- Requires understanding of immediate mode GUI concepts
- Additional dependency overhead

#### Neutral

- Requires a backend library (we chose crossterm)
- Terminal rendering inherently platform-dependent despite abstractions

### Alternatives Considered

#### Option 1: cursive

- Description: Alternative Rust TUI library with object-oriented design
- Pros: Easier learning curve, built-in event handling
- Cons: Less flexible, heavier abstraction, smaller community
- Why rejected: Ratatui offers more control and better fits immediate mode
  patterns

#### Option 2: tui-rs (deprecated)

- Description: The predecessor to Ratatui
- Pros: Similar API to Ratatui
- Cons: No longer maintained, moved to Ratatui
- Why rejected: Deprecated in favor of Ratatui

#### Option 3: Custom terminal control

- Description: Direct terminal escape codes and manual rendering
- Pros: Maximum control, minimal dependencies
- Cons: Significant development time, platform compatibility issues,
reinventing the wheel
- Why rejected: Not worth the development cost for standard TUI features

### References

- [Ratatui Documentation](https://docs.rs/ratatui)
- [Ratatui Website](https://ratatui.rs/)
- [Migration from tui-rs](https://ratatui.rs/blog/tui-rs-revival/)

---

## ADR-002: Use Crossterm for Terminal Backend

**Date:** 2025-10-23

**Status:** Accepted

### Context

Ratatui requires a terminal backend to handle low-level terminal operations
like cursor control, color output, and raw mode. We needed to choose between
crossterm, termion, and termwiz. Requirements included:

- Cross-platform support (Linux, macOS, Windows)
- Active maintenance
- Good integration with Ratatui
- Reliable event handling

### Decision

We chose Crossterm as the terminal backend for this application.

### Consequences

#### Positive

- Best-in-class Windows support alongside Unix systems
- Actively maintained with regular updates
- Clean, well-documented API
- Asynchronous event handling support
- Most popular choice in the Ratatui ecosystem
- Wide platform compatibility

#### Negative

- Slightly larger dependency tree than termion
- May include features we don't use

#### Neutral

- Locks us into crossterm-specific APIs for event handling
- Terminal capabilities still vary by platform despite abstraction

### Alternatives Considered

#### Option 1: termion

- Description: Unix-focused terminal manipulation library
- Pros: Lighter weight, simpler API
- Cons: Limited/no Windows support, less actively maintained
- Why rejected: Windows support is important for cross-platform
  compatibility

#### Option 2: termwiz

- Description: Terminal manipulation library from wezterm project
- Pros: Full-featured, good Windows support
- Cons: Smaller community, less Ratatui integration examples
- Why rejected: Crossterm has better documentation and wider
  adoption

### References

- [Crossterm Documentation](https://docs.rs/crossterm)
- [Crossterm GitHub](https://github.com/crossterm-rs/crossterm)
- [Ratatui Backend Comparison](https://ratatui.rs/concepts/backends/)

---

## ADR-003: Use color-eyre for Error Handling

**Date:** 2025-10-23

**Status:** Accepted

### Context

We needed an error handling strategy for the TUI application. Standard Rust
error handling with `Result` and `?` operator works but provides minimal
context for debugging. Terminal applications can be harder to debug due to
terminal state corruption on crashes. Requirements included:

- Better error messages and stack traces
- Graceful terminal restoration on panic
- Development and debugging experience improvement
- Minimal code changes to adopt

### Decision

We chose color-eyre for enhanced error reporting and panic handling.

### Consequences

#### Positive

- Beautiful, colored error reports with source code context
- Automatic panic hook installation to restore terminal state
- Suggestion system for common error patterns
- Easy integration with existing `Result` types
- Greatly improved debugging experience
- Works well in both development and production

#### Negative

- Adds dependency overhead
- Error messages may be verbose for simple errors
- Color output might not work in all terminal environments

#### Neutral

- Changes error type to `color_eyre::Result` throughout codebase
- Requires one-time setup with `color_eyre::install()`

### Alternatives Considered

#### Option 1: anyhow

- Description: Flexible error handling library
- Pros: Lightweight, widely used, good error context
- Cons: No automatic terminal restoration, less detailed error reports
- Why rejected: color-eyre provides TUI-specific benefits like terminal
  cleanup

#### Option 2: thiserror

- Description: Derive macros for custom error types
- Pros: Strongly typed errors, minimal runtime overhead
- Cons: More boilerplate, no enhanced panic handling
- Why rejected: For a skeleton app, the boilerplate doesn't add value over
  color-eyre

#### Option 3: Standard library only

- Description: Use `std::error::Error` and `Result<T, E>`
- Pros: No dependencies, minimal overhead
- Cons: Poor error messages, no terminal restoration, harder debugging
- Why rejected: The debugging experience improvement justifies the
  dependency

### References

- [color-eyre Documentation](https://docs.rs/color-eyre)
- [color-eyre GitHub](https://github.com/eyre-rs/color-eyre)

---

## ADR-004: Workspace Architecture with Service, RPC, and TUI Layers

**Date:** 2025-10-22

**Status:** Accepted

### Context

We needed to organize a Rust project that would have multiple interfaces (TUI, CLI, potential web API) all accessing the same core business logic. The codebase needed to be maintainable, testable, and allow for future expansion. Requirements included:

- Clear separation between UI, business logic, and API layers
- Reusable core logic across different interfaces
- Independent testing of each layer
- Ability to add new interfaces without modifying core logic
- Proper Rust workspace organization

### Decision

We chose a three-layer workspace architecture:
1. **Service Layer** (library) - Pure business logic and pipeline execution
2. **RPC Layer** (library) - Remote procedure call API wrapping service
3. **TUI Layer** (binary) - Terminal user interface consuming service

### Consequences

#### Positive

- Clear separation of concerns with well-defined boundaries
- Service layer is completely independent and highly reusable
- Easy to add new interfaces (web, gRPC, etc.) without touching core logic
- Each layer can be tested independently
- Enforces good architectural practices through Rust's module system
- Workspace structure makes dependencies explicit and prevents circular references
- Business logic stays pure without UI or API concerns

#### Negative

- More files and directories to navigate
- Slightly more boilerplate in initial setup
- Need to understand Rust workspace conventions
- May feel over-engineered for very small projects

#### Neutral

- Dependency direction is strictly one-way: TUI → Service, RPC → Service
- Service has zero dependencies on other workspace members
- Each layer has its own Cargo.toml and version management

### Alternatives Considered

#### Option 1: Single binary with modules

- Description: Everything in one `src/` directory with module separation
- Pros: Simpler file structure, easier initial navigation
- Cons: No enforcement of layer boundaries, risk of tight coupling, harder to reuse logic
- Why rejected: Doesn't scale well and allows bad practices

#### Option 2: Service and binary only

- Description: Just service library and TUI binary
- Pros: Simpler than three layers
- Cons: No clear place for RPC/API logic, would end up polluting service or TUI
- Why rejected: RPC layer serves as important API boundary

#### Option 3: Monorepo with separate repositories

- Description: Completely separate projects
- Pros: Maximum independence
- Cons: Harder to develop, version management nightmare, synchronization issues
- Why rejected: Too much overhead for a single application

### References

- [Rust Workspaces Documentation](https://doc.rust-lang.org/book/ch14-03-cargo-workspaces.html)
- [Clean Architecture Principles](https://blog.cleancoder.com/uncle-bob/2012/08/13/the-clean-architecture.html)

---

## ADR-005: YAML-Based Pipeline Execution System

**Date:** 2025-10-26

**Status:** Accepted

### Context

We needed a way to define and execute automated tasks (builds, tests, deployments) that could be run from both the TUI and CLI. Requirements included:

- Simple, human-readable task definitions
- Support for shell commands and scripts
- Environment variable configuration
- Real-time progress reporting for UI integration
- Cross-platform compatibility
- Extensibility for future step types

### Decision

We chose YAML as the pipeline definition format with a custom execution engine built on Tokio for async execution. Pipelines consist of named steps that run commands or shell scripts sequentially.

### Consequences

#### Positive

- YAML is familiar to developers from CI/CD systems (GitHub Actions, GitLab CI)
- Simple, declarative syntax is easy to learn and write
- Parser is straightforward to implement with serde_yaml
- Async execution with Tokio enables real-time progress reporting
- Environment variable support at pipeline and step levels
- Continue-on-error flag allows graceful handling of non-critical failures
- Easy to add new step types (docker, kubernetes, etc.) in the future
- Works consistently across Linux, macOS, and Windows

#### Negative

- YAML indentation can be error-prone
- Limited validation at parse time (errors appear at runtime)
- No built-in parallelization (steps run sequentially)
- Debugging can be harder than native code

#### Neutral

- File extension can be `.yaml` or `.yml`
- Each pipeline is a separate file (no monolithic config)
- Execution context separates working directory from definition location

### Alternatives Considered

#### Option 1: JSON pipeline format

- Description: Use JSON instead of YAML
- Pros: Stricter parsing, better tooling support
- Cons: More verbose, less readable, harder for humans to write
- Why rejected: Developer experience worse than YAML

#### Option 2: Rust DSL with macros

- Description: Define pipelines in Rust using macro DSL
- Pros: Type-safe, compile-time validation, IDE support
- Cons: Requires recompilation for changes, steeper learning curve, less portable
- Why rejected: Too heavyweight, can't be edited without rebuilding

#### Option 3: Embedded scripting (Lua, Rhai)

- Description: Use embedded scripting language for pipeline logic
- Pros: Full programming language features, very flexible
- Cons: Much more complex, harder to learn, overkill for most tasks
- Why rejected: YAML declarative approach is simpler for the use case

#### Option 4: Makefiles or shell scripts

- Description: Use existing build tools
- Pros: Familiar, widespread, no new format
- Cons: Platform-specific, harder to parse for progress reporting, no structured metadata
- Why rejected: Poor integration with TUI progress tracking

### References

- [YAML Specification](https://yaml.org/spec/)
- [GitHub Actions Workflow Syntax](https://docs.github.com/en/actions/using-workflows/workflow-syntax-for-github-actions)
- [Tokio Async Runtime](https://tokio.rs/)

---

## ADR-006: TUI State Machine with Pipeline Discovery

**Date:** 2025-10-26

**Status:** Accepted

### Context

The TUI needed to provide an intuitive interface for discovering, selecting, and executing pipelines. Requirements included:

- Automatic discovery of pipeline files
- Clear visual feedback during execution
- Real-time progress and output display
- Simple keyboard navigation
- State management for different screens

### Decision

Implemented a state machine with two main states: PipelineList and ExecutingPipeline. The app automatically scans the current directory for YAML files at startup and provides keyboard navigation for selection and execution.

### Consequences

#### Positive

- State machine makes transitions explicit and easy to reason about
- Automatic discovery reduces configuration burden
- Real-time updates via async channels provide responsive UI
- Progress bar and output streaming give excellent user feedback
- Non-blocking event handling keeps UI responsive
- Simple keyboard controls (vim-style hjkl supported)
- Color-coded output makes status clear at a glance

#### Negative

- Only discovers files in current directory (no recursive search)
- State machine can't be cancelled mid-execution (must wait for completion)
- Large output can overwhelm display (limited scrollback)

#### Neutral

- Requires valid YAML with name and steps fields to be discovered
- Files must have .yaml or .yml extension
- Selection state resets when returning from execution

### Alternatives Considered

#### Option 1: File browser with directory navigation

- Description: Allow navigating directory tree to find pipelines
- Pros: More flexible, can access pipelines in subdirectories
- Cons: More complex UI, slower to use for common case
- Why rejected: Current directory focus is simpler for most workflows

#### Option 2: Configuration file listing pipelines

- Description: Require explicit registration in config file
- Pros: More control over what appears
- Cons: Manual maintenance, extra step, forgot to register = invisible
- Why rejected: Automatic discovery is more convenient

#### Option 3: Multi-window interface

- Description: Show list and execution output simultaneously
- Pros: Can monitor multiple pipelines
- Cons: Complex UI, limited terminal space, harder navigation
- Why rejected: Sequential focus is clearer on small terminals

### References

- [Ratatui State Management](https://ratatui.rs/concepts/application-patterns/state-management/)
- [Finite State Machine Pattern](https://gameprogrammingpatterns.com/state.html)

---
