# TUI Usage Guide

## Overview

The TUI (Terminal User Interface) application provides an interactive way to browse, select, and execute pipeline YAML files from your current directory.

## Starting the TUI

```bash
# Navigate to a directory containing pipeline YAML files
cd /path/to/your/pipelines

# Start the TUI
cargo run --bin tui
```

## Interface Overview

### Pipeline List Screen

```
┌─────────────────────────────────────────────┐
│  Pipeline Runner                            │
└─────────────────────────────────────────────┘
┌─ Available Pipelines ──────────────────────┐
│  → example-pipeline - A simple example     │
│    rust-build-pipeline - Build Rust project│
│    advanced-pipeline - Complex workflow    │
└─────────────────────────────────────────────┘
┌─ Help ──────────────────────────────────────┐
│ ↑/↓: Navigate | Enter: Execute | q: Quit   │
└─────────────────────────────────────────────┘
```

**Features:**
- Lists all `.yaml` and `.yml` files in the current directory
- Shows pipeline name and description
- Highlights selected pipeline
- Empty state message if no pipelines found

### Pipeline Execution Screen

```
┌─────────────────────────────────────────────┐
│  Executing: example-pipeline                │
└─────────────────────────────────────────────┘
┌─ Progress ──────────────────────────────────┐
│ ████████████░░░░░░░░  Step 3/5             │
└─────────────────────────────────────────────┘
┌─ Output ────────────────────────────────────┐
│ Pipeline 'example-pipeline' started         │
│                                             │
│ [Step 1/5] Check Rust version              │
│   rustc 1.70.0 (90c541806 2023-05-31)      │
│   ✓ Completed in 0.05s                     │
│                                             │
│ [Step 2/5] List files                      │
│   total 64                                  │
│   drwxr-xr-x  12 user  staff  384 Oct 27   │
│   ✓ Completed in 0.02s                     │
│                                             │
│ [Step 3/5] Multi-line script               │
│   Starting multi-line script                │
│   Current directory: /path/to/project       │
│   ...                                       │
└─────────────────────────────────────────────┘
┌─ Help ──────────────────────────────────────┐
│ Pipeline executing...                       │
└─────────────────────────────────────────────┘
```

**Features:**
- Real-time progress bar showing current step
- Live output streaming as commands execute
- Color-coded status indicators:
  - ✓ Green for successful steps
  - ✗ Red for failed steps
  - Yellow for step headers
- Auto-scrolling output (shows most recent lines)
- Execution time per step

## Keyboard Controls

### Pipeline List Screen
| Key | Action |
|-----|--------|
| `↑` or `k` | Move selection up |
| `↓` or `j` | Move selection down |
| `Enter` | Execute selected pipeline |
| `q` or `Esc` | Quit application |

### Pipeline Execution Screen
| Key | Action |
|-----|--------|
| `q` or `Esc` | Return to pipeline list (only when pipeline completes) |

## Pipeline Discovery

The TUI automatically discovers pipeline files by:
1. Scanning the current working directory
2. Finding all files with `.yaml` or `.yml` extensions
3. Parsing each file to validate it's a valid pipeline
4. Extracting the pipeline name and description
5. Sorting pipelines alphabetically by name

## Execution Flow

1. **Select Pipeline**: Use arrow keys to highlight a pipeline
2. **Start Execution**: Press Enter to begin execution
3. **Watch Progress**: View real-time progress and output
4. **Completion**: Pipeline shows success or failure status
5. **Return to List**: Press q or Esc to select another pipeline

## Example Pipelines

### Simple Example
```yaml
name: hello-world
description: A simple hello world pipeline
steps:
  - name: Greet
    command: echo "Hello, World!"
```

### Multi-Step Build
```yaml
name: rust-build
description: Build and test Rust project
steps:
  - name: Check formatting
    command: cargo fmt --check
  
  - name: Run tests
    command: cargo test
  
  - name: Build release
    command: cargo build --release
```

## Tips

1. **Pipeline Organization**: Keep pipeline files in a dedicated directory for easy browsing
2. **Descriptive Names**: Use clear names and descriptions for pipelines
3. **Testing**: Test pipelines individually before chaining them
4. **Output Monitoring**: Watch the output panel for errors or unexpected behavior
5. **Quick Exit**: Press q during execution to see completion status faster

## Troubleshooting

### No Pipelines Found
- Ensure you're in a directory with `.yaml` or `.yml` files
- Check that files are valid pipeline YAML format
- Verify files have the required `name` and `steps` fields

### Pipeline Fails to Execute
- Check the output panel for error messages
- Verify commands are available on your system
- Check file permissions and working directory
- Review pipeline YAML syntax

### TUI Doesn't Respond
- Ensure terminal supports TUI applications
- Check terminal size is adequate (minimum 80x24)
- Try resizing terminal window

## Advanced Usage

### Running from Specific Directory
```bash
cd ~/projects/pipelines && cargo run --bin tui
```

### Building and Running Release Version
```bash
cargo build --release
./target/release/tui
```

### Integration with RPC Service
The TUI is designed to communicate with the RPC service layer for remote pipeline execution. Future versions will support:
- Remote pipeline execution
- Distributed builds
- Pipeline scheduling
- Execution history

## See Also

- [PIPELINE.md](PIPELINE.md) - Pipeline YAML format documentation
- [README.md](README.md) - Project overview
- [EXTENDING.md](EXTENDING.md) - Extending the pipeline system
