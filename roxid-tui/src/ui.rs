pub mod components;
pub mod execution;
pub mod layout;
pub mod log_viewer;
pub mod pipeline_list;
pub mod pipeline_tree;
pub mod test_results;

use ratatui::Frame;

use crate::app::{App, AppState};

pub fn render(app: &App, frame: &mut Frame) {
    match app.state {
        AppState::PipelineList => pipeline_list::render(app, frame),
        AppState::PipelineDetail => pipeline_tree::render(app, frame),
        AppState::ExecutingPipeline => execution::render(app, frame),
        AppState::ExecutionLog => log_viewer::render(app, frame),
        AppState::TestResults => test_results::render(app, frame),
        AppState::VariableEditor => render_variable_editor(app, frame),
    }
}

fn render_variable_editor(app: &App, frame: &mut Frame) {
    let chunks = layout::create_layout(frame.area());

    components::render_header("Variable Editor", frame, chunks[0]);

    if let Some(editor) = &app.variable_editor {
        use ratatui::{
            layout::{Constraint, Direction, Layout},
            style::{Color, Modifier, Style},
            text::{Line, Span},
            widgets::{Block, Borders, List, ListItem},
        };

        // Split main area into variables and parameters sections
        let sections = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[1]);

        // Variables section
        let var_items: Vec<ListItem> = editor
            .variables
            .iter()
            .enumerate()
            .map(|(i, var)| {
                let is_selected = i == editor.selected_index;
                let indicator = if is_selected { "> " } else { "  " };
                let readonly_marker = if var.readonly { " [readonly]" } else { "" };

                let value_display =
                    if is_selected && editor.editing && !editor.in_parameters_section {
                        format!("{}_", editor.edit_buffer)
                    } else {
                        var.value.clone()
                    };

                let style = if is_selected {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else if var.readonly {
                    Style::default().fg(Color::DarkGray)
                } else {
                    Style::default().fg(Color::White)
                };

                ListItem::new(Line::from(vec![
                    Span::styled(indicator, style),
                    Span::styled(&var.name, style),
                    Span::styled(" = ", Style::default().fg(Color::Gray)),
                    Span::styled(value_display, style),
                    Span::styled(readonly_marker, Style::default().fg(Color::DarkGray)),
                ]))
            })
            .collect();

        let var_list = List::new(var_items).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Variables")
                .border_style(Style::default().fg(Color::Cyan)),
        );
        frame.render_widget(var_list, sections[0]);

        // Parameters section
        let var_len = editor.variables.len();
        let param_items: Vec<ListItem> = editor
            .parameters
            .iter()
            .enumerate()
            .map(|(i, param)| {
                let global_idx = var_len + i;
                let is_selected = global_idx == editor.selected_index;
                let indicator = if is_selected { "> " } else { "  " };

                let value_display = if is_selected && editor.editing && editor.in_parameters_section
                {
                    format!("{}_", editor.edit_buffer)
                } else {
                    param.value.clone()
                };

                let label = param.display_name.as_deref().unwrap_or(&param.name);

                let style = if is_selected {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };

                let default_info = param
                    .default
                    .as_ref()
                    .map(|d| format!(" (default: {})", d))
                    .unwrap_or_default();

                ListItem::new(Line::from(vec![
                    Span::styled(indicator, style),
                    Span::styled(label, style),
                    Span::styled(" = ", Style::default().fg(Color::Gray)),
                    Span::styled(value_display, style),
                    Span::styled(default_info, Style::default().fg(Color::DarkGray)),
                ]))
            })
            .collect();

        let param_list = List::new(param_items).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Parameters")
                .border_style(Style::default().fg(Color::Magenta)),
        );
        frame.render_widget(param_list, sections[1]);
    } else {
        use ratatui::widgets::Paragraph;
        let msg = Paragraph::new("No pipeline selected")
            .style(ratatui::style::Style::default().fg(ratatui::style::Color::Yellow));
        frame.render_widget(msg, chunks[1]);
    }

    let footer = if app.variable_editor.as_ref().is_some_and(|e| e.editing) {
        "Enter: Confirm | Esc: Cancel"
    } else {
        "Enter: Edit | x: Execute | q/Esc: Back"
    };
    components::render_footer(footer, frame, chunks[2]);
}
