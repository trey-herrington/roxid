mod components;
mod layout;

use ratatui::Frame;

use crate::app::{App, AppState};

pub fn render(app: &App, frame: &mut Frame) {
    match app.state {
        AppState::PipelineList => render_pipeline_list(app, frame),
        AppState::ExecutingPipeline => render_execution(app, frame),
    }
}

fn render_pipeline_list(app: &App, frame: &mut Frame) {
    if app.discovery_errors.is_empty() {
        let chunks = layout::create_layout(frame.area());
        components::render_header("Pipeline Runner", frame, chunks[0]);
        components::render_pipeline_list(&app.pipelines, app.selected_index, frame, chunks[1]);
        components::render_footer("↑/↓: Navigate | Enter: Execute | q: Quit", frame, chunks[2]);
    } else {
        let chunks = layout::create_layout_with_errors(frame.area());
        components::render_header("Pipeline Runner", frame, chunks[0]);
        components::render_pipeline_list(&app.pipelines, app.selected_index, frame, chunks[1]);
        components::render_discovery_errors(&app.discovery_errors, frame, chunks[2]);
        components::render_footer("↑/↓: Navigate | Enter: Execute | q: Quit", frame, chunks[3]);
    }
}

fn render_execution(app: &App, frame: &mut Frame) {
    let chunks = layout::create_layout(frame.area());

    if let Some(exec_state) = &app.execution_state {
        components::render_header(
            &format!("Executing: {}", exec_state.pipeline_name),
            frame,
            chunks[0],
        );
        components::render_execution_view(exec_state, frame, chunks[1]);

        let footer_text = if exec_state.is_complete {
            "Press q or Esc to return to pipeline list"
        } else {
            "Pipeline executing..."
        };
        components::render_footer(footer_text, frame, chunks[2]);
    }
}
