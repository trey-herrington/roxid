# AGENTS.md - Coding Agent Instructions for Roxid

This document provides guidelines for AI coding agents working in the Roxid codebase.

## Project Overview

Roxid is a Rust workspace project for pipeline execution with a TUI interface. It consists of three crates:

- **pipeline-service**: gRPC service for pipeline parsing and execution (library + binary)
- **roxid-tui**: Terminal UI client using Ratatui
- **roxid-cli**: CLI entry point (main `roxid` binary)

## Build/Test/Lint Commands

**Prerequisites**: Requires `protoc` (protobuf compiler) installed.

```bash
cargo build                    # Build workspace
cargo build -p pipeline-service # Build specific package
cargo build --release --bin roxid # Build release binary
cargo check                    # Check without building (faster)

cargo test                     # Run all tests
cargo test -p pipeline-service # Test specific package
cargo test -p pipeline-service test_parse_simple_pipeline # Single test
cargo test -p pipeline-service parser::tests # Tests in module
cargo test -- --nocapture      # Tests with output

cargo fmt                      # Format code
cargo fmt --check              # Check formatting
cargo clippy                   # Lint
cargo clippy -- -D warnings    # Lint (strict)
```

## Code Style Guidelines

### Import Organization

Imports follow a three-tier grouping pattern with blank lines between groups:

```rust
// 1. Local crate imports
use crate::pipeline::models::{Pipeline, Step, StepResult};
use crate::ServiceResult;

// 2. Standard library
use std::fs;
use std::path::Path;

// 3. External crates
use serde::{Deserialize, Serialize};
```

### Naming Conventions

| Element | Convention | Examples |
|---------|------------|----------|
| Files | snake_case | `shell.rs`, `models.rs` |
| Types/Structs/Enums | PascalCase | `Pipeline`, `ServiceError` |
| Functions/Methods | snake_case | `execute_step`, `render_header` |
| Constants | SCREAMING_SNAKE_CASE | `DEFAULT_TIMEOUT` |

**Type suffix conventions**: `*Error`, `*Result`, `*State`
**Function prefixes**: `new`, `with_*`, `render_*`, `process_*`

### Type Definitions

```rust
pub type ServiceResult<T> = Result<T, ServiceError>;
pub type ProgressSender = mpsc::UnboundedSender<ExecutionEvent>;
```

### Error Handling

Custom error enum with `From` implementations:

```rust
#[derive(Debug)]
pub enum ServiceError {
    NotFound(String),
    IoError(std::io::Error),
}

impl From<std::io::Error> for ServiceError {
    fn from(err: std::io::Error) -> Self {
        ServiceError::IoError(err)
    }
}
```

Patterns: `?` operator, `ok_or_else`, `let _ = tx.send(...)`, `unwrap_or_default()`

### Module Organization

Re-export public items in mod.rs/lib.rs:

```rust
pub mod executor;
pub mod models;
pub use executor::{ExecutionEvent, PipelineExecutor};
pub use models::{Pipeline, Step};
```

### Test Organization

Tests are inline using `#[cfg(test)]` modules with `use super::*;`.

## Project Structure

```
roxid/
├── Cargo.toml                 # Workspace manifest
├── pipeline-service/
│   ├── build.rs               # Proto compilation
│   ├── proto/pipeline.proto   # gRPC service definition
│   └── src/
│       ├── lib.rs
│       ├── error.rs           # ServiceError, ServiceResult
│       ├── grpc.rs            # Proto type conversions
│       └── pipeline/
│           ├── models.rs      # Pipeline, Step, StepResult
│           ├── parser.rs      # YAML parsing (with tests)
│           ├── executor.rs    # Pipeline execution engine
│           └── runners/shell.rs
├── roxid-tui/
│   └── src/
│       ├── app.rs             # Application state, gRPC client
│       ├── events.rs          # Keyboard event handling
│       └── ui/                # UI components
└── roxid-cli/
    └── src/main.rs            # CLI entry point
```

## Key Dependencies

- **ratatui** 0.29 / **crossterm** 0.29 - TUI framework
- **tonic** 0.12 / **prost** 0.13 - gRPC
- **tokio** 1.0 - Async runtime
- **serde** / **serde_yaml** - YAML serialization
- **color-eyre** 0.6 - Error handling

## Common Patterns

### Builder-style constructors

```rust
impl ExecutionContext {
    pub fn new(pipeline_name: String, working_dir: String) -> Self {
        Self { pipeline_name, env: HashMap::new(), working_dir }
    }

    pub fn with_env(mut self, env: HashMap<String, String>) -> Self {
        self.env = env;
        self
    }
}
```

### Let-else for early returns

```rust
let Some(rx) = &mut self.event_receiver else {
    return;
};
```

### Platform-specific code

```rust
if cfg!(target_os = "windows") {
    ("cmd".to_string(), vec!["/C".to_string(), cmd.clone()])
} else {
    ("sh".to_string(), vec!["-c".to_string(), cmd.clone()])
}
```
