# Quick Reference: Phase 1 Features

## New Pipeline Format

### Stages-based Pipeline
```yaml
name: my-pipeline
description: Multi-stage pipeline

stages:
  - stage: build
    display_name: Build Stage
    jobs:
      - job: build_linux
        display_name: Build on Linux
        env:
          PLATFORM: linux
        steps:
          - name: Compile
            command: cargo build

      - job: build_windows
        display_name: Build on Windows
        env:
          PLATFORM: windows
        steps:
          - name: Compile
            command: cargo build

  - stage: test
    display_name: Test Stage
    depends_on: [build]  # Runs after build stage
    jobs:
      - job: unit_tests
        steps:
          - name: Run tests
            command: cargo test

      - job: integration_tests
        depends_on: [unit_tests]  # Runs after unit_tests
        steps:
          - name: Run integration tests
            command: cargo test --test '*'
```

### Legacy Format (Still Supported)
```yaml
name: my-pipeline
steps:
  - name: Build
    command: cargo build
  - name: Test
    command: cargo test
```

## Key Concepts

### Stages
- Top-level organizational unit
- Run sequentially (one after another)
- Can depend on other stages via `depends_on`
- Contain one or more jobs

### Jobs
- Execution unit within a stage
- Run in **parallel** by default (within same stage)
- Can depend on other jobs via `depends_on`
- Contain one or more steps

### Steps
- Individual commands or scripts
- Run sequentially within a job
- Same as before (no changes)

## Execution Model

```
Pipeline
  ↓
Stage 1 (sequential)
  ↓
  ├─ Job A ─────┐
  ├─ Job B ─────┼─→ (parallel)
  └─ Job C ─────┘
  ↓
Stage 2 (depends_on: stage1)
  ↓
  ├─ Job D ─────┐
  └─ Job E ─────┘ (parallel)
```

## Dependency Examples

### Stage Dependencies
```yaml
stages:
  - stage: build
    jobs: [...]

  - stage: test
    depends_on: [build]  # Waits for build to complete
    jobs: [...]

  - stage: deploy
    depends_on: [build, test]  # Waits for both
    jobs: [...]
```

### Job Dependencies
```yaml
stages:
  - stage: test
    jobs:
      - job: unit_tests
        steps: [...]

      - job: integration_tests
        depends_on: [unit_tests]  # Runs after unit_tests
        steps: [...]

      - job: performance_tests
        depends_on: [unit_tests]  # Also runs after unit_tests
        steps: [...]
```

### Parallel Jobs (Default)
```yaml
stages:
  - stage: parallel_work
    jobs:
      - job: job1
        steps: [...]
      - job: job2
        steps: [...]
      - job: job3
        steps: [...]
    # All three jobs run simultaneously
```

## Environment Variables

### Pipeline Level
```yaml
name: my-pipeline
env:
  RUST_LOG: debug
  BUILD_TYPE: release

stages: [...]
```

### Job Level
```yaml
jobs:
  - job: build
    env:
      TARGET: x86_64-unknown-linux-gnu
    steps: [...]
```

### Step Level (Existing)
```yaml
steps:
  - name: Custom build
    command: cargo build
    env:
      RUSTFLAGS: "-C target-cpu=native"
```

## Error Handling

### Continue on Error (Step Level)
```yaml
steps:
  - name: Optional step
    command: cargo clippy
    continue_on_error: true  # Won't fail the job

  - name: Required step
    command: cargo build
```

### Failed Job Stops Stage
If any job in a stage fails, the entire stage is marked as failed and subsequent stages won't run (unless conditions are specified in Phase 2).

## Migration from Legacy

Your existing pipelines work without changes. To use new features:

```yaml
# Old (still works)
name: my-pipeline
steps:
  - name: Build
    command: cargo build

# New (with stages)
name: my-pipeline
stages:
  - stage: build
    jobs:
      - job: default
        steps:
          - name: Build
            command: cargo build
```

## API Usage

### Parsing
```rust
use pipeline_service::pipeline::PipelineParser;

let pipeline = PipelineParser::from_file("pipeline.yaml")?;
println!("Stages: {}", pipeline.stages.len());
println!("Is legacy: {}", pipeline.is_legacy());
```

### Execution
```rust
use pipeline_service::pipeline::{PipelineExecutor, ExecutionContext};

let context = ExecutionContext::new(
    pipeline.name.clone(),
    "/path/to/working/dir".to_string()
);

let executor = PipelineExecutor::new(context);
let results = executor.execute(pipeline, None).await;
```

### With Progress Events
```rust
let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

// Spawn executor
tokio::spawn(async move {
    executor.execute(pipeline, Some(tx)).await
});

// Monitor events
while let Some(event) = rx.recv().await {
    match event {
        ExecutionEvent::StageStarted { stage_name, .. } => {
            println!("Stage started: {}", stage_name);
        }
        ExecutionEvent::JobStarted { job_name, .. } => {
            println!("Job started: {}", job_name);
        }
        ExecutionEvent::StepCompleted { result, .. } => {
            println!("Step completed: {:?}", result.status);
        }
        _ => {}
    }
}
```

## Limitations (Phase 1)

❌ **Not yet implemented:**
- Condition evaluation (conditions are parsed but not evaluated)
- Matrix strategy (parsed but not executed)
- Pool selection (parsed but not used)
- Variable groups
- Templates
- Built-in tasks
- Container jobs

✅ **What works now:**
- Multi-stage pipelines
- Multiple jobs per stage
- Stage dependencies
- Job dependencies
- Parallel job execution
- All existing step types
- Environment variables
- Error handling

## Testing

Run the example:
```bash
cargo run --example test_stages
```

Run tests:
```bash
cargo test -p pipeline-service
```

Test your pipeline:
```bash
# Using the CLI (if service is running)
roxid run my-pipeline.yaml
```

## Files to Reference

- `PHASE1_COMPLETION.md` - Full implementation details
- `test-stages-pipeline.yaml` - Example multi-stage pipeline
- `test-parallel-jobs.yaml` - Example parallel execution
- `test-legacy-pipeline.yaml` - Backward compatibility example
- `pipeline-service/examples/test_stages.rs` - API usage example
