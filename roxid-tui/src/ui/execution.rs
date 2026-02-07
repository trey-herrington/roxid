use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::app::{App, OutputKind, StageProgress};
use crate::ui::{components, layout};

use pipeline_service::parser::models::{JobStatus, StageStatus};

pub fn render(app: &App, frame: &mut Frame) {
    let chunks = layout::create_execution_layout(frame.area());

    let exec = match &app.execution_state {
        Some(exec) => exec,
        None => {
            components::render_header("Execution", frame, chunks[0]);
            return;
        }
    };

    // Header
    components::render_header(
        &format!("Executing: {}", exec.pipeline_name),
        frame,
        chunks[0],
    );

    // Progress bar
    let progress = app.execution_progress();
    let label = if exec.is_complete {
        if exec.success {
            format!(
                "Completed in {}",
                exec.duration
                    .map(|d| components::format_duration(d.as_secs_f64()))
                    .unwrap_or_default()
            )
        } else {
            "Failed".to_string()
        }
    } else {
        let completed_stages = exec
            .stages
            .iter()
            .filter(|s| {
                matches!(
                    s.status,
                    StageStatus::Succeeded | StageStatus::Failed | StageStatus::Skipped
                )
            })
            .count();
        format!("Stage {}/{}", completed_stages, exec.stages.len())
    };

    let gauge_color = if exec.is_complete {
        if exec.success {
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
        .ratio(progress);
    frame.render_widget(gauge, chunks[1]);

    // Split main area: stage list on left, output on right
    let main_sections = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(chunks[2]);

    // Stage/job progress panel
    render_stage_panel(&exec.stages, frame, main_sections[0]);

    // Output panel
    render_output_panel(&exec.output_lines, frame, main_sections[1]);

    // Footer
    let footer = if exec.is_complete {
        "l: View Logs | q/Esc: Back"
    } else {
        "l: View Logs | Executing..."
    };
    components::render_footer(footer, frame, chunks[3]);
}

fn render_stage_panel(stages: &[StageProgress], frame: &mut Frame, area: ratatui::layout::Rect) {
    let mut items: Vec<ListItem> = Vec::new();

    for stage in stages {
        let (symbol, color) = stage_status_display(&stage.status);
        let duration_str = stage
            .duration
            .map(|d| format!(" ({})", components::format_duration(d.as_secs_f64())))
            .unwrap_or_default();
        let stage_label = stage.display_name.as_deref().unwrap_or(&stage.name);

        items.push(ListItem::new(Line::from(vec![
            Span::styled(format!("{} ", symbol), Style::default().fg(color)),
            Span::styled(
                stage_label,
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            ),
            Span::styled(duration_str, Style::default().fg(Color::DarkGray)),
        ])));

        // Show jobs under the stage
        for job in &stage.jobs {
            let (job_sym, job_color) = job_status_display(&job.status);
            let job_label = job.display_name.as_deref().unwrap_or(&job.name);
            let job_dur = job
                .duration
                .map(|d| format!(" ({})", components::format_duration(d.as_secs_f64())))
                .unwrap_or_default();

            items.push(ListItem::new(Line::from(vec![
                Span::raw("  "),
                Span::styled(format!("{} ", job_sym), Style::default().fg(job_color)),
                Span::styled(job_label, Style::default().fg(job_color)),
                Span::styled(job_dur, Style::default().fg(Color::DarkGray)),
            ])));
        }
    }

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Stages")
            .border_style(Style::default().fg(Color::Cyan)),
    );
    frame.render_widget(list, area);
}

fn render_output_panel(
    lines: &[crate::app::OutputLine],
    frame: &mut Frame,
    area: ratatui::layout::Rect,
) {
    let visible_height = area.height.saturating_sub(2) as usize;
    let start = lines.len().saturating_sub(visible_height);

    let visible_lines: Vec<Line> = lines
        .iter()
        .skip(start)
        .map(|line| {
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
            let modifier = match line.kind {
                OutputKind::StageHeader | OutputKind::Success | OutputKind::Failure => {
                    Modifier::BOLD
                }
                _ => Modifier::empty(),
            };
            Line::from(Span::styled(
                &line.text,
                Style::default().fg(color).add_modifier(modifier),
            ))
        })
        .collect();

    let output = Paragraph::new(visible_lines)
        .block(Block::default().borders(Borders::ALL).title("Output"))
        .wrap(Wrap { trim: false });
    frame.render_widget(output, area);
}

fn stage_status_display(status: &StageStatus) -> (&str, Color) {
    match status {
        StageStatus::Pending => (".", Color::DarkGray),
        StageStatus::Running => ("~", Color::Cyan),
        StageStatus::Succeeded => ("O", Color::Green),
        StageStatus::SucceededWithIssues => ("!", Color::Yellow),
        StageStatus::Failed => ("X", Color::Red),
        StageStatus::Canceled => ("-", Color::DarkGray),
        StageStatus::Skipped => ("-", Color::DarkGray),
    }
}

fn job_status_display(status: &JobStatus) -> (&str, Color) {
    match status {
        JobStatus::Pending => (".", Color::DarkGray),
        JobStatus::Running => ("~", Color::Cyan),
        JobStatus::Succeeded => ("O", Color::Green),
        JobStatus::SucceededWithIssues => ("!", Color::Yellow),
        JobStatus::Failed => ("X", Color::Red),
        JobStatus::Canceled => ("-", Color::DarkGray),
        JobStatus::Skipped => ("-", Color::DarkGray),
    }
}
