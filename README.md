# Roxid

A Terminal User Interface (TUI) application built with [Ratatui](https://ratatui.rs/) for managing and executing YAML-based pipelines.

## Table of Contents

- [Features](#features)
- [Quick Start](#quick-start)
- [Installation](#installation)
- [Usage](#usage)
- [Architecture](#architecture)
- [Pipeline System](#pipeline-system)
- [TUI Guide](#tui-guide)
- [Extending the Application](#extending-the-application)
- [Project Structure](#project-structure)
- [Resources](#resources)

## Features

### TUI Features
- **Pipeline Discovery**: Automatically discovers pipeline YAML files in the current directory
- **Interactive Selection**: Navigate through available pipelines with keyboard controls
- **Real-time Execution**: Execute pipelines with live progress tracking
- **Progress Visualization**: Visual progress bar showing current step
- **Live Output**: Real-time output display during pipeline execution
- **Error Handling**: Uses color-eyre for better error reporting
- **Cross-platform**: Works on Linux, macOS, and Windows

### Pipeline Features
- **YAML-based pipeline definitions**: Simple, declarative syntax
- **Command and shell script execution**: Run single commands or multi-line scripts
- **Environment variable support**: Pipeline-level and step-level env vars
- **Real-time progress reporting**: Live updates via async channels
- **Error handling**: Continue on error support for non-critical steps

## Quick Start

### 1. Build the Project
```bash
cd /path/to/roxid
cargo build
```

### 2. Run the TUI
```bash
cargo run --bin roxid-tui
```

### 3. Navigate and Execute
- Use **↑** and **↓** arrow keys (or **k**/**j**) to select a pipeline
- Press **Enter** to execute the selected pipeline
- Watch the progress bar and live output
- Press **q** or **Esc** to return to the list after completion

## Installation

### Option 1: From crates.io (Recommended)

```bash
cargo install roxid
roxid
```

### Option 2: Pre-built Binary

Download from [Releases](https://github.com/yourusername/roxid/releases/latest):
- **Linux x86_64**: `roxid-linux-x86_64.tar.gz`
- **macOS Intel**: `roxid-macos-x86_64.tar.gz`
- **macOS Apple Silicon**: `roxid-macos-aarch64.tar.gz`
- **Windows**: `roxid-windows-x86_64.exe.zip`

**Linux/macOS:**
```bash
tar xzf roxid-*.tar.gz
sudo mv roxid-* /usr/local/bin/roxid
chmod +x /usr/local/bin/roxid
```

**Windows:** Extract ZIP and add to PATH.

### Option 3: From Git

```bash
cargo install --git https://github.com/yourusername/roxid
```

### Option 4: Build from Source

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

### Interactive TUI

Run the TUI application to browse and execute pipelines:

```bash
cd /path/to/your/pipelines
cargo run --bin roxid-tui
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
cargo run --bin roxid-cli run example-pipeline.yaml

# Run Rust build pipeline
cargo run --bin roxid-cli run rust-build-pipeline.yaml
```

## Architecture

### Workspace Structure

This workspace follows a typical Rust project structure with separation of concerns:

```
roxid/
├── Cargo.toml              # Workspace manifest
├── roxid-tui/              # Terminal UI application (binary)
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs         # Entry point
│       ├── app.rs          # Application state and logic
│       ├── events.rs       # Event handling (keyboard, mouse)
│       └── ui/             # UI rendering modules
│           ├── mod.rs
│           ├── components.rs  # Reusable UI components
│           └── layout.rs      # Layout definitions
├── pipeline-service/       # Business logic layer (library)
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs          # Library entry point
│       ├── error.rs        # Error types
│       ├── models/         # Data models
│       ├── pipeline/       # Pipeline execution system
│       └── services/       # Business logic
├── pipeline-rpc/           # RPC API layer (library)
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs          # Library entry point
│       ├── api.rs          # RPC server setup
│       └── handlers/       # RPC request handlers
└── roxid-cli/              # CLI application (binary)
    ├── Cargo.toml
    └── src/
        └── main.rs         # CLI entry point
```

### Architecture Layers

#### 1. **Service Package** (`pipeline-service/`)
- **Purpose**: Core business logic
- **Type**: Library crate
- **Dependencies**: None (pure business logic)
- **Structure**:
  - `models/`: Data structures and domain models
  - `pipeline/`: Pipeline execution system
  - `services/`: Business logic implementations
  - `error.rs`: Domain-specific error types

#### 2. **RPC Package** (`pipeline-rpc/`)
- **Purpose**: Remote procedure call API layer
- **Type**: Library crate
- **Dependencies**: Depends on `pipeline-service`
- **Structure**:
  - `api.rs`: RPC server setup
  - `handlers/`: Request handlers that call service layer (PipelineHandler, UserHandler)
  - `error.rs`: RPC-specific error handling
- **Responsibilities**: Provides the API interface that clients use to interact with the service layer

#### 3. **TUI Package** (`roxid-tui/`)
- **Purpose**: Terminal user interface
- **Type**: Binary crate (executable)
- **Dependencies**: Depends only on `pipeline-rpc`
- **Structure**:
  - `main.rs`: Application entry point
  - `app.rs`: Application state management
  - `events.rs`: User input handling
  - `ui/`: All rendering logic separated by concern

#### 4. **CLI Package** (`roxid-cli/`)
- **Purpose**: Command-line interface
- **Type**: Binary crate (executable)
- **Dependencies**: Depends only on `pipeline-rpc`
- **Structure**:
  - `main.rs`: CLI entry point with command parsing

### Dependency Direction
```
roxid-tui → pipeline-rpc → pipeline-service
roxid-cli → pipeline-rpc → pipeline-service
```

**Key Architectural Principle**: Both client applications (TUI and CLI) communicate with the service layer exclusively through the RPC API. This ensures:
- Clean separation between presentation and business logic
- Consistent API interface for all clients
- Easy addition of new clients (web, mobile, etc.) without modifying service code
- Centralized API logic in the RPC layer

### TUI Application Flow

1. **Pipeline List State**: 
   - Discovers and displays available pipeline YAML files
   - User navigates with arrow keys and selects with Enter
   
2. **Pipeline Execution State**:
   - Executes selected pipeline asynchronously via RPC handler
   - Displays real-time progress bar (current step / total steps)
   - Streams output to the terminal as it's generated
   - Shows completion status (success/failure)

3. **Event Loop**: The main loop continuously:
   - Draws the UI based on current state
   - Handles keyboard events with non-blocking polling
   - Updates state based on execution events
   
4. **RPC Communication**: 
   - TUI uses `PipelineHandler` from pipeline-rpc to interact with pipelines
   - All service operations go through the RPC layer
   - Uses async channels for progress updates and output streaming
   - No direct dependency on pipeline-service

### State Machine
```
┌─────────────────┐
│ PipelineList    │
│ - Discover YAML │
│ - Navigate      │
│ - Select        │
└────────┬────────┘
         │ Enter
         ▼
┌─────────────────┐
│ ExecutingPipe   │
│ - Run pipeline  │
│ - Show progress │
│ - Stream output │
└────────┬────────┘
         │ Complete
         ▼
┌─────────────────┐
│ PipelineList    │
│ (return)        │
└─────────────────┘
```

### Components

- **App State Machine**: Manages transitions between PipelineList and ExecutingPipeline states
- **Event Handler**: Non-blocking event processing with state-aware key bindings
- **UI Rendering**: Modular components for pipeline list, progress bar, and output display
- **Pipeline Executor**: Async execution with progress events streamed via channels

## Pipeline System

### Pipeline YAML Format

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

### Creating a Pipeline

Create a file ending in `.yaml` or `.yml`:

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

### Basic Usage Example

```rust
use pipeline_rpc::{PipelineHandler, ExecutionEvent};
use color_eyre::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Create RPC handler
    let handler = PipelineHandler::new();
    
    // Parse pipeline from file
    let pipeline = handler.parse_from_file("pipeline.yaml")?;
    
    // Get working directory
    let working_dir = std::env::current_dir()?.to_string_lossy().to_string();
    
    // Execute pipeline through RPC layer
    handler.execute_pipeline(pipeline, working_dir, None).await?;
    
    Ok(())
}
```

### With Progress Reporting

```rust
use pipeline_rpc::{PipelineHandler, ExecutionEvent};

// Create event channel
let (tx, mut rx) = PipelineHandler::create_event_channel();

// Create handler and parse pipeline
let handler = PipelineHandler::new();
let pipeline = handler.parse_from_file("pipeline.yaml")?;
let working_dir = std::env::current_dir()?.to_string_lossy().to_string();

// Spawn executor
let handle = tokio::spawn(async move {
    handler.execute_pipeline(pipeline, working_dir, Some(tx)).await
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

let result = handle.await?;
```

### Example Pipelines

#### Simple Test Pipeline
```yaml
name: quick-test
steps:
  - name: Check version
    command: rustc --version
  - name: Run tests
    command: cargo test
```

#### Build Pipeline
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

### Pipeline Architecture

The pipeline system consists of:

- **Parser** (`pipeline/parser.rs`) - Parses YAML into Rust structs
- **Executor** (`pipeline/executor.rs`) - Orchestrates step execution
- **Runners** (`pipeline/runners/`) - Execute different step types
  - `shell.rs` - Runs shell commands and scripts
- **Models** (`pipeline/models.rs`) - Data structures for pipelines, steps, results

## TUI Guide

### Interface Overview

#### Pipeline List Screen
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

#### Pipeline Execution Screen
```
┌─────────────────────────────────────────────┐
│  Executing: example-pipeline                │
└─────────────────────────────────────────────┘
┌─ Progress ──────────────────────────────────┐
│ ████████████░░░░░░░░  Step 3/5             │
└─────────────────────────────────────────────┘
┌─ Output ────────────────────────────────────┐
│ [Step 1/5] Check Rust version              │
│   rustc 1.70.0 (90c541806 2023-05-31)      │
│   ✓ Completed in 0.05s                     │
│                                             │
│ [Step 2/5] List files                      │
│   ✓ Completed in 0.02s                     │
│                                             │
│ [Step 3/5] Multi-line script               │
│   Starting multi-line script                │
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

### Execution Flow

1. **Select Pipeline**: Use arrow keys to highlight a pipeline
2. **Start Execution**: Press Enter to begin execution
3. **Watch Progress**: View real-time progress and output
4. **Completion**: Pipeline shows success or failure status
5. **Return to List**: Press q or Esc to select another pipeline

### Troubleshooting

**No Pipelines Found**
- Ensure you're in a directory with `.yaml` or `.yml` files
- Check that files are valid pipeline YAML format
- Verify files have the required `name` and `steps` fields

**Pipeline Fails to Execute**
- Check the output panel for error messages
- Verify commands are available on your system
- Check file permissions and working directory
- Review pipeline YAML syntax

**TUI Doesn't Respond**
- Ensure terminal supports TUI applications
- Check terminal size is adequate (minimum 80x24)
- Try resizing terminal window

## Extending the Application

### Adding a New Tab System

```rust
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum Tab {
    #[default]
    Counter,
    Items,
    Settings,
}

// Add to App struct:
struct App {
    counter: i32,
    items: Vec<String>,
    current_tab: Tab,
    should_quit: bool,
}

// Handle tab switching:
KeyCode::Char('1') => self.current_tab = Tab::Counter,
KeyCode::Char('2') => self.current_tab = Tab::Items,
KeyCode::Char('3') => self.current_tab = Tab::Settings,
```

### Adding Scrolling to Lists

```rust
use ratatui::widgets::ListState;

struct App {
    // ... existing fields
    list_state: ListState,
}

// Handle scrolling:
KeyCode::Up => {
    let i = self.list_state.selected().unwrap_or(0);
    if i > 0 {
        self.list_state.select(Some(i - 1));
    }
}
KeyCode::Down => {
    let i = self.list_state.selected().unwrap_or(0);
    if i < self.items.len() - 1 {
        self.list_state.select(Some(i + 1));
    }
}

// Render with state:
frame.render_stateful_widget(list, main_chunks[1], &mut self.list_state);
```

### Adding Input Fields

```rust
struct App {
    // ... existing fields
    input: String,
    input_mode: bool,
}

// Handle input mode:
KeyCode::Char('i') if !self.input_mode => {
    self.input_mode = true;
}
KeyCode::Esc if self.input_mode => {
    self.input_mode = false;
}
KeyCode::Char(c) if self.input_mode => {
    self.input.push(c);
}
KeyCode::Backspace if self.input_mode => {
    self.input.pop();
}
KeyCode::Enter if self.input_mode => {
    self.items.push(self.input.clone());
    self.input.clear();
    self.input_mode = false;
}
```

### Adding Mouse Support

```rust
use crossterm::event::{MouseEvent, MouseEventKind};

// In handle_events:
Event::Mouse(mouse_event) => self.handle_mouse_event(mouse_event),

// Handler:
fn handle_mouse_event(&mut self, mouse_event: MouseEvent) {
    match mouse_event.kind {
        MouseEventKind::Down(_) => {
            // Handle click at position (mouse_event.column, mouse_event.row)
        }
        MouseEventKind::ScrollUp => {
            // Handle scroll up
        }
        MouseEventKind::ScrollDown => {
            // Handle scroll down
        }
        _ => {}
    }
}
```

### Adding Async Operations

```rust
// Add to Cargo.toml:
// tokio = { version = "1", features = ["full"] }

use tokio::sync::mpsc;
use std::time::Duration;

enum AppEvent {
    Input(Event),
    DataUpdate(String),
}

#[tokio::main]
async fn main() -> Result<()> {
    let (tx, mut rx) = mpsc::channel(100);
    
    // Spawn background task
    let tx_clone = tx.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;
            let _ = tx_clone.send(AppEvent::DataUpdate("Updated".to_string())).await;
        }
    });
    
    // Main event loop
    loop {
        if event::poll(Duration::from_millis(100))? {
            tx.send(AppEvent::Input(event::read()?)).await?;
        }
        
        while let Ok(event) = rx.try_recv() {
            match event {
                AppEvent::Input(e) => { /* handle input */ }
                AppEvent::DataUpdate(data) => { /* update state */ }
            }
        }
    }
}
```

### Custom Pipeline Steps

To extend the pipeline system:

1. **Add new pipeline steps**: Update pipeline YAML files with new commands or shell scripts
2. **Custom step runners**: Implement new runners in `pipeline-service/src/pipeline/runners/`
3. **RPC handlers**: Add new RPC handlers in `pipeline-rpc/src/handlers/` to expose new functionality
4. **Update RPC API**: Export new handlers and types in `pipeline-rpc/src/lib.rs`
5. **Client updates**: Use new RPC handlers in CLI/TUI applications
6. **UI screens**: Create new app states and corresponding UI components in TUI
7. **Pipeline filters**: Add filtering/searching capabilities to the pipeline list

**Remember**: All client functionality must go through the RPC layer. Never import `pipeline-service` directly in CLI or TUI.

### Best Practices

1. **Keep render functions pure** - Don't modify state during rendering
2. **Handle errors gracefully** - Use Result types and proper error handling
3. **Test rendering logic** - Ratatui supports testing with Buffer
4. **Profile performance** - TUI apps should render at 60fps
5. **Use proper terminal cleanup** - Always restore terminal state on exit

## Project Structure

### Building and Running

```bash
# Build entire workspace
cargo build

# Build specific package
cargo build -p roxid-tui
cargo build -p pipeline-service
cargo build -p pipeline-rpc
cargo build -p roxid-cli

# Run the TUI application
cargo run --bin roxid-tui

# Run the CLI application
cargo run --bin roxid-cli run example-pipeline.yaml

# Run tests for all packages
cargo test

# Run tests for specific package
cargo test -p pipeline-rpc

# Check code without building
cargo check
```

### Benefits of This Structure

1. **Separation of Concerns**: Each package has a single responsibility
2. **Reusability**: Service logic can be used by multiple clients through RPC layer
3. **Testability**: Each layer can be tested independently
4. **Maintainability**: Clear boundaries make code easier to understand
5. **Scalability**: Easy to add new interfaces (web, mobile, desktop) without touching core logic
6. **API Consistency**: All clients use the same RPC interface, ensuring consistent behavior
7. **Security**: RPC layer can add authentication, rate limiting, and validation before reaching service
8. **Flexibility**: RPC layer can be made into a network service without changing clients

## Resources

- [Ratatui Documentation](https://docs.rs/ratatui)
- [Ratatui Website](https://ratatui.rs/)
- [Ratatui Examples](https://github.com/ratatui/ratatui/tree/main/examples)
- [Ratatui Book](https://ratatui.rs/concepts/)
- [Crossterm Documentation](https://docs.rs/crossterm)
- [Example Projects](https://github.com/ratatui/awesome-ratatui)

## License

This skeleton application is provided as-is for educational and development purposes.
