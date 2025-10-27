# TUI Quick Start Guide

Get started with the Pipeline TUI in 60 seconds!

## Prerequisites

- Rust toolchain installed
- Terminal that supports TUI applications

## Quick Start

### 1. Clone and Build (if needed)
```bash
cd /home/trey/repos/rust-tui-app
cargo build
```

### 2. Run the TUI
```bash
cargo run --bin tui
```

### 3. Navigate and Execute
- Use **↑** and **↓** arrow keys (or **k**/**j**) to select a pipeline
- Press **Enter** to execute the selected pipeline
- Watch the progress bar and live output
- Press **q** or **Esc** to return to the list after completion

## What You'll See

### Initial Screen
```
┌─ Pipeline Runner ──────────────────────────────────────┐
│                                                         │
└─────────────────────────────────────────────────────────┘
┌─ Available Pipelines ──────────────────────────────────┐
│  → example-pipeline - A simple example pipeline        │
│    rust-build-pipeline - Build Rust project            │
│    advanced-pipeline - Complex workflow example        │
└─────────────────────────────────────────────────────────┘
┌─ Help ─────────────────────────────────────────────────┐
│ ↑/↓: Navigate | Enter: Execute | q: Quit               │
└─────────────────────────────────────────────────────────┘
```

### During Execution
```
┌─ Executing: example-pipeline ──────────────────────────┐
│                                                         │
└─────────────────────────────────────────────────────────┘
┌─ Progress ─────────────────────────────────────────────┐
│ ████████████░░░░░░░░░░░░  Step 3/5                    │
└─────────────────────────────────────────────────────────┘
┌─ Output ───────────────────────────────────────────────┐
│ [Step 1/5] Check Rust version                          │
│   rustc 1.70.0 (90c541806 2023-05-31)                 │
│   ✓ Completed in 0.05s                                │
│ [Step 2/5] List files                                  │
│   ✓ Completed in 0.02s                                │
│ [Step 3/5] Multi-line script                           │
│   Starting multi-line script...                        │
└─────────────────────────────────────────────────────────┘
```

## Example Pipeline Files

The repository includes these example pipelines:

1. **example-pipeline.yaml** - Simple demo with basic commands
2. **rust-build-pipeline.yaml** - Builds this Rust project
3. **advanced-pipeline.yaml** - More complex multi-step workflow

## Creating Your Own Pipeline

Create a file ending in `.yaml` or `.yml` in the same directory:

```yaml
name: my-first-pipeline
description: My first custom pipeline
steps:
  - name: Hello World
    command: echo "Hello from my pipeline!"
  
  - name: Show Date
    command: date
  
  - name: List Files
    command: ls -la
```

Rerun the TUI and your new pipeline will appear in the list!

## Keyboard Reference

| Screen | Key | Action |
|--------|-----|--------|
| Pipeline List | `↑` or `k` | Move up |
| Pipeline List | `↓` or `j` | Move down |
| Pipeline List | `Enter` | Execute selected |
| Pipeline List | `q` or `Esc` | Quit |
| Execution | `q` or `Esc` | Back to list (when done) |

## Tips

- Start with **example-pipeline.yaml** to see how it works
- Create pipelines in a dedicated directory for easy access
- Use descriptive names and descriptions
- Watch the output for errors during execution

## Next Steps

- Read [TUI_USAGE.md](TUI_USAGE.md) for detailed usage
- Learn pipeline syntax in [PIPELINE.md](PIPELINE.md)
- Explore extending the system in [EXTENDING.md](EXTENDING.md)

## Troubleshooting

**No pipelines appear?**
- Make sure you're in a directory with `.yaml` or `.yml` files
- Check that files have valid pipeline format with `name` and `steps`

**Pipeline won't execute?**
- Check the output for error messages
- Verify commands exist on your system
- Ensure you have necessary permissions

**TUI looks broken?**
- Make sure terminal is at least 80x24 characters
- Try a different terminal emulator
- Check that your terminal supports color

## Get Help

- Run `cargo run --bin tui` and explore the interface
- Check example pipelines for syntax reference
- Read full documentation in [TUI_USAGE.md](TUI_USAGE.md)

Happy pipeline running! 🚀
