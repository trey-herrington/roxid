use ratatui::layout::{Constraint, Direction, Layout, Rect};

pub fn create_layout(area: Rect) -> Vec<Rect> {
    Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(3),
        ])
        .split(area)
        .to_vec()
}

pub fn create_layout_with_errors(area: Rect) -> Vec<Rect> {
    Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),      // Header
            Constraint::Percentage(60), // Main content
            Constraint::Percentage(40), // Errors
            Constraint::Length(3),      // Footer
        ])
        .split(area)
        .to_vec()
}
