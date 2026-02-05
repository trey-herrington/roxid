use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::app::{DiscoveryError, ExecutionState, PipelineInfo};

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

pub fn render_pipeline_list(
    pipelines: &[PipelineInfo],
    selected_index: usize,
    frame: &mut Frame,
    area: Rect,
) {
    if pipelines.is_empty() {
        let empty_msg = Paragraph::new(vec![
            Line::from("No valid pipeline YAML files found in current directory."),
            Line::from(""),
            Line::from("Pipeline files must have:"),
            Line::from("  - Extension: .yaml or .yml"),
            Line::from("  - Required field: 'name'"),
            Line::from("  - Required field: 'steps' (array)"),
        ])
        .style(Style::default().fg(Color::Yellow))
        .block(Block::default().borders(Borders::ALL).title("Pipelines"))
        .wrap(Wrap { trim: true });
        frame.render_widget(empty_msg, area);
        return;
    }

    let list_items: Vec<ListItem> = pipelines
        .iter()
        .enumerate()
        .map(|(i, pipeline)| {
            let style = if i == selected_index {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            let indicator = if i == selected_index { "→ " } else { "  " };
            let desc = pipeline
                .description
                .as_ref()
                .map(|d| format!(" - {}", d))
                .unwrap_or_default();

            let content = Line::from(vec![
                Span::styled(indicator, style),
                Span::styled(&pipeline.name, style),
                Span::styled(desc, Style::default().fg(Color::Gray)),
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

pub fn render_execution_view(exec_state: &ExecutionState, frame: &mut Frame, area: Rect) {
    use ratatui::layout::{Constraint, Direction, Layout};

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Progress bar
            Constraint::Min(0),    // Output
        ])
        .split(area);

    // Progress bar
    let progress = if exec_state.total_steps > 0 {
        (exec_state.current_step as f64 / exec_state.total_steps as f64) * 100.0
    } else {
        0.0
    };

    let label = format!(
        "Step {}/{}",
        exec_state.current_step, exec_state.total_steps
    );

    let gauge_color = if exec_state.is_complete {
        if exec_state.success {
            Color::Green
        } else {
            Color::Red
        }
    } else {
        Color::Cyan
    };

    let gauge = Gauge::default()
        .block(Block::default().borders(Borders::ALL).title("Progress"))
        .gauge_style(Style::default().fg(gauge_color))
        .label(label)
        .ratio(progress / 100.0);

    frame.render_widget(gauge, chunks[0]);

    // Output
    let output_height = chunks[1].height.saturating_sub(2) as usize;
    let start_line = exec_state.output_lines.len().saturating_sub(output_height);
    let visible_lines: Vec<Line> = exec_state
        .output_lines
        .iter()
        .skip(start_line)
        .map(|line| {
            if line.contains("✓") {
                Line::from(Span::styled(line, Style::default().fg(Color::Green)))
            } else if line.contains("✗") {
                Line::from(Span::styled(line, Style::default().fg(Color::Red)))
            } else if line.starts_with("[Step") {
                Line::from(Span::styled(
                    line,
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ))
            } else {
                Line::from(line.as_str())
            }
        })
        .collect();

    let output = Paragraph::new(visible_lines)
        .block(Block::default().borders(Borders::ALL).title("Output"))
        .style(Style::default().fg(Color::White))
        .wrap(Wrap { trim: false });

    frame.render_widget(output, chunks[1]);
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
                        "✗ ",
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

    let error_widget = Paragraph::new(error_lines)
        .style(Style::default())
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Discovery Errors")
                .border_style(Style::default().fg(Color::Red)),
        )
        .wrap(Wrap { trim: false });

    frame.render_widget(error_widget, area);
}
