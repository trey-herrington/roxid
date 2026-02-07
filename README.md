# Roxid

A 100% Azure DevOps Pipelines compatible local execution environment with an interactive Terminal UI built with [Ratatui](https://ratatui.rs/).

Run actual `azure-pipelines.yml` files locally and write unit tests for pipeline logic.

## Table of Contents

- [Features](#features)
- [Quick Start](#quick-start)
- [Installation](#installation)
- [Usage](#usage)
- [Architecture](#architecture)
- [Pipeline Format](#pipeline-format)
- [Testing Pipelines](#testing-pipelines)
- [TUI Guide](#tui-guide)
- [Project Structure](#project-structure)
- [Resources](#resources)

## Features

### Azure DevOps Compatibility
- **Full YAML schema support**: Stages, jobs, steps, variables, parameters, resources, triggers
- **Expression engine**: All three expression types - `${{ }}` compile-time, `$[ ]` runtime, `$(var)` macro substitution
- **Built-in functions**: `eq`, `ne`, `contains`, `startsWith`, `format`, `join`, `replace`, `coalesce`, `iif`, status checks, and more
- **Template system**: Template resolution with `extends`, `${{ if }}`, `${{ each }}`, cross-repo templates, parameter validation
- **DAG execution**: Dependency-based stage/job ordering with `dependsOn`, parallel execution, and `maxParallel`
- **Matrix strategies**: Full matrix expansion with inline definitions
- **Condition evaluation**: Azure DevOps condition expressions on stages, jobs, and steps

### Runners
- **Shell runner**: `script`, `bash`, `pwsh`, `powershell` step types with real-time output streaming
- **Task runner**: Download and execute Azure DevOps tasks (e.g., `Bash@3`, `PowerShell@2`) from the marketplace
- **Container runner**: Docker-based container job execution with service containers, volume mounting, and port mapping

### Testing Framework
- **Test definitions**: YAML-based test suites (`roxid-test.yml`) with pipeline-level assertions
- **Assertions**: `pipeline_succeeded`, `step_succeeded`, `step_output_equals`, `step_ran_before`, `variable_equals`, and more
- **Multiple output formats**: JUnit XML, TAP, and terminal output
- **Test discovery**: Automatic discovery of `roxid-test.yml` files
- **Filtering**: Glob-based test name filtering with fail-fast support

### TUI Features
- **Pipeline discovery**: Automatically discovers pipeline YAML files in the current directory
- **Pipeline tree view**: Expandable stages, jobs, and steps with type indicators
- **Real-time execution**: Live progress bar, stage panel, and output panel
- **Log viewer**: Scrollable, searchable output with filtering
- **Test results panel**: Summary bar with pass/fail list
- **Variable editor**: Edit variables before execution
- **Cross-platform**: Works on Linux, macOS, and Windows

## Quick Start

```bash
# Launch TUI (default)
roxid

# Run a specific pipeline
roxid run azure-pipelines.yml

# Run with variable overrides
roxid run azure-pipelines.yml --var "buildConfiguration=Release"

# Run tests
roxid test

# Validate a pipeline
roxid validate azure-pipelines.yml
```

## Installation

### Option 1: From crates.io (Recommended)

```bash
cargo install roxid
roxid
```

### Option 2: Build from Source

```bash
git clone https://github.com/yourusername/roxid
cd roxid
cargo build --release
sudo cp target/release/roxid /usr/local/bin/
```

### Verify

```bash
roxid --help
```

### Uninstall

```bash
# If installed via cargo:
cargo uninstall roxid

# If installed manually:
sudo rm /usr/local/bin/roxid
```

## Usage

### CLI Commands

```bash
# Run a pipeline
roxid run azure-pipelines.yml
roxid run azure-pipelines.yml --var "foo=bar"
roxid run azure-pipelines.yml --stage Build

# Test pipelines
roxid test                           # Run all tests in roxid-test.yml
roxid test --filter "deploy*"        # Filter tests by name
roxid test --output junit            # JUnit XML output
roxid test --output tap              # TAP output

# Validate pipelines
roxid validate azure-pipelines.yml   # Check syntax and references
roxid validate --templates           # Validate template resolution

# TUI mode
roxid tui                            # Launch interactive TUI
roxid                                # Default: launches TUI

# Task management
roxid task list                      # List cached tasks
roxid task fetch Bash@3              # Pre-download a task
roxid task clear                     # Clear task cache
roxid task path                      # Show task cache path
```

### TUI Controls

| Key | Action |
|-----|--------|
| `↑/k` | Move up |
| `↓/j` | Move down |
| `Enter` | Execute/Expand |
| `Tab` | Switch panels |
| `v` | Edit variables |
| `t` | Run tests |
| `l` | View logs |
| `/` | Search |
| `q/Esc` | Back/Quit |

## Architecture

### Workspace Structure

Roxid is a Rust workspace with three crates that communicate via direct library calls:

```
roxid/
├── Cargo.toml              # Workspace manifest (resolver v2)
├── pipeline-service/       # Core library (v0.8.0)
│   └── src/
│       ├── lib.rs          # Public API re-exports
│       ├── error.rs        # ServiceError, ServiceResult
│       ├── parser/         # Azure DevOps YAML parser, models, templates
│       ├── expression/     # Expression engine (lexer, parser, evaluator, functions)
│       ├── execution/      # DAG builder, executor, matrix, context, events
│       ├── runners/        # Shell, task, and container runners
│       ├── tasks/          # Task cache and manifest parsing
│       ├── testing/        # Test runner, assertions, parser, reporter
│       └── workflow/       # GitHub Actions workflow support (future)
├── roxid-tui/              # Terminal UI (v0.8.0, library + binary)
│   └── src/
│       ├── lib.rs, main.rs # Entry points
│       ├── app.rs          # Application state machine
│       ├── events.rs       # Keyboard event handling
│       ├── ui.rs           # UI module root
│       └── ui/             # UI components
│           ├── layout.rs, components.rs
│           ├── pipeline_list.rs, pipeline_tree.rs
│           ├── execution.rs, log_viewer.rs
│           └── test_results.rs
└── roxid-cli/              # CLI entry point (v0.8.0, `roxid` binary, clap-based)
    └── src/
        ├── main.rs         # CLI entry point
        ├── output.rs       # Terminal formatting helpers
        └── commands/       # run, test, validate, task subcommands
```

### Dependency Graph

```
roxid-cli ──→ roxid-tui ──→ pipeline-service
     └─────────────────────────→ (also depends directly)
```

All communication is via direct Rust library calls. There is no RPC, gRPC, or network protocol involved.

### Architecture Layers

#### 1. pipeline-service (Core Library)
- **Purpose**: Core pipeline parsing, expression evaluation, execution, and testing
- **Type**: Library crate
- **Components**:
  - **Parser**: Azure DevOps YAML parser with template resolution and validation
  - **Expression Engine**: Full `${{ }}`, `$[ ]`, `$(var)` support with built-in functions
  - **Execution Engine**: DAG-based scheduling with parallel execution and matrix expansion
  - **Runners**: Shell, task, and container runners for step execution
  - **Task Cache**: Download and cache Azure DevOps tasks from the marketplace
  - **Testing Framework**: Test definitions, assertions, runner, and reporters

#### 2. roxid-tui (Terminal UI)
- **Purpose**: Interactive terminal interface for pipeline management
- **Type**: Library + binary crate
- **Dependencies**: Direct library calls to `pipeline-service`
- **States**: PipelineList, PipelineDetail, ExecutingPipeline, ExecutionLog, TestResults, VariableEditor

#### 3. roxid-cli (CLI)
- **Purpose**: Command-line interface for pipeline execution
- **Type**: Binary crate (`roxid`)
- **Dependencies**: Direct library calls to `pipeline-service` and `roxid-tui`
- **Framework**: clap 4 with derive macros

### Communication

```
┌─────────────┐                        ┌──────────────────┐
│  roxid-cli  │──── library calls ────→│ pipeline-service │
│  (binary)   │                        │  (core library)  │
└──────┬──────┘                        └──────────────────┘
       │                                        ▲
       │ library calls                          │
       ▼                                        │
┌─────────────┐──── library calls ──────────────┘
│  roxid-tui  │
│  (lib+bin)  │
└─────────────┘
```

### Application Flow

1. **Initialization**: Discovers pipeline YAML files using `AzureParser::parse_file()` and `normalize_pipeline()`
2. **Pipeline List**: Browse discovered pipelines with stage/job/step counts
3. **Pipeline Detail**: Expandable tree view of stages, jobs, and steps
4. **Execution**: Spawns tokio task with progress channel (`ExecutionEvent` streaming) for real-time updates
5. **Results**: View logs, test results, and execution status

The TUI uses a `pending_execution`/`pending_test_run` flag pattern to bridge synchronous keyboard handlers to the async main loop.

## Pipeline Format

Roxid uses Azure DevOps pipeline YAML format:

```yaml
trigger:
  - main

pool:
  vmImage: ubuntu-latest

variables:
  buildConfiguration: Release

stages:
  - stage: Build
    jobs:
      - job: BuildJob
        steps:
          - script: echo "Building $(buildConfiguration)"
            displayName: Build

          - bash: |
              echo "Running tests"
              cargo test
            displayName: Test

  - stage: Deploy
    dependsOn: Build
    condition: succeeded()
    jobs:
      - job: DeployJob
        steps:
          - task: Bash@3
            inputs:
              targetType: inline
              script: echo "Deploying..."
```

### Supported Step Types

- `script` - Default shell (sh on Unix, cmd on Windows)
- `bash` - Bash scripts
- `pwsh` - PowerShell Core scripts
- `powershell` - Windows PowerShell scripts
- `checkout` - Repository checkout
- `task` - Azure DevOps marketplace tasks (e.g., `Bash@3`)
- `template` - Template reference with parameters
- `download` / `publish` - Artifact operations

### Template Example

```yaml
# templates/build-steps.yml
parameters:
  - name: configuration
    type: string
    default: Debug

steps:
  - script: echo "Building ${{ parameters.configuration }}"
```

```yaml
# azure-pipelines.yml
stages:
  - stage: Build
    jobs:
      - job: BuildJob
        steps:
          - template: templates/build-steps.yml
            parameters:
              configuration: Release
```

## Testing Pipelines

Create a `roxid-test.yml` file:

```yaml
name: Pipeline Tests
defaults:
  working_dir: .

tests:
  - name: Build succeeds
    pipeline: azure-pipelines.yml
    variables:
      buildConfiguration: Debug
    assertions:
      - pipeline_succeeded
      - step_succeeded: Build

  - name: Deploy runs after build
    pipeline: azure-pipelines.yml
    assertions:
      - step_ran_before:
          first: Build
          second: Deploy

  - name: Output contains expected text
    pipeline: azure-pipelines.yml
    assertions:
      - step_output_contains:
          step: Build
          contains: "Building"
```

### Available Assertions

| Assertion | Description |
|-----------|-------------|
| `pipeline_succeeded` | Pipeline completed successfully |
| `pipeline_failed` | Pipeline failed |
| `step_succeeded: <name>` | Named step succeeded |
| `step_failed: <name>` | Named step failed |
| `step_skipped: <name>` | Named step was skipped |
| `job_succeeded: <name>` | Named job succeeded |
| `stage_succeeded: <name>` | Named stage succeeded |
| `step_output_equals` | Step output matches expected value |
| `step_output_contains` | Step output contains expected text |
| `step_ran_before` | Verify execution ordering |
| `variable_equals` | Variable has expected value |
| `variable_contains` | Variable contains expected text |

### Running Tests

```bash
roxid test                        # Run all tests
roxid test --filter "deploy*"     # Filter by name
roxid test --output junit         # JUnit XML for CI
roxid test --output tap           # TAP format
```

## Project Structure

```
roxid/
├── Cargo.toml                    # Workspace manifest
├── pipeline-service/src/
│   ├── lib.rs                    # Public API re-exports
│   ├── error.rs                  # ServiceError, ServiceResult
│   ├── parser/
│   │   ├── mod.rs
│   │   ├── azure.rs              # Azure DevOps YAML parser (AzureParser)
│   │   ├── error.rs              # ParseError, ValidationError (rich errors)
│   │   ├── models.rs             # Pipeline, Stage, Job, Step, Value, etc.
│   │   └── template.rs           # Template resolution (TemplateEngine)
│   ├── expression/
│   │   ├── mod.rs
│   │   ├── evaluator.rs          # ExpressionEngine, ExpressionContext
│   │   ├── functions.rs          # Built-in functions
│   │   ├── lexer.rs              # Tokenizer
│   │   └── parser.rs             # Expression AST parser
│   ├── execution/
│   │   ├── mod.rs
│   │   ├── executor.rs           # PipelineExecutor, DAG-based scheduling
│   │   ├── graph.rs              # ExecutionGraph, DAG builder
│   │   ├── matrix.rs             # MatrixExpander
│   │   ├── context.rs            # RuntimeContext
│   │   └── events.rs             # ExecutionEvent, channel types
│   ├── runners/
│   │   ├── mod.rs
│   │   ├── shell.rs              # ShellRunner (sh/bash/pwsh)
│   │   ├── task.rs               # TaskRunner (Azure DevOps tasks)
│   │   └── container.rs          # ContainerRunner (Docker)
│   ├── tasks/
│   │   ├── mod.rs
│   │   ├── cache.rs              # TaskCache management
│   │   └── manifest.rs           # task.json parser
│   ├── testing/
│   │   ├── mod.rs
│   │   ├── runner.rs             # TestRunner
│   │   ├── assertions.rs         # Assertion logic
│   │   ├── parser.rs             # Test file parser
│   │   └── reporter.rs           # JUnit/TAP/terminal output
│   └── workflow/
│       ├── mod.rs
│       ├── models.rs             # GitHub Actions Workflow types
│       └── parser.rs             # WorkflowParser
├── roxid-tui/src/
│   ├── lib.rs, main.rs           # TUI entry points
│   ├── app.rs                    # Application state machine (6 states)
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
    └── commands/                 # Subcommands
        ├── mod.rs
        ├── run.rs                # roxid run
        ├── test.rs               # roxid test
        ├── validate.rs           # roxid validate
        └── task.rs               # roxid task
```

## Key Dependencies

| Crate | Used by | Purpose |
|-------|---------|---------|
| tokio 1.0 | all | Async runtime (full features) |
| serde + serde_yaml | pipeline-service, roxid-tui | YAML serialization |
| thiserror | pipeline-service | Error derive macros |
| async-trait | pipeline-service | Async trait support |
| clap 4 | roxid-cli | CLI argument parsing |
| ratatui 0.29 | roxid-tui | Terminal UI framework |
| crossterm 0.29 | roxid-tui | Terminal backend |
| color-eyre | roxid-cli, roxid-tui | Error reporting |
| dirs | pipeline-service | Platform directory paths |
| which | pipeline-service | Executable lookup |
| tempfile | pipeline-service (dev) | Temp files in tests |

## Resources

- [Azure DevOps YAML Schema](https://docs.microsoft.com/en-us/azure/devops/pipelines/yaml-schema)
- [Azure DevOps Expressions](https://docs.microsoft.com/en-us/azure/devops/pipelines/process/expressions)
- [Azure DevOps Templates](https://docs.microsoft.com/en-us/azure/devops/pipelines/process/templates)
- [Ratatui Documentation](https://docs.rs/ratatui)
- [Ratatui Website](https://ratatui.rs/)
- [Crossterm Documentation](https://docs.rs/crossterm)

## License

Licensed under either of Apache License 2.0 or MIT license at your option.
