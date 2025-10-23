use color_eyre::Result;
use ratatui::DefaultTerminal;

use crate::events::EventHandler;
use crate::ui;

#[derive(Debug, Default)]
pub struct App {
    pub counter: i32,
    pub items: Vec<String>,
    pub should_quit: bool,
}

impl App {
    pub fn new() -> Self {
        Self {
            counter: 0,
            items: vec![
                "Welcome to your Rust TUI app!".to_string(),
                "Press 'j' to increment counter".to_string(),
                "Press 'k' to decrement counter".to_string(),
                "Press 'a' to add item".to_string(),
                "Press 'q' to quit".to_string(),
            ],
            should_quit: false,
        }
    }

    pub fn run(&mut self, mut terminal: DefaultTerminal) -> Result<()> {
        while !self.should_quit {
            terminal.draw(|frame| ui::render(self, frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    pub fn increment_counter(&mut self) {
        self.counter += 1;
    }

    pub fn decrement_counter(&mut self) {
        self.counter -= 1;
    }

    pub fn add_item(&mut self) {
        let item = format!("New item {}", self.items.len() - 4);
        self.items.push(item);
    }

    pub fn quit(&mut self) {
        self.should_quit = true;
    }
}
