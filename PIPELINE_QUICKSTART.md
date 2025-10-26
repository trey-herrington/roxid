# Pipeline Execution - Quick Start

## What is it?

A YAML-based pipeline execution system built into your Rust application that allows you to run automated tasks locally with real-time progress reporting.

## Quick Example

Create a file `my-pipeline.yaml`:

```yaml
name: my-first-pipeline
steps:
  - name: Hello World
    command: echo "Hello from pipeline!"
  
  - name: Show Environment
    shell:
      script: |
        echo "User: $USER"
        echo "Path: $(pwd)"
```

Run it:

```bash
cargo run --bin pipeline-cli my-pipeline.yaml
```

## Key Features

✅ **Simple YAML syntax** - Easy to write and read  
✅ **Real-time output** - See command output as it happens  
✅ **Environment variables** - Set env vars at pipeline or step level  
✅ **Error handling** - Continue on error or stop execution  
✅ **Cross-platform** - Works on Linux, macOS, and Windows  
✅ **Async execution** - Built on Tokio for performance  

## Next Steps

- See [PIPELINE.md](PIPELINE.md) for complete documentation
- Check out example pipelines:
  - `example-pipeline.yaml` - Basic examples
  - `rust-build-pipeline.yaml` - Rust project CI/CD
  - `advanced-pipeline.yaml` - Advanced features

## Integration

The pipeline system is designed to integrate with the TUI:

```rust
use service::pipeline::{PipelineExecutor, ExecutionEvent};

// Execute and monitor progress
let (tx, mut rx) = mpsc::unbounded_channel();
tokio::spawn(async move {
    executor.execute(pipeline, Some(tx)).await
});

// Update UI based on events
while let Some(event) = rx.recv().await {
    match event {
        ExecutionEvent::StepOutput { output, .. } => {
            // Update TUI with output
        }
        _ => {}
    }
}
```

## Common Use Cases

- **Build automation** - Compile, test, and package your projects
- **Development workflows** - Run linters, formatters, and tests
- **Deployment** - Automate deployment steps
- **CI/CD locally** - Test your CI pipeline before pushing
- **Task orchestration** - Chain multiple commands together
