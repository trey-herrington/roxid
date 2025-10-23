use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};

use crate::app::App;

pub trait EventHandler {
    fn handle_events(&mut self) -> Result<()>;
    fn handle_key_event(&mut self, key_event: KeyEvent);
}

impl EventHandler for App {
    fn handle_events(&mut self) -> Result<()> {
        match event::read()? {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event(key_event)
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Char('q') => self.quit(),
            KeyCode::Char('j') => self.increment_counter(),
            KeyCode::Char('k') => self.decrement_counter(),
            KeyCode::Char('a') => self.add_item(),
            _ => {}
        }
    }
}
