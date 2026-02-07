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

## ADR-004: Workspace Architecture with Direct Library Calls

**Date:** 2025-10-27 (Updated 2026-02-07)

**Status:** Accepted (Updated)

### Context

We needed to organize a Rust project with multiple interfaces (TUI, CLI) all accessing the same core business logic. The codebase needed to be maintainable, testable, and allow for future expansion.

The project previously went through several architectural phases:
1. Initially a thin `pipeline-rpc` wrapper layer
2. Then migrated to gRPC-based microservice architecture
3. Finally settled on direct library calls during the complete rewrite (2026-02)

The gRPC approach added unnecessary complexity for a local execution tool. The rewrite eliminated all network protocol overhead in favor of direct Rust library dependencies.

### Decision

We chose a workspace with three crates using direct library calls:

1. **pipeline-service** (library) - Core pipeline parsing, expression evaluation, execution engine, runners, task management, testing framework
2. **roxid-tui** (library + binary) - Terminal user interface, depends directly on `pipeline-service`
3. **roxid-cli** (binary) - CLI entry point, depends on both `roxid-tui` and `pipeline-service`

### Consequences

#### Positive

- Clear separation of concerns with well-defined crate boundaries
- pipeline-service is completely independent and highly reusable
- No network protocol overhead for local execution
- Simple dependency graph: `roxid-cli -> roxid-tui -> pipeline-service`
- Each crate can be tested independently
- No service lifecycle management needed (no separate process to start/stop)
- Workspace structure makes dependencies explicit

#### Negative

- More files and directories to navigate than a single crate
- Need to understand Rust workspace conventions

#### Neutral

- Dependency direction is strictly enforced by Cargo
- pipeline-service has zero dependencies on other workspace members
- Each crate has its own Cargo.toml and version management (all at v0.8.0)

### References

