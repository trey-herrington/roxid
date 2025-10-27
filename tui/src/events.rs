use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};

use crate::app::{App, AppState};

pub trait EventHandler {
    fn handle_events(&mut self) -> Result<()>;
    fn handle_key_event(&mut self, key_event: KeyEvent) -> Result<()>;
}

impl EventHandler for App {
    fn handle_events(&mut self) -> Result<()> {
        if event::poll(std::time::Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                    self.handle_key_event(key_event)?
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) -> Result<()> {
        match self.state {
            AppState::PipelineList => {
                match key_event.code {
                    KeyCode::Char('q') | KeyCode::Esc => self.quit(),
                    KeyCode::Up | KeyCode::Char('k') => self.move_up(),
                    KeyCode::Down | KeyCode::Char('j') => self.move_down(),
                    KeyCode::Enter => {
                        let _ = self.execute_selected_pipeline();
                    }
                    _ => {}
                }
            }
            AppState::ExecutingPipeline => {
                match key_event.code {
                    KeyCode::Char('q') | KeyCode::Esc => {
                        if self.execution_state.as_ref().map_or(false, |s| s.is_complete) {
                            self.back_to_list();
                        }
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }
}
