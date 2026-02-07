use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};

use crate::app::{App, AppState};

pub trait EventHandler {
    fn handle_events(&mut self) -> Result<()>;
    fn handle_key_event(&mut self, key_event: KeyEvent) -> Result<()>;
}

impl EventHandler for App {
    fn handle_events(&mut self) -> Result<()> {
        if event::poll(std::time::Duration::from_millis(50))? {
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
        // Handle search input mode first (captures all keys)
        if self.log_viewer.search_active {
            match key_event.code {
                KeyCode::Esc => {
                    self.log_viewer.search_active = false;
                }
                KeyCode::Enter => {
                    self.confirm_search();
                }
                KeyCode::Backspace => {
                    self.search_pop_char();
                }
                KeyCode::Char(c) => {
                    self.search_push_char(c);
                }
                _ => {}
            }
            return Ok(());
        }

        // Handle variable editing mode (captures all keys)
        if let Some(editor) = &self.variable_editor {
            if editor.editing {
                match key_event.code {
                    KeyCode::Esc => {
                        self.cancel_edit();
                    }
                    KeyCode::Enter => {
                        self.confirm_edit();
                    }
                    KeyCode::Backspace => {
                        self.edit_buffer_pop();
                    }
                    KeyCode::Char(c) => {
                        self.edit_buffer_push(c);
                    }
                    _ => {}
                }
                return Ok(());
            }
        }

        // Normal mode key handling per state
        match self.state {
            AppState::PipelineList => match key_event.code {
                KeyCode::Char('q') | KeyCode::Esc => self.quit(),
                KeyCode::Up | KeyCode::Char('k') => self.move_up(),
                KeyCode::Down | KeyCode::Char('j') => self.move_down(),
                KeyCode::Enter => self.enter_pipeline_detail(),
                KeyCode::Char('v') => self.open_variable_editor(),
                KeyCode::Char('t') => self.request_test_run(),
                _ => {}
            },

            AppState::PipelineDetail => match key_event.code {
                KeyCode::Char('q') | KeyCode::Esc => self.go_back(),
                KeyCode::Up | KeyCode::Char('k') => self.move_up(),
                KeyCode::Down | KeyCode::Char('j') => self.move_down(),
                KeyCode::Enter | KeyCode::Char(' ') => self.toggle_tree_node(),
                KeyCode::Char('x') => self.request_execute_pipeline(),
                KeyCode::Char('v') => self.open_variable_editor(),
                KeyCode::Char('t') => self.request_test_run(),
                _ => {}
            },

            AppState::ExecutingPipeline => match key_event.code {
                KeyCode::Char('q') | KeyCode::Esc => {
                    if self.execution_state.as_ref().is_some_and(|s| s.is_complete) {
                        self.go_back();
                    }
                }
                KeyCode::Char('l') => self.open_log_viewer(),
                _ => {}
            },

            AppState::ExecutionLog => match key_event.code {
                KeyCode::Char('q') | KeyCode::Esc => self.go_back(),
                KeyCode::Up | KeyCode::Char('k') => self.move_up(),
                KeyCode::Down | KeyCode::Char('j') => self.move_down(),
                KeyCode::PageUp => self.page_up(),
                KeyCode::PageDown => self.page_down(),
                KeyCode::Char('/') => self.start_search(),
                KeyCode::Char('n') => self.next_search_match(),
                KeyCode::Home | KeyCode::Char('g') => {
                    self.log_viewer.scroll_offset = 0;
                }
                KeyCode::End | KeyCode::Char('G') => {
                    if let Some(exec) = &self.execution_state {
                        self.log_viewer.scroll_offset = exec.output_lines.len().saturating_sub(1);
                    }
                }
                _ => {}
            },

            AppState::TestResults => match key_event.code {
                KeyCode::Char('q') | KeyCode::Esc => self.go_back(),
                KeyCode::Up | KeyCode::Char('k') => self.move_up(),
                KeyCode::Down | KeyCode::Char('j') => self.move_down(),
                _ => {}
            },

            AppState::VariableEditor => match key_event.code {
                KeyCode::Char('q') | KeyCode::Esc => self.go_back(),
                KeyCode::Up | KeyCode::Char('k') => self.move_up(),
                KeyCode::Down | KeyCode::Char('j') => self.move_down(),
                KeyCode::Enter => self.start_editing_variable(),
                KeyCode::Char('x') => {
                    // Execute with current variable overrides
                    self.go_back(); // return to detail/list
                    self.request_execute_pipeline();
                }
                _ => {}
            },
        }
        Ok(())
    }
}
