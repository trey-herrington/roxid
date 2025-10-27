use color_eyre::Result;
use ratatui::DefaultTerminal;
use service::pipeline::{ExecutionEvent, PipelineParser, PipelineExecutor, ExecutionContext};
use std::path::PathBuf;
use tokio::sync::mpsc::UnboundedReceiver;

use crate::events::EventHandler;
use crate::ui;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppState {
    PipelineList,
    ExecutingPipeline,
}

#[derive(Debug)]
pub struct App {
    pub state: AppState,
    pub pipelines: Vec<PipelineInfo>,
    pub selected_index: usize,
    pub should_quit: bool,
    pub execution_state: Option<ExecutionState>,
    pub event_receiver: Option<UnboundedReceiver<ExecutionEvent>>,
}

#[derive(Debug, Clone)]
pub struct PipelineInfo {
    pub name: String,
    pub path: PathBuf,
    pub description: Option<String>,
}

#[derive(Debug)]
pub struct ExecutionState {
    pub pipeline_name: String,
    pub total_steps: usize,
    pub current_step: usize,
    pub output_lines: Vec<String>,
    pub is_complete: bool,
    pub success: bool,
}

impl App {
    pub fn new() -> Self {
        let pipelines = Self::discover_pipelines();
        Self {
            state: AppState::PipelineList,
            pipelines,
            selected_index: 0,
            should_quit: false,
            execution_state: None,
            event_receiver: None,
        }
    }

    pub async fn run(&mut self, mut terminal: DefaultTerminal) -> Result<()> {
        while !self.should_quit {
            terminal.draw(|frame| ui::render(self, frame))?;
            self.handle_events()?;
            self.process_execution_events();
        }
        Ok(())
    }

    fn discover_pipelines() -> Vec<PipelineInfo> {
        let mut pipelines = Vec::new();
        
        if let Ok(entries) = std::fs::read_dir(".") {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(ext) = path.extension() {
                    if ext == "yaml" || ext == "yml" {
                        if let Ok(pipeline) = PipelineParser::from_file(&path) {
                            pipelines.push(PipelineInfo {
                                name: pipeline.name.clone(),
                                path: path.clone(),
                                description: pipeline.description.clone(),
                            });
                        }
                    }
                }
            }
        }
        
        pipelines.sort_by(|a, b| a.name.cmp(&b.name));
        pipelines
    }

    pub fn move_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    pub fn move_down(&mut self) {
        if self.selected_index < self.pipelines.len().saturating_sub(1) {
            self.selected_index += 1;
        }
    }

    pub fn execute_selected_pipeline(&mut self) -> Result<()> {
        if self.pipelines.is_empty() {
            return Ok(());
        }

        let pipeline_info = &self.pipelines[self.selected_index];
        let pipeline = PipelineParser::from_file(&pipeline_info.path)?;
        
        self.state = AppState::ExecutingPipeline;
        self.execution_state = Some(ExecutionState {
            pipeline_name: pipeline.name.clone(),
            total_steps: pipeline.steps.len(),
            current_step: 0,
            output_lines: Vec::new(),
            is_complete: false,
            success: false,
        });

        let working_dir = std::env::current_dir()?.to_string_lossy().to_string();
        let context = ExecutionContext::new(pipeline.name.clone(), working_dir);
        let executor = PipelineExecutor::new(context);

        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        self.event_receiver = Some(rx);
        
        let pipeline_clone = pipeline.clone();
        tokio::spawn(async move {
            executor.execute(pipeline_clone, Some(tx)).await;
        });

        Ok(())
    }

    pub fn process_execution_events(&mut self) {
        let Some(rx) = &mut self.event_receiver else {
            return;
        };

        let mut should_close_receiver = false;

        while let Ok(event) = rx.try_recv() {
            if let Some(exec_state) = &mut self.execution_state {
                match event {
                    ExecutionEvent::PipelineStarted { name } => {
                        exec_state.output_lines.push(format!("Pipeline '{}' started", name));
                    }
                    ExecutionEvent::StepStarted { step_name, step_index } => {
                        exec_state.current_step = step_index + 1;
                        exec_state.output_lines.push(format!("\n[Step {}/{}] {}", 
                            step_index + 1, exec_state.total_steps, step_name));
                    }
                    ExecutionEvent::StepOutput { output, .. } => {
                        for line in output.lines() {
                            exec_state.output_lines.push(format!("  {}", line));
                        }
                    }
                    ExecutionEvent::StepCompleted { result, .. } => {
                        let status = match result.status {
                            service::pipeline::StepStatus::Success => "✓",
                            service::pipeline::StepStatus::Failed => "✗",
                            _ => "?",
                        };
                        exec_state.output_lines.push(format!("  {} Completed in {:.2}s", 
                            status, result.duration.as_secs_f64()));
                    }
                    ExecutionEvent::PipelineCompleted { success, total_steps, failed_steps } => {
                        exec_state.is_complete = true;
                        exec_state.success = success;
                        if success {
                            exec_state.output_lines.push(format!("\n✓ Pipeline completed successfully! ({} steps)", total_steps));
                        } else {
                            exec_state.output_lines.push(format!("\n✗ Pipeline failed! ({} of {} steps failed)", 
                                failed_steps, total_steps));
                        }
                        should_close_receiver = true;
                    }
                }
            }
        }

        if should_close_receiver {
            self.event_receiver = None;
        }
    }

    pub fn back_to_list(&mut self) {
        self.state = AppState::PipelineList;
        self.execution_state = None;
        self.event_receiver = None;
    }

    pub fn quit(&mut self) {
        self.should_quit = true;
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}