- [Rust Workspaces Documentation](https://doc.rust-lang.org/book/ch14-03-cargo-workspaces.html)
- Supersedes previous gRPC-based architecture (ADR-011)

---

## ADR-005: YAML-Based Pipeline Execution System

**Date:** 2025-10-26 (Updated 2026-02-07)

**Status:** Accepted (Updated)

### Context

We needed a way to define and execute automated tasks (builds, tests, deployments) that could be run from both the TUI and CLI. The project evolved from a simple custom YAML format to full Azure DevOps pipeline compatibility.

### Decision

We chose to implement 100% Azure DevOps Pipelines compatible YAML parsing and execution. Pipelines use the standard `azure-pipelines.yml` format with stages, jobs, steps, variables, parameters, templates, and expressions.

### Consequences

#### Positive

- Users can run their existing Azure DevOps pipelines locally without modification
- Full expression engine (`${{ }}`, `$[ ]`, `$(var)`) with built-in functions
- Template system with `extends`, `${{ if }}`, `${{ each }}`, and cross-repo support
- DAG-based execution with dependency resolution, parallel execution, and matrix strategies
- Comprehensive testing framework with JUnit/TAP output

#### Negative

- Azure DevOps YAML schema is complex, requiring significant parsing logic
- Some Azure DevOps features depend on cloud services not available locally
- Expression engine edge cases require ongoing maintenance

#### Neutral

- Pipeline files use standard `.yml` or `.yaml` extension
- Triggers are parsed but execution is manual-only (local tool)
- GitHub Actions workflow support is planned as a future phase

### References

- [Azure DevOps YAML Schema](https://docs.microsoft.com/en-us/azure/devops/pipelines/yaml-schema)
- [Azure DevOps Expressions](https://docs.microsoft.com/en-us/azure/devops/pipelines/process/expressions)
- [Azure DevOps Templates](https://docs.microsoft.com/en-us/azure/devops/pipelines/process/templates)

---

## ADR-006: TUI State Machine with Pipeline Discovery

**Date:** 2025-10-26 (Updated 2026-02-07)

**Status:** Accepted (Updated)

### Context

The TUI needed to provide an intuitive interface for discovering, selecting, inspecting, executing, and testing Azure DevOps pipelines locally.

### Decision

Implemented a state machine with six states and a navigation stack (`previous_states: Vec<AppState>`) with `push_state()`/`go_back()` pattern:

```rust
pub enum AppState {
    PipelineList,        // Browse discovered pipelines
    PipelineDetail,      // Expandable tree: stages -> jobs -> steps
    ExecutingPipeline,   // Real-time execution with progress
    ExecutionLog,        // Scrollable/searchable log viewer
    TestResults,         // Test suite pass/fail visualization
    VariableEditor,      // Edit variables before execution
}
```

### Consequences

#### Positive

- Six states cover the full workflow from discovery to testing
- Navigation stack enables natural back/forward navigation
- `pending_execution`/`pending_test_run` flags bridge sync keyboard handlers to async main loop
- Progress channel feeds `ExecutionEvent`s from spawned tokio task to TUI
- Real-time updates via async channels provide responsive UI
- Pipeline tree view gives clear visibility into pipeline structure

#### Negative

- State machine complexity increases with more states
- Flag-based async bridging adds indirection

#### Neutral

- `App::new()` is synchronous -- pipeline discovery uses `AzureParser::parse_file()` directly
- All 16 `ExecutionEvent` variants processed in `process_execution_events()`

### References

- [Ratatui State Management](https://ratatui.rs/concepts/application-patterns/state-management/)

---

## ADR-007: Clients Must Use RPC Layer, Not Service Layer Directly

**Date:** 2025-10-27

**Status:** Superseded by ADR-014

*This ADR enforced a `pipeline-rpc` API boundary layer. The complete rewrite (2026-02) eliminated the RPC layer entirely. Both CLI and TUI now depend directly on `pipeline-service` via Rust library calls. See ADR-014.*

---

## ADR-008: Code Formatting and Quality Standards

**Date:** 2025-10-27

**Status:** Accepted

### Context

The codebase needed consistent formatting standards to improve maintainability, readability, and reduce merge conflicts.

### Decision

Enforced consistent code formatting using `rustfmt` with default Rust formatting conventions. No `rustfmt.toml` or `clippy.toml` -- default rules apply.

### Consequences

#### Positive

- Consistent codebase: All files follow same formatting standards
- Automated enforcement: `cargo fmt` can verify/fix formatting
- Reduced bike-shedding: No debates about formatting in code reviews
- CI integration: `cargo fmt --check` and `cargo clippy -- -D warnings`

#### Neutral

- Future changes automatically maintain standards with `cargo fmt`
- Three-tier import organization: (1) local crate, (2) std, (3) external crates

### References

- [rustfmt Documentation](https://rust-lang.github.io/rustfmt/)
- [Rust Style Guide](https://doc.rust-lang.org/nightly/style-guide/)

---

## ADR-009: API Boundary Refinement and Parser Simplification

**Date:** 2025-10-27

**Status:** Superseded by ADR-014

*This ADR addressed API inconsistencies in the `pipeline-rpc` layer. The RPC layer no longer exists. The parser API is now `AzureParser::parse()` and `AzureParser::parse_file()` in `pipeline-service`, called directly by clients. See ADR-014.*

---

## ADR-010: Documentation Consolidation

**Date:** 2025-10-27

**Status:** Accepted

### Context

The repository had separate files for installation and publishing instructions. Consolidating into README.md improved discoverability.

### Decision

Consolidated all user-facing documentation into `README.md`. Removed separate `INSTALL.md` and `PUBLISHING.md` files.

### Consequences

#### Positive

- Single source of truth for user-facing documentation
- Easier discovery: Users find everything in README
- Less maintenance: One file to keep updated

### References

- [GitHub README Best Practices](https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/about-readmes)

---

## ADR-011: Migration to gRPC Architecture

**Date:** 2025-11-03

**Status:** Superseded by ADR-014

*This ADR described migration to gRPC with Tonic. The complete rewrite (2026-02) removed all gRPC infrastructure in favor of direct library calls. There are no proto files, no tonic/prost dependencies, no build.rs for proto compilation, and no server binary. See ADR-014.*

---

## ADR-012: Automatic Service Lifecycle Management

**Date:** 2025-11-03

**Status:** Superseded by ADR-014

*This ADR implemented auto-start/stop of a gRPC service process. With the removal of gRPC (ADR-014), there is no separate service process to manage. The CLI and TUI call pipeline-service as a library directly in-process. See ADR-014.*

---

## ADR-013: TUI Async Execution Model Fix

**Date:** 2025-11-03

**Status:** Superseded by ADR-014

*This ADR fixed nested tokio runtime issues in the gRPC-based TUI. The rewrite (ADR-014) redesigned the TUI from scratch with a proper async model using `pending_execution` flags and progress channels, making this fix obsolete. The pattern described (flag-based bridging from sync event handlers to async main loop) was incorporated into the new architecture. See ADR-006 (Updated).*

---

## ADR-014: Complete Rewrite - Azure DevOps Pipeline Emulator

**Date:** 2026-02-04

**Status:** Accepted

### Context

The original Roxid was a simple pipeline runner with custom YAML format and gRPC architecture. It had several limitations:

- Custom pipeline format not compatible with any CI system
- gRPC architecture added complexity for what is fundamentally a local tool
- No expression engine, template system, or testing framework
- Limited pipeline features (sequential steps only, no stages/jobs/dependencies)

We needed a complete rewrite to become a 100% Azure DevOps Pipelines compatible local execution environment.

### Decision

Complete rewrite of all three crates over 7 phases:

1. **Core Foundation**: Comprehensive Azure DevOps data models, YAML parser with validation, full expression engine with built-in functions
2. **Execution Engine**: DAG builder, pipeline executor with parallel execution, matrix expansion, condition evaluation
3. **Runners**: Shell runner (script/bash/pwsh/powershell), task runner (Azure DevOps marketplace tasks), container runner (Docker)
4. **Template System**: Template resolution with extends, `${{ if }}`/`${{ each }}` directives, parameter validation, cross-repo templates
5. **Testing Framework**: Test definitions, assertion engine, test runner with filtering, JUnit/TAP/terminal reporters
6. **TUI Rewrite**: Six-state application, pipeline tree view, real-time execution, log viewer, test results, variable editor
7. **CLI Enhancements**: run, test, validate, task subcommands with clap 4

### Key Architectural Changes

- **Removed gRPC**: All communication is now direct Rust library calls
- **Removed `pipeline-rpc` crate**: No longer needed without network protocol
- **No service process**: Everything runs in a single process
- **Azure DevOps first**: Primary format is Azure DevOps YAML (GitHub Actions planned for future)
- **Direct library dependency**: `roxid-cli -> roxid-tui -> pipeline-service`

### Consequences

#### Positive

- Run real `azure-pipelines.yml` files locally without modification
- Full expression engine with all three expression types
- Template system matching Azure DevOps behavior
- Testing framework for pipeline logic validation
- Simpler architecture with no network overhead
- 245+ tests across all crates

#### Negative

- Complete rewrite, not incremental improvement
- Azure DevOps YAML complexity adds maintenance burden
- Some Azure DevOps features depend on cloud services (triggers, some tasks)

#### Neutral

- GitHub Actions compatibility planned as future phase
- Mock/stub system for external services deferred to post-MVP
- Dry-run mode deferred to post-MVP

### References

- [REWRITE_PLAN.md](./REWRITE_PLAN.md) - Detailed implementation plan
- Supersedes: ADR-004 (partially), ADR-007, ADR-009, ADR-011, ADR-012, ADR-013

---
