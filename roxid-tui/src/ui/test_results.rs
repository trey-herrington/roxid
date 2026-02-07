use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::app::App;
use crate::ui::{components, layout};

pub fn render(app: &App, frame: &mut Frame) {
    let chunks = layout::create_layout(frame.area());

    components::render_header("Test Results", frame, chunks[0]);

    if let Some(test_state) = &app.test_state {
        if test_state.is_running {
            let msg = Paragraph::new("Running tests...")
                .style(
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )
                .block(Block::default().borders(Borders::ALL).title("Tests"));
            frame.render_widget(msg, chunks[1]);
        } else if test_state.results.is_empty() {
            let msg = Paragraph::new(vec![
                Line::from("No test files found."),
                Line::from(""),
                Line::from("Test files should be named 'roxid-test.yml'"),
                Line::from("and placed in the current directory or subdirectories."),
            ])
            .style(Style::default().fg(Color::Yellow))
            .block(Block::default().borders(Borders::ALL).title("Tests"))
            .wrap(Wrap { trim: true });
            frame.render_widget(msg, chunks[1]);
        } else {
            // Split into summary and details
            let sections = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(5), Constraint::Min(1)])
                .split(chunks[1]);

            // Summary bar
            let total =
                test_state.total_passed + test_state.total_failed + test_state.total_skipped;
            let summary = Paragraph::new(vec![
                Line::from(vec![
                    Span::styled(
                        format!(" {} total", total),
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw("  |  "),
                    Span::styled(
                        format!("{} passed", test_state.total_passed),
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw("  |  "),
                    Span::styled(
                        format!("{} failed", test_state.total_failed),
                        Style::default()
                            .fg(if test_state.total_failed > 0 {
                                Color::Red
                            } else {
                                Color::DarkGray
                            })
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw("  |  "),
                    Span::styled(
                        format!("{} skipped", test_state.total_skipped),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]),
                Line::from(""),
                Line::from(vec![Span::styled(
                    format!(" {} suites", test_state.results.len()),
                    Style::default().fg(Color::Gray),
                )]),
            ])
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Summary")
                    .border_style(if test_state.total_failed > 0 {
                        Style::default().fg(Color::Red)
                    } else {
                        Style::default().fg(Color::Green)
                    }),
            );
            frame.render_widget(summary, sections[0]);

            // Test results list
            let mut items: Vec<ListItem> = Vec::new();
            let mut global_idx = 0;

            for suite in &test_state.results {
                // Suite header
                let suite_name = &suite.suite_name;
                let all_passed = suite.failed == 0;
                items.push(ListItem::new(Line::from(vec![
                    Span::styled(
                        if all_passed { "O " } else { "X " },
                        Style::default().fg(if all_passed { Color::Green } else { Color::Red }),
                    ),
                    Span::styled(
                        suite_name,
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!(" ({}/{} passed)", suite.passed, suite.total),
                        Style::default().fg(Color::DarkGray),
                    ),
                ])));

                // Individual tests
                for test in &suite.results {
                    let is_selected = global_idx == test_state.selected_test;
                    let style = if is_selected {
                        Style::default()
                            .fg(Color::Black)
                            .bg(Color::Cyan)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::White)
                    };

                    let (symbol, symbol_color) = if test.passed {
                        ("  O ", Color::Green)
                    } else {
                        ("  X ", Color::Red)
                    };

                    let duration_str = format!(
                        " ({})",
                        components::format_duration(test.duration.as_secs_f64())
                    );

                    let mut spans = vec![
                        Span::styled(symbol, Style::default().fg(symbol_color)),
                        Span::styled(&test.name, style),
                        Span::styled(duration_str, Style::default().fg(Color::DarkGray)),
                    ];

                    if !test.passed {
                        if let Some(msg) = &test.failure_message {
                            spans.push(Span::styled(
                                format!(" - {}", msg),
                                Style::default().fg(Color::Red),
                            ));
                        }
                    }

                    items.push(ListItem::new(Line::from(spans)));
                    global_idx += 1;
                }
            }

            let test_list = List::new(items).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Test Results")
                    .border_style(Style::default().fg(Color::Cyan)),
            );
            frame.render_widget(test_list, sections[1]);
        }
    } else {
        let msg = Paragraph::new("No test results available.")
            .style(Style::default().fg(Color::DarkGray))
            .block(Block::default().borders(Borders::ALL).title("Tests"));
        frame.render_widget(msg, chunks[1]);
    }

    components::render_footer("j/k: Navigate | q/Esc: Back", frame, chunks[2]);
}
