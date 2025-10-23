# Extending the TUI Application

This guide shows how to extend the skeleton TUI application with common patterns.

## Adding a New Tab System

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

## Adding Scrolling to Lists

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

## Adding Input Fields

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

// Render input field:
let input = Paragraph::new(self.input.as_str())
    .style(match self.input_mode {
        true => Style::default().fg(Color::Yellow),
        false => Style::default(),
    })
    .block(Block::default().borders(Borders::ALL).title("Input"));
```

## Adding Mouse Support

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

## Adding Popup Dialogs

```rust
fn render_popup(&self, frame: &mut Frame) {
    let popup_area = centered_rect(60, 20, frame.area());
    
    let popup = Paragraph::new("Are you sure?")
        .block(Block::default()
            .title("Confirm")
            .borders(Borders::ALL))
        .style(Style::default().bg(Color::DarkGray));
    
    frame.render_widget(Clear, popup_area); // Clear background
    frame.render_widget(popup, popup_area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
```

## Adding Async Operations

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

## Separating into Modules

Create a multi-file structure:

```
src/
├── main.rs       # Entry point
├── app.rs        # App state and logic
├── ui.rs         # Rendering functions
├── events.rs     # Event handling
└── state.rs      # State management
```

## Best Practices

1. **Keep render functions pure** - Don't modify state during rendering
2. **Handle errors gracefully** - Use Result types and proper error handling
3. **Test rendering logic** - Ratatui supports testing with Buffer
4. **Profile performance** - TUI apps should render at 60fps
5. **Use proper terminal cleanup** - Always restore terminal state on exit

## Useful Widgets

- `Paragraph` - Text display
- `List` - Scrollable lists
- `Table` - Tabular data
- `Block` - Borders and titles
- `Gauge` - Progress bars
- `Chart` - Line/Bar charts
- `Sparkline` - Inline graphs
- `Tabs` - Tab navigation
- `Canvas` - Custom drawing

## Further Reading

- [Ratatui Async Template](https://github.com/ratatui/templates)
- [Ratatui Book](https://ratatui.rs/concepts/)
- [Example Projects](https://github.com/ratatui/awesome-ratatui)
