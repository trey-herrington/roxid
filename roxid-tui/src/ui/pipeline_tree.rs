use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
    Frame,
};

use crate::app::App;
use crate::ui::{components, layout};

pub fn render(app: &App, frame: &mut Frame) {
    let chunks = layout::create_layout(frame.area());

    let pipeline = match app.selected_pipeline() {
        Some(info) => &info.pipeline,
        None => {
            components::render_header("Pipeline Detail", frame, chunks[0]);
            components::render_footer("q/Esc: Back", frame, chunks[2]);
            return;
        }
    };

    let pipeline_name = app
        .selected_pipeline()
        .map(|p| p.name.as_str())
        .unwrap_or("Pipeline");
    components::render_header(&format!("Pipeline: {}", pipeline_name), frame, chunks[0]);

    // Build tree lines
    let mut items: Vec<ListItem> = Vec::new();
    let mut line_idx = 0;

    for (si, stage) in pipeline.stages.iter().enumerate() {
        let is_selected = line_idx == app.tree_state.selected_line;
        let expanded =
            si < app.tree_state.expanded_stages.len() && app.tree_state.expanded_stages[si];

        let arrow = if expanded { "v " } else { "> " };
        let stage_name = stage.display_name.as_deref().unwrap_or(&stage.stage);
        let jobs_info = format!(" ({} jobs)", stage.jobs.len());

        let style = if is_selected {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        };

        items.push(ListItem::new(Line::from(vec![
            Span::styled(arrow, style),
            Span::styled(format!("Stage: {}", stage_name), style),
            Span::styled(jobs_info, Style::default().fg(Color::DarkGray)),
        ])));
        line_idx += 1;

        if expanded {
            for (ji, job) in stage.jobs.iter().enumerate() {
                let is_job_selected = line_idx == app.tree_state.selected_line;
                let job_expanded = si < app.tree_state.expanded_jobs.len()
                    && ji < app.tree_state.expanded_jobs[si].len()
                    && app.tree_state.expanded_jobs[si][ji];

                let job_arrow = if job_expanded { "  v " } else { "  > " };
                let job_name = job
                    .display_name
                    .as_deref()
                    .or(job.identifier())
                    .unwrap_or("job");
                let steps_info = format!(" ({} steps)", job.steps.len());

                let job_style = if is_job_selected {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Green)
                };

                items.push(ListItem::new(Line::from(vec![
                    Span::styled(job_arrow, job_style),
                    Span::styled(format!("Job: {}", job_name), job_style),
                    Span::styled(steps_info, Style::default().fg(Color::DarkGray)),
                ])));
                line_idx += 1;

                if job_expanded {
                    for step in &job.steps {
                        let is_step_selected = line_idx == app.tree_state.selected_line;
                        let step_name = step
                            .display_name
                            .as_deref()
                            .or(step.name.as_deref())
                            .unwrap_or("step");

                        let step_type = match &step.action {
                            pipeline_service::parser::models::StepAction::Script(_) => "[script]",
                            pipeline_service::parser::models::StepAction::Bash(_) => "[bash]",
                            pipeline_service::parser::models::StepAction::Pwsh(_) => "[pwsh]",
                            pipeline_service::parser::models::StepAction::PowerShell(_) => "[ps]",
                            pipeline_service::parser::models::StepAction::Checkout(_) => {
                                "[checkout]"
                            }
                            pipeline_service::parser::models::StepAction::Task(_) => "[task]",
                            pipeline_service::parser::models::StepAction::Template(_) => {
                                "[template]"
                            }
                            pipeline_service::parser::models::StepAction::Download(_) => {
                                "[download]"
                            }
                            pipeline_service::parser::models::StepAction::Publish(_) => "[publish]",
                            pipeline_service::parser::models::StepAction::GetPackage(_) => "[pkg]",
                            pipeline_service::parser::models::StepAction::ReviewApp(_) => {
                                "[review]"
                            }
                        };

                        let step_style = if is_step_selected {
                            Style::default()
                                .fg(Color::Black)
                                .bg(Color::Cyan)
                                .add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(Color::White)
                        };

                        items.push(ListItem::new(Line::from(vec![
                            Span::styled("      ", step_style),
                            Span::styled(step_name, step_style),
                            Span::styled(
                                format!(" {}", step_type),
                                Style::default().fg(Color::DarkGray),
                            ),
                        ])));
                        line_idx += 1;
                    }
                }
            }
        }
    }

    let tree = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Pipeline Structure")
            .border_style(Style::default().fg(Color::Cyan)),
    );
    frame.render_widget(tree, chunks[1]);

    components::render_footer(
        "j/k: Navigate | Enter/Space: Expand | x: Execute | v: Variables | q/Esc: Back",
        frame,
        chunks[2],
    );
}
