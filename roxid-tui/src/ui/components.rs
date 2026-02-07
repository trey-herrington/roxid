use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::app::DiscoveryError;

pub fn render_header(title: &str, frame: &mut Frame, area: Rect) {
    let header = Paragraph::new(title)
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(header, area);
}

pub fn render_footer(text: &str, frame: &mut Frame, area: Rect) {
    let footer = Paragraph::new(text)
        .style(Style::default().fg(Color::Gray))
        .block(Block::default().borders(Borders::ALL).title("Help"));
    frame.render_widget(footer, area);
}

pub fn render_discovery_errors(errors: &[DiscoveryError], frame: &mut Frame, area: Rect) {
    if errors.is_empty() {
        return;
    }

    let error_lines: Vec<Line> = errors
        .iter()
        .flat_map(|err| {
            vec![
                Line::from(vec![
                    Span::styled(
                        "x ",
                        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(&err.file_name, Style::default().fg(Color::Yellow)),
                ]),
                Line::from(vec![
                    Span::raw("  "),
                    Span::styled(&err.error, Style::default().fg(Color::Gray)),
                ]),
                Line::from(""),
            ]
        })
        .collect();

    let error_widget = Paragraph::new(error_lines).style(Style::default()).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Discovery Errors")
            .border_style(Style::default().fg(Color::Red)),
    );

    frame.render_widget(error_widget, area);
}

/// Render a status indicator symbol with color
#[allow(dead_code)]
pub fn status_style(succeeded: bool) -> Style {
    if succeeded {
        Style::default().fg(Color::Green)
    } else {
        Style::default().fg(Color::Red)
    }
}

/// Format a duration for display
pub fn format_duration(secs: f64) -> String {
    if secs < 1.0 {
        format!("{:.0}ms", secs * 1000.0)
    } else if secs < 60.0 {
        format!("{:.1}s", secs)
    } else {
        let mins = (secs / 60.0).floor();
        let remaining = secs - mins * 60.0;
        format!("{:.0}m {:.0}s", mins, remaining)
    }
}
