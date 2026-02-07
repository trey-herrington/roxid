use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use crate::app::{App, OutputKind};
use crate::ui::{components, layout};

pub fn render(app: &App, frame: &mut Frame) {
    let chunks = layout::create_layout(frame.area());

    components::render_header("Execution Log", frame, chunks[0]);

    let filtered = app.filtered_output_lines();

    if filtered.is_empty() {
        let msg = Paragraph::new("No output yet.")
            .style(Style::default().fg(Color::DarkGray))
            .block(Block::default().borders(Borders::ALL).title("Log"));
        frame.render_widget(msg, chunks[1]);
    } else {
        let visible_height = chunks[1].height.saturating_sub(2) as usize;
        let total = filtered.len();
        let offset = app
            .log_viewer
            .scroll_offset
            .min(total.saturating_sub(visible_height));

        let visible_lines: Vec<Line> = filtered
            .iter()
            .skip(offset)
            .take(visible_height)
            .enumerate()
            .map(|(i, line)| {
                let line_num = offset + i;
                let is_match = app.log_viewer.search_matches.contains(&line_num);

                let color = match line.kind {
                    OutputKind::Success => Color::Green,
                    OutputKind::Failure => Color::Red,
                    OutputKind::Error => Color::Red,
                    OutputKind::Warning => Color::Yellow,
                    OutputKind::StageHeader => Color::Yellow,
                    OutputKind::JobHeader => Color::Green,
                    OutputKind::StepHeader => Color::Cyan,
                    OutputKind::Info => Color::Gray,
                    OutputKind::Output => Color::White,
                };

                let bg = if is_match {
                    Color::DarkGray
                } else {
                    Color::Reset
                };

                Line::from(Span::styled(&line.text, Style::default().fg(color).bg(bg)))
            })
            .collect();

        let title = if app.log_viewer.search_active {
            format!("Log [Search: {}_]", app.log_viewer.search_query)
        } else if !app.log_viewer.search_query.is_empty() {
            format!(
                "Log [{} matches for '{}']",
                app.log_viewer.search_matches.len(),
                app.log_viewer.search_query
            )
        } else {
            format!("Log [{}/{}]", offset + 1, total)
        };

        let log = Paragraph::new(visible_lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(title)
                    .border_style(if app.log_viewer.search_active {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default().fg(Color::Cyan)
                    }),
            )
            .wrap(Wrap { trim: false });
        frame.render_widget(log, chunks[1]);
    }

    let footer = if app.log_viewer.search_active {
        "Type to search | Enter: Confirm | Esc: Cancel"
    } else {
        "j/k: Scroll | PgUp/PgDn: Page | /: Search | n: Next match | g/G: Top/Bottom | q/Esc: Back"
    };
    components::render_footer(footer, frame, chunks[2]);
}
