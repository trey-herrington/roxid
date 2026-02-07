use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::app::App;
use crate::ui::{components, layout};

pub fn render(app: &App, frame: &mut Frame) {
    if app.discovery_errors.is_empty() {
        let chunks = layout::create_layout(frame.area());
        components::render_header("Roxid Pipeline Runner", frame, chunks[0]);
        render_list(app, frame, chunks[1]);
        components::render_footer(
            "j/k: Navigate | Enter: Details | v: Variables | t: Tests | q: Quit",
            frame,
            chunks[2],
        );
    } else {
        let chunks = layout::create_layout_with_errors(frame.area());
        components::render_header("Roxid Pipeline Runner", frame, chunks[0]);
        render_list(app, frame, chunks[1]);
        components::render_discovery_errors(&app.discovery_errors, frame, chunks[2]);
        components::render_footer(
            "j/k: Navigate | Enter: Details | v: Variables | t: Tests | q: Quit",
            frame,
            chunks[3],
        );
    }
}

fn render_list(app: &App, frame: &mut Frame, area: ratatui::layout::Rect) {
    if app.pipelines.is_empty() {
        let empty_msg = Paragraph::new(vec![
            Line::from("No valid pipeline YAML files found in current directory."),
            Line::from(""),
            Line::from("Pipeline files must have:"),
            Line::from("  - Extension: .yaml or .yml"),
            Line::from("  - Valid Azure DevOps YAML schema"),
        ])
        .style(Style::default().fg(Color::Yellow))
        .block(Block::default().borders(Borders::ALL).title("Pipelines"))
        .wrap(Wrap { trim: true });
        frame.render_widget(empty_msg, area);
        return;
    }

    let list_items: Vec<ListItem> = app
        .pipelines
        .iter()
        .enumerate()
        .map(|(i, pipeline)| {
            let is_selected = i == app.selected_index;
            let style = if is_selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            let indicator = if is_selected { "> " } else { "  " };
            let info = format!(
                " ({} stages, {} jobs, {} steps)",
                pipeline.stages_count, pipeline.jobs_count, pipeline.steps_count
            );

            let content = Line::from(vec![
                Span::styled(indicator, style),
                Span::styled(&pipeline.name, style),
                Span::styled(info, Style::default().fg(Color::DarkGray)),
            ]);
            ListItem::new(content)
        })
        .collect();

    let list = List::new(list_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Available Pipelines"),
        )
        .style(Style::default());

    frame.render_widget(list, area);
}
