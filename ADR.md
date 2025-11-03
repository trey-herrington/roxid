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

## ADR-004: Workspace Architecture with Service, RPC, and Client Layers

**Date:** 2025-10-27 (Updated)

**Status:** Accepted

### Context

We needed to organize a Rust project that would have multiple interfaces (TUI, CLI, potential web API) all accessing the same core business logic. The codebase needed to be maintainable, testable, and allow for future expansion. Requirements included:

- Clear separation between UI, business logic, and API layers
- Reusable core logic across different interfaces
- Independent testing of each layer
- Ability to add new interfaces without modifying core logic
- Proper Rust workspace organization
- **Clients should not directly access business logic - all access through API layer**

### Decision

We chose a layered workspace architecture with strict dependency rules:

1. **pipeline-service** (library) - Pure business logic and pipeline execution
2. **pipeline-rpc** (library) - API layer providing handlers that wrap service functionality
3. **roxid-tui** (binary) - Terminal user interface consuming only RPC API
4. **roxid-cli** (binary) - Command-line interface consuming only RPC API

**Critical Rule**: Client applications (TUI and CLI) **only** depend on `pipeline-rpc`, never on `pipeline-service` directly.

### Consequences

#### Positive

- Clear separation of concerns with well-defined boundaries
- pipeline-service is completely independent and highly reusable
- **API layer provides consistent interface for all clients**
- **Easy to add authentication, validation, and rate limiting in RPC layer**
- **Can convert RPC layer to network service without changing clients**
- Easy to add new interfaces (web, gRPC, mobile) without touching core logic
- Each layer can be tested independently
- Enforces good architectural practices through Rust's module system
- Workspace structure makes dependencies explicit and prevents circular references
- Business logic stays pure without UI or API concerns
- Multiple frontend options (TUI and CLI) share same API interface
- **RPC layer can evolve independently of service implementation**
- **Clients are shielded from service layer changes**

#### Negative

- More files and directories to navigate
- Additional indirection layer (RPC) between clients and service
- Slightly more boilerplate in initial setup
- Need to understand Rust workspace conventions
- May feel over-engineered for very small projects
- **All new service features must be exposed through RPC handlers**

#### Neutral

- Dependency direction is strictly enforced: roxid-tui → pipeline-rpc → pipeline-service, roxid-cli → pipeline-rpc → pipeline-service
- pipeline-service has zero dependencies on other workspace members
- **pipeline-rpc acts as the single source of truth for the API**
- Each layer has its own Cargo.toml and version management
- RPC layer re-exports necessary types from service layer for client convenience

### Alternatives Considered

#### Option 1: Single binary with modules

- Description: Everything in one `src/` directory with module separation
- Pros: Simpler file structure, easier initial navigation
- Cons: No enforcement of layer boundaries, risk of tight coupling, harder to reuse logic
- Why rejected: Doesn't scale well and allows bad practices

#### Option 2: Clients directly accessing pipeline-service

- Description: TUI and CLI depend directly on pipeline-service (previous architecture)
- Pros: One less layer, simpler dependency graph
- Cons: No API abstraction, clients coupled to service implementation, harder to add cross-cutting concerns
- Why rejected: Violates clean architecture principles, harder to evolve API independently

#### Option 3: Monorepo with separate repositories

- Description: Completely separate projects
- Pros: Maximum independence
- Cons: Harder to develop, version management nightmare, synchronization issues
- Why rejected: Too much overhead for a single application

### References

