use ratatui::layout::{Constraint, Direction, Layout, Rect};

/// Standard 3-section layout: header, main, footer
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

/// 4-section layout with an errors panel
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

/// Execution layout: header, progress, output, footer
pub fn create_execution_layout(area: Rect) -> Vec<Rect> {
    Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Length(3), // Progress bar
            Constraint::Min(1),    // Stage/output area
            Constraint::Length(3), // Footer
        ])
        .split(area)
        .to_vec()
}
