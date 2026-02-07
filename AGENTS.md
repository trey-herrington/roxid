# AGENTS.md - Coding Agent Instructions for Roxid

## Project Overview

Roxid is a Rust workspace (resolver v2) for Azure DevOps pipeline emulation. Three crates:

- **pipeline-service** (v0.8.0): Core library - pipeline parsing, expression evaluation, execution engine, runners, task management, testing, and GitHub Actions workflow support
- **roxid-tui** (v0.8.0): Terminal UI using Ratatui/Crossterm (library + standalone binary)
- **roxid-cli** (v0.8.0): CLI entry point (`roxid` binary, clap-based). Default subcommand launches TUI.

Dependency graph: `roxid-cli -> roxid-tui -> pipeline-service`

## Build/Test/Lint Commands

```bash
cargo build                    # Build entire workspace
cargo build -p pipeline-service # Build one crate
cargo build --release --bin roxid # Release binary
cargo check                    # Type-check only (faster)

cargo test                     # All tests, all crates
cargo test -p pipeline-service # Tests in one crate
cargo test -p pipeline-service test_parse_simple_pipeline  # Single test by name
cargo test -p pipeline-service parser::tests               # Tests in a module
cargo test -- --nocapture      # Show stdout/stderr from tests

cargo fmt                      # Format all code
cargo fmt --check              # CI format check
cargo clippy                   # Lint
cargo clippy -- -D warnings    # Lint, fail on warnings (CI mode)
```

No `rustfmt.toml` or `clippy.toml` -- default Rust formatting rules apply.

## Code Style

### Import Organization

Three-tier grouping separated by blank lines: (1) local crate, (2) std, (3) external crates.

```rust
use crate::execution::context::RuntimeContext;
use crate::parser::models::{Pipeline, Step, StepResult};

use std::collections::HashMap;
use std::path::PathBuf;

use tokio::sync::Semaphore;
```

Note: the TUI crate sometimes groups `pipeline_service::` imports as a fourth tier after external crates.

### Naming Conventions

| Element       | Convention           | Examples                                  |
|---------------|----------------------|-------------------------------------------|
| Files/modules | snake_case           | `shell.rs`, `models.rs`                   |
| Types/Enums   | PascalCase           | `Pipeline`, `ServiceError`, `ShellConfig` |
| Functions     | snake_case           | `execute_step`, `render_header`            |
| Constants     | SCREAMING_SNAKE_CASE | `DEFAULT_TIMEOUT`                          |

**Type suffixes**: `*Error`, `*Result`, `*Context`, `*Config`, `*State`, `*Progress`
**Function prefixes**: `new`, `with_*`, `from_*`, `execute_*`, `eval_*`, `render_*`, `process_*`

### Type Aliases

```rust
pub type ServiceResult<T> = Result<T, ServiceError>;
pub type ParseResult<T> = Result<T, ParseError>;
pub type ProgressSender = mpsc::UnboundedSender<ExecutionEvent>;
```

### Error Handling

Two error patterns coexist:

1. **Enum with `From` impls** (top-level `ServiceError`): manual `Display`, `Error`, and `From` implementations.
2. **Struct with builder methods** (parser `ParseError`): rich context with line/column, suggestions, and error kinds. Uses `with_*` builder methods like `with_kind()`, `with_suggestion()`, `with_context()`.

Common idioms: `?` operator with `.map_err()`, `ok_or_else`, `let _ = tx.send(...)` for fire-and-forget channels, `unwrap_or_default()`. No panicking `unwrap()` in production code.

### Derives and Serde

Models use heavy derive and serde annotations:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Pipeline { ... }
```

Common attributes: `#[serde(default)]`, `#[serde(deserialize_with = "...")]`, `#[serde(rename_all = "camelCase")]`, `#[serde(untagged)]`, `#[serde(flatten)]`.

### Module Organization

Re-export public items in `mod.rs` and `lib.rs`:

```rust
pub mod context;
pub mod events;
pub mod executor;

pub use context::RuntimeContext;
pub use executor::{ExecutionResult, PipelineExecutor};
```

`lib.rs` aggregates all public types with comment-separated sections for each module.

### Documentation and Comments

- File-level `//` comment headers (not `///`): `// Pipeline Executor` / `// Orchestrates pipeline execution with DAG-based scheduling`
- `///` doc comments on public structs, enums, and their fields
- `// ===...===` section separators in long files (see `app.rs`)
- Inline `//` for TODOs and implementation notes

### Test Organization

