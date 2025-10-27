# Rust TUI Application

A Terminal User Interface (TUI) application built with [Ratatui](https://ratatui.rs/) for managing and executing YAML-based pipelines.

## Features

- **Pipeline Discovery**: Automatically discovers pipeline YAML files in the current directory
- **Interactive Selection**: Navigate through available pipelines with keyboard controls
- **Real-time Execution**: Execute pipelines with live progress tracking
- **Progress Visualization**: Visual progress bar showing current step
- **Live Output**: Real-time output display during pipeline execution
- **RPC Integration**: Communicates with the pipeline execution service via RPC API
- **Error Handling**: Uses color-eyre for better error reporting
- **Cross-platform**: Works on Linux, macOS, and Windows

## Dependencies

- `ratatui` - Core TUI framework
- `crossterm` - Cross-platform terminal manipulation
- `color-eyre` - Error handling and reporting
- `serde` & `serde_yaml` - YAML parsing for pipelines
- `tokio` - Async runtime for pipeline execution

## Installation

```bash
cargo build --release
```

## Usage

### Interactive TUI

Run the TUI application to browse and execute pipelines:

```bash
cd /path/to/your/pipelines
cargo run --bin tui
```

The TUI will discover all `.yaml` and `.yml` files in the current directory and display them as available pipelines.

### Keyboard Controls

**Pipeline List Screen:**
- `↑` or `k` - Move selection up
- `↓` or `j` - Move selection down
- `Enter` - Execute selected pipeline
- `q` or `Esc` - Quit application

**Pipeline Execution Screen:**
- `q` or `Esc` - Return to pipeline list (only after completion)

### Pipeline Execution

You can also execute pipelines directly via CLI:

```bash
# Run example pipeline
cargo run --bin pipeline-cli example-pipeline.yaml

# Run Rust build pipeline
cargo run --bin pipeline-cli rust-build-pipeline.yaml
```

See [PIPELINE.md](PIPELINE.md) for detailed documentation on creating and executing YAML pipelines.

## Project Structure

```
rust-tui-app/
├── Cargo.toml          # Workspace manifest
├── README.md           # This file
├── PIPELINE.md         # Pipeline execution documentation
├── tui/                # TUI application (binary)
├── service/            # Business logic layer (library)
│   └── src/
│       ├── pipeline/   # Pipeline execution system
│       ├── models/     # Data models
│       └── services/   # Business services
├── rpc/                # RPC API layer (library)
├── pipeline-cli/       # CLI tool for testing pipelines
└── example-pipeline.yaml
```

## Architecture

The application follows a modern TUI architecture with state management:

### TUI Application Flow

1. **Pipeline List State**: 
   - Discovers and displays available pipeline YAML files
   - User navigates with arrow keys and selects with Enter
   
2. **Pipeline Execution State**:
   - Executes selected pipeline asynchronously
   - Displays real-time progress bar (current step / total steps)
   - Streams output to the terminal as it's generated
   - Shows completion status (success/failure)

3. **Event Loop**: The main loop continuously:
   - Draws the UI based on current state
   - Handles keyboard events with non-blocking polling
   - Updates state based on execution events
   
4. **RPC Communication**: 
   - TUI communicates with the service layer through RPC API
   - Uses message passing for progress updates and output streaming

### Components

- **App State Machine**: Manages transitions between PipelineList and ExecutingPipeline states
- **Event Handler**: Non-blocking event processing with state-aware key bindings
- **UI Rendering**: Modular components for pipeline list, progress bar, and output display
- **Pipeline Executor**: Async execution with progress events streamed via channels

## Extending the Application

To extend this application:

1. **Add new pipeline steps**: Update pipeline YAML files with new commands or shell scripts
2. **Custom step runners**: Implement new runners in `service/src/pipeline/runners/`
3. **RPC handlers**: Add new RPC handlers in `rpc/src/handlers/` for custom operations
4. **UI screens**: Create new app states and corresponding UI components
5. **Pipeline filters**: Add filtering/searching capabilities to the pipeline list

See [EXTENDING.md](EXTENDING.md) for detailed guides on extending the pipeline system.

## Resources

- [Ratatui Documentation](https://docs.rs/ratatui)
- [Ratatui Website](https://ratatui.rs/)
- [Ratatui Examples](https://github.com/ratatui/ratatui/tree/main/examples)
- [Crossterm Documentation](https://docs.rs/crossterm)

## License

This skeleton application is provided as-is for educational and development purposes.
