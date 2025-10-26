# Pipeline Execution System

The service layer includes a complete YAML pipeline execution system for running automated tasks locally.

## Features

- **YAML-based pipeline definitions** - Simple, declarative syntax
- **Command and shell script execution** - Run single commands or multi-line scripts
- **Environment variable support** - Pipeline-level and step-level env vars
- **Real-time progress reporting** - Live updates via async channels
- **Error handling** - Continue on error support for non-critical steps
- **Cross-platform** - Works on Linux, macOS, and Windows

## Pipeline YAML Format

```yaml
name: my-pipeline
description: Optional description
env:
  GLOBAL_VAR: value

steps:
  - name: Step name
    command: echo "Hello World"
    
  - name: Multi-line script
    shell:
      script: |
        echo "Line 1"
        echo "Line 2"
    env:
      STEP_VAR: value
    continue_on_error: true
```

## Usage

### Basic Example

```rust
use service::pipeline::{
    ExecutionContext, PipelineExecutor, PipelineParser,
};

#[tokio::main]
async fn main() -> Result<()> {
    // Parse pipeline from file
    let pipeline = PipelineParser::from_file("pipeline.yaml")?;
    
    // Create execution context
    let context = ExecutionContext::new(
        pipeline.name.clone(),
        std::env::current_dir()?.to_string_lossy().to_string()
    );
    
    // Execute pipeline
    let executor = PipelineExecutor::new(context);
    let results = executor.execute(pipeline, None).await;
    
    // Check results
    for result in results {
        println!("{}: {:?}", result.step_name, result.status);
    }
    
    Ok(())
}
```

### With Progress Reporting

```rust
use service::pipeline::{ExecutionEvent, PipelineExecutor};
use tokio::sync::mpsc;

let (tx, mut rx) = mpsc::unbounded_channel();

// Spawn executor
let handle = tokio::spawn(async move {
    executor.execute(pipeline, Some(tx)).await
});

// Monitor progress
while let Some(event) = rx.recv().await {
    match event {
        ExecutionEvent::StepStarted { step_name, .. } => {
            println!("Running: {}", step_name);
        }
        ExecutionEvent::StepOutput { output, .. } => {
            println!("  {}", output);
        }
        ExecutionEvent::StepCompleted { result, .. } => {
            println!("Completed: {:?}", result.status);
        }
        _ => {}
    }
}

let results = handle.await?;
```

## CLI Tool

A command-line tool is included for testing pipelines:

```bash
cargo run --bin pipeline-cli example-pipeline.yaml
```

## Example Pipelines

### Simple Test Pipeline

```yaml
name: quick-test
steps:
  - name: Check version
    command: rustc --version
  - name: Run tests
    command: cargo test
```

### Build Pipeline

```yaml
name: build-pipeline
env:
  RUST_BACKTRACE: "1"

steps:
  - name: Format check
    command: cargo fmt --check
    continue_on_error: true
    
  - name: Build
    command: cargo build --all
    
  - name: Test
    command: cargo test --all
```

## Architecture

The pipeline system consists of:

- **Parser** (`pipeline/parser.rs`) - Parses YAML into Rust structs
- **Executor** (`pipeline/executor.rs`) - Orchestrates step execution
- **Runners** (`pipeline/runners/`) - Execute different step types
  - `shell.rs` - Runs shell commands and scripts
- **Models** (`pipeline/models.rs`) - Data structures for pipelines, steps, results

## Integration with TUI

The pipeline system is designed to integrate with the TUI layer:

1. **Real-time updates** - Progress events can drive UI updates
2. **Step status display** - Show running/completed/failed steps
3. **Log streaming** - Display command output in real-time
4. **Interactive controls** - Pause/cancel/retry operations

Example integration in TUI:

```rust
// In app.rs
pub struct App {
    pipeline_results: Vec<StepResult>,
    current_step: Option<String>,
    logs: Vec<String>,
}

// Handle execution events
match event {
    ExecutionEvent::StepStarted { step_name, .. } => {
        app.current_step = Some(step_name);
    }
    ExecutionEvent::StepOutput { output, .. } => {
        app.logs.push(output);
    }
    ExecutionEvent::StepCompleted { result, .. } => {
        app.pipeline_results.push(result);
    }
    _ => {}
}
```

## Error Handling

Pipeline execution handles errors at multiple levels:

1. **Parse errors** - Invalid YAML or missing required fields
2. **Execution errors** - Command not found, permission denied, etc.
3. **Step failures** - Non-zero exit codes

Steps can continue on error with `continue_on_error: true`.

## Future Enhancements

Potential features to add:

- **Parallel execution** - Run independent steps concurrently
- **Conditional steps** - Skip steps based on conditions
- **Docker support** - Run steps in containers
- **Artifacts** - Save/restore step outputs
- **Matrix builds** - Run pipeline with different configurations
- **Step dependencies** - Explicit dependency graph
- **Timeout support** - Kill steps that run too long
- **Retry logic** - Automatically retry failed steps