All tests are inline `#[cfg(test)]` modules at the bottom of each file:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn make_context() -> ExpressionContext { ... }  // helper factories

    #[test]
    fn test_parse_simple_pipeline() { ... }

    #[tokio::test]
    async fn test_execute_pipeline() { ... }
}
```

Tests use `test_` prefix naming, `assert!`/`assert_eq!`/`matches!` assertions, and helper factory functions. Async tests use `#[tokio::test]`.

### Constructor and Builder Patterns

```rust
impl ExecutorConfig {
    pub fn new() -> Self { Self { max_parallel_stages: 0, ... } }
    pub fn with_timeout(mut self, timeout: Duration) -> Self { self.timeout = Some(timeout); self }
}
```

Also uses `from_*()` named constructors: `ParseError::from_yaml_error(...)`.

### Platform-Specific Code

Uses runtime checks (`cfg!`), not compile-time `#[cfg]`:

```rust
if cfg!(target_os = "windows") {
    ("cmd", &["/C"])
} else {
    ("sh", &["-c"])
}
```

## Project Structure

```
roxid/
├── Cargo.toml                    # Workspace manifest
├── pipeline-service/src/
│   ├── lib.rs                    # Public API re-exports
│   ├── error.rs                  # ServiceError, ServiceResult
│   ├── parser/
│   │   ├── azure.rs              # YAML parser (AzureParser)
│   │   ├── error.rs              # ParseError, ValidationError (rich errors)
│   │   ├── models.rs             # Pipeline, Stage, Job, Step, Value, etc.
│   │   └── template.rs           # Template resolution (TemplateEngine)
│   ├── expression/
│   │   ├── evaluator.rs          # ExpressionEngine, ExpressionContext
│   │   ├── functions.rs          # Built-in functions
│   │   ├── lexer.rs              # Tokenizer
│   │   └── parser.rs             # Expression AST parser
│   ├── execution/
│   │   ├── executor.rs           # PipelineExecutor, DAG-based scheduling
│   │   ├── graph.rs              # ExecutionGraph, DAG builder
│   │   ├── matrix.rs             # MatrixExpander
│   │   ├── context.rs            # RuntimeContext
│   │   └── events.rs             # ExecutionEvent, channel types
│   ├── runners/
│   │   ├── shell.rs              # ShellRunner (sh/bash/pwsh)
│   │   ├── task.rs               # TaskRunner (Azure DevOps tasks)
│   │   └── container.rs          # ContainerRunner (Docker)
│   ├── tasks/
│   │   ├── cache.rs              # TaskCache management
│   │   └── manifest.rs           # task.json parser
│   ├── testing/
│   │   ├── runner.rs             # TestRunner
│   │   ├── assertions.rs         # Assertion logic
│   │   ├── parser.rs             # Test file parser
│   │   └── reporter.rs           # JUnit/TAP/terminal output
│   └── workflow/
│       ├── models.rs             # GitHub Actions Workflow types
│       └── parser.rs             # WorkflowParser
├── roxid-tui/src/
│   ├── lib.rs, main.rs           # TUI entry points
│   ├── app.rs                    # Application state machine
│   ├── events.rs                 # Keyboard event handling
│   ├── ui.rs                     # UI module root
│   └── ui/                       # UI components
│       ├── layout.rs             # Layout system
│       ├── components.rs         # Header, footer, status helpers
│       ├── pipeline_list.rs      # Pipeline browser
│       ├── pipeline_tree.rs      # Expandable tree view
│       ├── execution.rs          # Real-time execution display
│       ├── log_viewer.rs         # Scrollable log viewer
│       └── test_results.rs       # Test results panel
└── roxid-cli/src/
    ├── main.rs                   # CLI entry point (clap)
    ├── output.rs                 # Terminal formatting helpers
    └── commands/                 # run, test, validate, task subcommands
```

## Key Dependencies

| Crate        | Used by              | Purpose                       |
|--------------|----------------------|-------------------------------|
| tokio 1.0    | all                  | Async runtime (full features) |
| serde + yaml | pipeline-service,tui | YAML serialization            |
| thiserror    | pipeline-service     | Error derive macros           |
| async-trait  | pipeline-service     | Async trait support           |
| clap 4       | roxid-cli            | CLI argument parsing          |
| ratatui 0.29 | roxid-tui            | Terminal UI framework         |
| crossterm    | roxid-tui            | Terminal backend              |
| color-eyre   | roxid-cli, roxid-tui | Error reporting               |
| tempfile     | pipeline-service dev | Temp files in tests           |