- [Rust Workspaces Documentation](https://doc.rust-lang.org/book/ch14-03-cargo-workspaces.html)
- [Clean Architecture Principles](https://blog.cleancoder.com/uncle-bob/2012/08/13/the-clean-architecture.html)
- [Hexagonal Architecture](https://alistair.cockburn.us/hexagonal-architecture/)

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

## ADR-007: Clients Must Use RPC Layer, Not Service Layer Directly

**Date:** 2025-10-27

**Status:** Accepted

### Context

Initially, both roxid-cli and roxid-tui had direct dependencies on pipeline-service, bypassing the pipeline-rpc layer. This created several issues:

- Tight coupling between clients and service implementation
- No single API boundary for external access
- Difficult to add cross-cutting concerns (auth, logging, rate limiting)
- RPC layer was underutilized and redundant
- Unclear whether to use service or RPC for new features
- Inconsistent access patterns across the codebase

We needed to enforce a clear architectural boundary where all external access to business logic goes through a well-defined API layer.

### Decision

**All client applications (CLI, TUI, and future clients) must depend only on `pipeline-rpc` and never directly on `pipeline-service`.**

Implementation changes:
1. Created `PipelineHandler` in pipeline-rpc to wrap pipeline operations
2. Exported necessary types (`ExecutionEvent`, `Pipeline`, `StepStatus`) from pipeline-rpc
3. Updated roxid-cli to remove pipeline-service dependency and use PipelineHandler
4. Updated roxid-tui to remove pipeline-service dependency and use PipelineHandler
5. RPC layer now provides complete API surface for clients

### Consequences

#### Positive

- **Single API boundary**: All external access goes through one well-defined layer
- **Consistent interface**: CLI and TUI use identical API, ensuring consistency
- **Easier evolution**: Service layer can change without breaking clients (as long as RPC API stays stable)
- **Cross-cutting concerns**: Can add auth, logging, metrics, rate limiting in RPC layer
- **Network transparency**: RPC layer can be converted to network service without client changes
- **Testing**: Can mock RPC layer for client tests without involving service
- **Security**: RPC layer acts as security boundary, validating inputs before reaching service
- **Documentation**: Single API layer to document for external consumers
- **API versioning**: Can version RPC API independently of service implementation

#### Negative

- **Extra indirection**: One more layer between clients and actual business logic
- **API maintenance**: Every service feature must be explicitly exposed via RPC handler
- **Type duplication**: Some types need to be re-exported or wrapped
- **Learning curve**: Developers must understand layering rules

#### Neutral

- RPC layer re-exports types from service layer for convenience
- PipelineHandler is currently a thin wrapper, may grow with features
- Clients can only access features explicitly exposed by RPC handlers

### Alternatives Considered

#### Option 1: Allow direct service access for internal clients

- Description: Let CLI/TUI access pipeline-service directly since they're "internal"
- Pros: Less indirection, simpler dependency graph
- Cons: Inconsistent access patterns, hard to migrate to network service later, no API boundary
- Why rejected: Violates architectural principle, makes future evolution harder

#### Option 2: Separate internal and external APIs

- Description: Have two API layers - one for internal clients, one for external
- Pros: Internal clients get "fast path", external clients get security boundary
- Cons: Maintenance burden, inconsistency, unclear which API to use
- Why rejected: Complexity doesn't justify benefits for this project

#### Option 3: Make service layer the API

- Description: Pipeline-service becomes the public API, eliminate RPC layer
- Pros: One less layer
- Cons: No place for cross-cutting concerns, harder to add network protocol later
- Why rejected: Service layer should remain pure business logic

### Migration Path

For future features:
1. Implement core logic in pipeline-service
2. Create handler in pipeline-rpc to expose functionality
3. Export necessary types from pipeline-rpc
4. Use RPC handler in clients (CLI/TUI)

**Never** import pipeline-service directly in client code.

### References

- [Clean Architecture - The Dependency Rule](https://blog.cleancoder.com/uncle-bob/2012/08/13/the-clean-architecture.html)
- [Hexagonal Architecture (Ports and Adapters)](https://alistair.cockburn.us/hexagonal-architecture/)
- [ADR-004: Workspace Architecture](#adr-004-workspace-architecture-with-service-rpc-and-client-layers)

---

## ADR-008: Code Formatting and Quality Standards

**Date:** 2025-10-27

**Status:** Accepted

### Context

The codebase had accumulated inconsistent formatting across multiple files:
- Mixed formatting styles in struct initializations and function calls
- Inconsistent spacing and line breaks
- Non-standard use of `rustfmt` conventions
- Code readability suffered from formatting variations

We needed to establish consistent code formatting standards to improve maintainability, readability, and reduce merge conflicts.

### Decision

Enforced consistent code formatting using `rustfmt` with default Rust formatting conventions:
- Vertically formatted struct fields for multi-field initializations
- Consistent spacing around operators and braces
- Standardized line breaks for function parameters
- Applied formatting across entire workspace (all crates)

### Consequences

#### Positive

- **Consistent codebase**: All files follow same formatting standards
- **Improved readability**: Code is easier to scan and understand
- **Automated enforcement**: `cargo fmt` can verify/fix formatting
- **Reduced bike-shedding**: No debates about formatting in code reviews
- **Better diffs**: Logical changes aren't mixed with formatting changes
- **Professional appearance**: Code looks polished and well-maintained
- **CI integration ready**: Can add formatting checks to CI pipeline

#### Negative

- **Large initial diff**: Reformatting touched many files at once
- **Git history noise**: Formatting commits can obscure functional changes
- **Potential merge conflicts**: In-progress branches may conflict with formatting changes

#### Neutral

- One-time reformatting applied across entire project
- Future changes automatically maintain standards with `cargo fmt`
- Formatting is purely stylistic - no functional changes

### Implementation Details

Files reformatted include:
- `pipeline-service/src/pipeline/executor.rs` - Event enum and execution logic
- `pipeline-rpc/src/handlers/*` - Handler implementations
- `roxid-tui/src/app.rs` - TUI application logic
- `roxid-tui/src/ui/components.rs` - UI rendering components
- `roxid-cli/src/main.rs` - CLI entry point
- And other workspace files for consistency

### Alternatives Considered

#### Option 1: Manual formatting guidelines

- Description: Document formatting rules without enforcement
- Pros: No tooling required, flexibility
- Cons: Inconsistently applied, requires review comments, wastes time
- Why rejected: Unenforceable, leads to inconsistency

#### Option 2: Custom rustfmt configuration

- Description: Override default rustfmt rules with custom config
- Pros: Tailored to project preferences
- Cons: Harder to onboard new developers, non-standard conventions
- Why rejected: Default rustfmt conventions are well-designed and familiar

#### Option 3: Only format new code

- Description: Leave existing code as-is, format only new additions
- Pros: Minimal disruption
- Cons: Perpetuates inconsistency, harder to maintain standards
- Why rejected: Inconsistency is worse than one-time reformatting pain

### References

- [rustfmt Documentation](https://rust-lang.github.io/rustfmt/)
- [Rust Style Guide](https://doc.rust-lang.org/nightly/style-guide/)

---

## ADR-009: API Boundary Refinement and Parser Simplification

**Date:** 2025-10-27

**Status:** Accepted

### Context

During the implementation of ADR-007 (enforcing RPC layer usage), we discovered API inconsistencies:

- `PipelineParser` had both `from_str()` and `parse()` methods doing the same thing
- Multiple ways to achieve the same result confused API consumers
- `pipeline-rpc` exposed `from_str()` while internal code used `parse()`
- Method naming didn't follow Rust conventions (`parse` is more idiomatic than `from_str`)

### Decision

Simplified and standardized the pipeline parsing API:
1. Kept `parse()` as the primary method in `PipelineParser`
2. Updated `pipeline-rpc::PipelineHandler` to use `parse()` instead of `from_str()`
3. Removed redundant parsing methods
4. Standardized naming throughout the API surface

### Consequences

#### Positive

- **Single, clear API**: Only one way to parse pipelines
- **Idiomatic Rust**: `parse()` follows Rust naming conventions
- **Less confusion**: Developers know which method to use
- **Easier documentation**: Fewer methods to explain
- **Reduced maintenance**: Less code to maintain and test
- **Consistency**: Internal and external APIs use same method names

#### Negative

- **Breaking change**: If external code called `from_str()` directly (unlikely given project age)
- **Migration needed**: Had to update call sites in pipeline-rpc

#### Neutral

- Change is internal to the workspace (no external users yet)
- Both methods had identical functionality, so no behavioral change

### Implementation Details

Changed in `pipeline-rpc/src/handlers/pipeline_handler.rs`:
```rust
// Before:
pub fn parse_from_str(&self, content: &str) -> RpcResult<Pipeline> {
    Ok(PipelineParser::from_str(content)?)
}

// After:
pub fn parse_from_str(&self, content: &str) -> RpcResult<Pipeline> {
    Ok(PipelineParser::parse(content)?)
}
```

### Alternatives Considered

#### Option 1: Keep both methods

- Description: Maintain both `parse()` and `from_str()` as aliases
- Pros: No breaking changes, maximum compatibility
- Cons: API confusion, documentation burden, unnecessary maintenance
- Why rejected: Simplicity and clarity more important than non-existent backward compatibility

#### Option 2: Use `from_str()` everywhere

- Description: Standardize on `from_str()` instead of `parse()`
- Pros: Alternative naming convention
- Cons: Less idiomatic Rust (`parse` is more common), more keystrokes
- Why rejected: `parse()` is more idiomatic in Rust ecosystem

### References

- [Rust API Guidelines - Method Names](https://rust-lang.github.io/api-guidelines/naming.html)
- [ADR-007: Clients Must Use RPC Layer](#adr-007-clients-must-use-rpc-layer-not-service-layer-directly)

---

## ADR-010: Documentation Consolidation

**Date:** 2025-10-27

**Status:** Accepted

### Context

The repository had separate files for installation and publishing instructions:
- `INSTALL.md` (85 lines) - Installation instructions for end users
- `PUBLISHING.md` (179 lines) - Publishing guide for maintainers
- `README.md` - Main documentation

This fragmented documentation structure had several issues:
- Information spread across multiple files
- Harder to find relevant documentation
- Duplication between files
- README didn't have complete installation instructions
- More files to maintain and keep synchronized

### Decision

Consolidated all documentation into `README.md`:
1. Deleted `INSTALL.md` and `PUBLISHING.md`
2. Moved installation instructions to README with multiple options:
   - Install from crates.io (recommended)
   - Pre-built binary downloads
   - Install from Git
   - Build from source
3. Added detailed platform-specific instructions
4. Included verification and uninstallation steps

### Consequences

#### Positive

- **Single source of truth**: All user-facing documentation in one place
- **Easier discovery**: Users find everything in README
- **Better UX**: Complete installation guide in main documentation
- **Less maintenance**: One file to keep updated instead of three
- **Standard convention**: Most projects document installation in README
- **GitHub integration**: README shows on repository home page
- **Multiple installation paths**: Users can choose method that fits their needs

#### Negative

- **Longer README**: More scrolling to find specific sections
- **Publishing info removed**: Maintainer-specific docs may need separate location
- **Git history**: Installation history previously in INSTALL.md is now harder to track

#### Neutral

- Removed example YAML files (`example-pipeline.yaml`, `advanced-pipeline.yaml`, `rust-build-pipeline.yaml`) - likely moved to examples directory or incorporated into documentation
- Documentation is now more end-user focused, less maintainer-focused

### Implementation Details

**Removed files:**
- `INSTALL.md` - 85 lines
- `PUBLISHING.md` - 179 lines
- `advanced-pipeline.yaml` - 58 lines (example file)
- `example-pipeline.yaml` - 26 lines (example file)
- `rust-build-pipeline.yaml` - 34 lines (example file)

**Added to README:**
- Four installation options (crates.io, binary, git, source)
- Platform-specific instructions (Linux, macOS, Windows)
- Verification steps
- Uninstallation instructions
- Total: ~51 lines added

**Net change:** Reduced by ~468 lines across multiple files while improving accessibility.

### Alternatives Considered

#### Option 1: Keep separate INSTALL.md

- Description: Maintain dedicated installation file
- Pros: Focused document, shorter README
- Cons: Users have to navigate to find it, less discoverable
- Why rejected: README is standard location for installation instructions

#### Option 2: Move to docs/ directory

- Description: Create `docs/` folder with organized subdocuments
- Pros: Organized structure, scalable for large documentation
- Cons: Overkill for current project size, less accessible
- Why rejected: Project documentation not yet large enough to justify

#### Option 3: Keep PUBLISHING.md

- Description: Remove INSTALL.md but keep maintainer docs
- Pros: Preserves maintainer information
- Cons: Publishing process may be automated/CI-based now, making manual docs less relevant
- Why rejected: If needed later, can be recreated or added to CONTRIBUTING.md

### References

- [GitHub README Best Practices](https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/about-readmes)
- [Make a README](https://www.makeareadme.com/)

---

## ADR-011: Migration to gRPC Architecture

**Date:** 2025-11-03

**Status:** Accepted

### Context

The original architecture used a thin `pipeline-rpc` abstraction layer that didn't provide actual RPC functionality - it was essentially just an API wrapper around the `pipeline-service`. This created several issues:

- The "RPC" layer didn't actually provide remote procedure call capabilities
- Clients and service were tightly coupled, requiring same process/machine
- No true service independence or remote execution capability
- Limited ability to support clients in other languages
- Confusion about the purpose of the RPC layer

The application needed a true microservice architecture where:
- Service can run independently (different process, machine, container)
- Multiple clients can connect concurrently
- Real-time streaming updates during pipeline execution
- Language-agnostic client support
- True separation between service and clients

### Decision

**Converted the architecture to use gRPC with Tonic:**

1. **Removed `pipeline-rpc` package** - eliminated the non-functional abstraction layer
2. **Converted `pipeline-service` to gRPC service**:
   - Created `proto/pipeline.proto` defining the service interface
   - Implemented gRPC server binary (`src/bin/server.rs`)
   - Added proto conversion code (`src/grpc.rs`)
   - Service listens on `[::1]:50051`
3. **Updated clients to use gRPC**:
   - Both `roxid-cli` and `roxid-tui` now connect via gRPC
   - Clients maintain persistent connections for streaming
   - Real-time execution events via gRPC streaming

**gRPC Service Definition:**
```protobuf
service PipelineService {
    rpc ParsePipeline(ParsePipelineRequest) returns (ParsePipelineResponse);
    rpc ExecutePipeline(ExecutePipelineRequest) returns (stream ExecutionEvent);
}
```

### Consequences

#### Positive

- **True Service Independence**: Service runs in separate process, can be deployed remotely
- **Concurrent Clients**: Multiple TUI/CLI instances can use the service simultaneously
- **Real-time Streaming**: gRPC streaming provides immediate execution feedback
- **Language Agnostic**: Any language with gRPC support can build clients (Python, Go, Java, etc.)
- **Scalability**: Service can be containerized, load-balanced, deployed in Kubernetes
- **Network Transparency**: Same code works locally or with remote service
- **Standard Protocol**: Uses industry-standard Protocol Buffers and HTTP/2
- **Type Safety**: Proto definitions provide strong typing across language boundaries
- **Better Architecture**: Clear separation between service and clients
- **Development Workflow**: Service can be developed/tested independently

#### Negative

- **Service Must Be Running**: Clients require service to be running (previously in-process)
- **Additional Complexity**: gRPC adds more moving parts compared to direct library calls
- **Proto Compilation**: Build process now includes proto compilation step
- **Connection Management**: Clients must handle connection errors and retries
- **Startup Overhead**: Two processes instead of one for local development

#### Neutral

- Service default is localhost IPv6 (`[::1]:50051`)
- Proto files define the contract between service and clients
- Both clients and service compile the proto definitions
- Removed one package (pipeline-rpc) but service gained gRPC dependencies

### Implementation Details

**Files Created:**
- `pipeline-service/proto/pipeline.proto` - Service definition
- `pipeline-service/build.rs` - Proto compilation
- `pipeline-service/src/grpc.rs` - Type conversions
- `pipeline-service/src/bin/server.rs` - gRPC server
- `roxid-cli/build.rs` - Proto compilation
- `roxid-tui/build.rs` - Proto compilation

**Files Modified:**
- All Cargo.toml files updated with tonic/prost dependencies
- `roxid-cli/src/main.rs` - gRPC client implementation
- `roxid-tui/src/app.rs` - gRPC client and streaming
- `roxid-tui/src/events.rs` - Async execution handling
- `roxid-tui/src/lib.rs` - Async initialization
- Workspace `Cargo.toml` - Removed pipeline-rpc member

**Files Removed:**
- Entire `pipeline-rpc/` directory and all contents

**Dependencies Added:**
- `tonic = "0.12"` - gRPC framework
- `prost = "0.13"` - Protocol Buffers
- `tonic-build = "0.12"` - Build-time proto compilation
- `tokio-stream = "0.1"` - Async streaming utilities

### Migration Path

**For Development:**
1. Start the service: `cargo run --bin pipeline-service`
2. In another terminal: `cargo run --bin roxid`

**For Production:**
- Service can be containerized and deployed independently
- Clients connect via network (update address in client code)
- Service can use TLS for encrypted communication (future enhancement)
- Authentication/authorization can be added to gRPC service (future enhancement)

### Future Enhancements

Enabled by this architecture:
- **TLS/mTLS**: Encrypted and authenticated connections
- **Authentication**: Token-based or certificate-based auth
- **Load Balancing**: Multiple service instances behind load balancer
- **Service Discovery**: Integrate with Consul, etcd, or Kubernetes
- **Metrics/Tracing**: gRPC middleware for observability
- **Web Clients**: JavaScript/TypeScript clients with grpc-web
- **Mobile Clients**: iOS/Android apps using gRPC libraries
- **API Gateway**: Expose service via gRPC-HTTP gateway

### Alternatives Considered

#### Option 1: Keep pipeline-rpc as thin wrapper

- Description: Leave architecture as-is with in-process abstraction
- Pros: Simpler, no network complexity
- Cons: No remote execution, no language-agnostic clients, misleading naming
- Why rejected: Doesn't provide actual RPC capability, limits growth

#### Option 2: REST API with HTTP/JSON

- Description: Implement REST API instead of gRPC
- Pros: More familiar, easier debugging with curl
- Cons: No bidirectional streaming, more overhead, manual serialization
- Why rejected: gRPC streaming perfect for real-time pipeline updates

#### Option 3: Message Queue (RabbitMQ/Kafka)

- Description: Use message broker for communication
- Pros: Async by nature, good for high volume
- Cons: More infrastructure, overkill for request-response, complex setup
- Why rejected: Too heavyweight for direct client-service communication

#### Option 4: WebSocket-based custom protocol

- Description: Build custom WebSocket protocol
- Pros: Real-time bidirectional communication
- Cons: Reinventing the wheel, no tooling, no multi-language support
- Why rejected: gRPC provides all benefits with better tooling

### References

- [gRPC Documentation](https://grpc.io/docs/)
- [Tonic (Rust gRPC)](https://github.com/hyperium/tonic)
- [Protocol Buffers](https://protobuf.dev/)
- [ADR-004: Workspace Architecture](#adr-004-workspace-architecture-with-service-rpc-and-client-layers) - Superseded by this decision
- [ADR-007: Clients Must Use RPC Layer](#adr-007-clients-must-use-rpc-layer-not-service-layer-directly) - Superseded by this decision

---

## ADR-012: Automatic Service Lifecycle Management

**Date:** 2025-11-03

**Status:** Accepted

### Context

After implementing the gRPC architecture (ADR-011), users needed to manually start the pipeline-service before running `roxid`. This created friction:

- Required two terminal windows (one for service, one for client)
- Easy to forget to start the service
- Users had to manage background processes manually
- Service would remain running after client exited
- Not intuitive for simple use cases

We wanted a "just works" experience where users could simply type `roxid` and everything would work automatically.

### Decision

**Implemented automatic service lifecycle management in the `roxid` binary:**

1. **Auto-start**: Client automatically starts `pipeline-service` if not running
2. **Auto-stop**: Client stops service on exit only if it started it
3. **Smart detection**: Doesn't interfere with manually managed services
4. **Cross-platform**: Works on Unix/Linux/macOS (pkill) and Windows (taskkill)

### Implementation

**Added to `roxid-cli/src/main.rs`:**

```rust
fn is_service_running() -> bool {
    // Check if port 50051 is listening
}

fn start_service() -> Result<bool> {
    // Spawn pipeline-service binary
    // Wait for it to be ready
    // Return true (we started it)
}

fn stop_service() {
    // Kill pipeline-service process
    // Unix: pkill -f pipeline-service
    // Windows: taskkill /F /IM pipeline-service.exe
}

async fn ensure_service_running() -> Result<bool> {
    // Returns whether we started the service
}
```

**Main function tracks service ownership:**
```rust
let we_started_service = ensure_service_running().await?;

// ... run client ...

// Cleanup only if we started it
if we_started_service {
    stop_service();
}
```

### Consequences

#### Positive

- **Zero configuration**: Just run `roxid` and it works
- **Clean**: No leftover processes after exit
- **Smart**: Respects manually managed services
- **Simple**: User doesn't think about service management
- **Fast**: Reuses running service when available
- **Intuitive**: Works like a normal application

#### Negative

- **Process management complexity**: Client now manages child processes
- **Platform-specific code**: Different commands for Unix vs Windows
- **Hidden behavior**: Service lifecycle less visible to users
- **Startup delay**: ~1-2 seconds to start service first time

#### Neutral

- Service must be in same directory as `roxid` binary
- Uses port 50051 for service detection
- Logs go to stdio (hidden when auto-started)
- Service stops quickly but not gracefully

### User Experience

**Before:**
```bash
# Terminal 1
$ cargo run --bin pipeline-service
Pipeline gRPC server listening on [::1]:50051

# Terminal 2
$ cargo run --bin roxid run pipeline.yaml
# ... runs ...
```

**After:**
```bash
$ roxid run pipeline.yaml
Starting pipeline service...
Service ready!
# ... runs ...
Stopping service...
$
```

### Edge Cases Handled

1. **Service already running manually**: Detected, reused, left running ✓
2. **Service fails to start**: Clear error message ✓
3. **Service on wrong port**: Not detected, starts new instance ✓
4. **Multiple clients**: First client starts, subsequent reuse ✓
5. **Client crashes**: Service keeps running (acceptable) ⚠️

### Testing

Verified scenarios:
- ✅ CLI starts fresh service, stops on exit
- ✅ CLI with existing service, doesn't stop it
- ✅ TUI starts fresh service, stops on quit
- ✅ Multiple CLI calls in sequence
- ✅ Concurrent clients sharing service

### Future Enhancements

Could add:
- Graceful shutdown signal to service
- Service health checks
- Service restart on crash
- Configuration for service port
- Daemon mode for persistent service
- Service logs location option

### References

- [ADR-011: Migration to gRPC Architecture](#adr-011-migration-to-grpc-architecture)
- Implementation PR: Added service lifecycle management to roxid binary

---

## ADR-013: TUI Async Execution Model Fix

**Date:** 2025-11-03

**Status:** Accepted

### Context

After implementing auto-start (ADR-012), the TUI had a critical runtime error when executing pipelines:

```
Cannot start a runtime from within a runtime. This happens because a function 
(like `block_on`) attempted to block the current thread while the thread is 
being used to drive asynchronous tasks.
```

**Root cause**: The TUI was already running inside tokio's async runtime. When pressing Enter to execute a pipeline, the event handler tried to call `block_on()` on an async function, which tokio explicitly forbids.

### Decision

**Changed TUI execution model from blocking to event-driven:**

Instead of:
```rust
KeyCode::Enter => {
    let rt = tokio::runtime::Handle::current();
    rt.block_on(async { self.execute_selected_pipeline().await });
}
```

Use a pending execution flag:
```rust
KeyCode::Enter => {
    self.request_execute_pipeline(); // Set flag, non-blocking
}
```

Then handle in main loop:
```rust
pub async fn run(&mut self, terminal: DefaultTerminal) -> Result<()> {
    while !self.should_quit {
        terminal.draw(|frame| ui::render(self, frame))?;
        self.handle_events()?; // Synchronous event handling
        
        // Handle async operations in proper context
        if self.pending_execution {
            self.pending_execution = false;
            self.execute_selected_pipeline().await?;
        }
        
        self.process_execution_events().await;
    }
    Ok(())
}
```

### Implementation

**Added to `App` struct:**
- `pending_execution: bool` field
- `request_execute_pipeline()` method to set flag

**Modified main loop:**
- Check flag each iteration
- Execute pipeline in async context when flagged
- Reset flag after execution

**Updated event handler:**
- Removed `block_on()` call
- Just sets flag synchronously

### Consequences

#### Positive

- **Fixes runtime panic**: No more nested runtime error
- **Proper async**: All async code runs in correct context
- **Responsive UI**: Events processed without blocking
- **Clean separation**: Sync event handling, async execution
- **More idiomatic**: Follows tokio best practices

#### Negative

- **Slight delay**: Pipeline starts next loop iteration (negligible)
- **More state**: Additional field to track pending execution
- **Indirect flow**: Execution not immediate from event handler

#### Neutral

- One extra loop iteration before execution
- Flag-based state machine pattern
- Could extend for other async operations

### Pattern

This establishes a pattern for handling async operations in the TUI:

1. **Event handler**: Set flags/enqueue operations (sync)
2. **Main loop**: Process flagged operations (async)
3. **Render**: Pure function, no side effects (sync)

Can be extended for:
- Pipeline cancellation requests
- Service restart requests  
- File reload operations
- Any async operation triggered by user input

### Testing

Verified:
- ✅ TUI starts without errors
- ✅ Arrow keys navigate pipelines
- ✅ Enter key executes pipeline (no panic)
- ✅ Pipeline output streams correctly
- ✅ Can execute multiple pipelines in session
- ✅ Quit works properly with service cleanup

### References

- [Tokio Runtime Documentation](https://docs.rs/tokio/latest/tokio/runtime/)
- [Nested Runtime Error](https://docs.rs/tokio/latest/tokio/runtime/struct.Runtime.html#method.block_on)
- [ADR-012: Automatic Service Lifecycle Management](#adr-012-automatic-service-lifecycle-management)

---
