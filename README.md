# Rust TUI Application

A skeleton Terminal User Interface (TUI) application built with [Ratatui](https://ratatui.rs/).

## Features

- **Interactive Counter**: Increment and decrement a counter with keyboard controls
- **Dynamic List**: Add items to a list dynamically
- **Clean Layout**: Organized header, main content area, and footer with help text
- **Error Handling**: Uses color-eyre for better error reporting
- **Cross-platform**: Works on Linux, macOS, and Windows

## Dependencies

- `ratatui` - Core TUI framework
- `crossterm` - Cross-platform terminal manipulation
- `color-eyre` - Error handling and reporting

## Installation

```bash
cargo build --release
```

## Usage

Run the application:

```bash
cargo run
```

### Keyboard Controls

- `j` - Increment counter
- `k` - Decrement counter
- `a` - Add a new item to the list
- `q` - Quit the application

## Project Structure

```
rust-tui-app/
├── Cargo.toml          # Project dependencies and metadata
├── README.md           # This file
└── src/
    └── main.rs         # Main application code
```

## Architecture

The application follows a simple state management pattern:

1. **App State**: The `App` struct holds the application state (counter, items, quit flag)
2. **Event Loop**: The main loop continuously:
   - Draws the UI based on current state
   - Handles keyboard events
   - Updates state accordingly
3. **Rendering**: Split into separate functions for header, main content, and footer

## Extending the Application

To extend this skeleton:

1. **Add new state**: Update the `App` struct with additional fields
2. **Handle new events**: Extend `handle_key_event` with new key bindings
3. **Create new widgets**: Add rendering functions for new UI components
4. **Modularize**: Split code into separate files (e.g., `ui.rs`, `app.rs`, `events.rs`)

## Resources

- [Ratatui Documentation](https://docs.rs/ratatui)
- [Ratatui Website](https://ratatui.rs/)
- [Ratatui Examples](https://github.com/ratatui/ratatui/tree/main/examples)
- [Crossterm Documentation](https://docs.rs/crossterm)

## License

This skeleton application is provided as-is for educational and development purposes.
