use color_eyre::Result;
use ratatui::DefaultTerminal;
use std::path::PathBuf;
use tokio::sync::mpsc::UnboundedReceiver;

use crate::events::EventHandler;
use crate::ui;

use pipeline_service::grpc::proto::{
    pipeline_service_client::PipelineServiceClient, parse_pipeline_request, ExecutePipelineRequest,
    ParsePipelineRequest, ExecutionEvent, execution_event::Event, StepStatus,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppState {
    PipelineList,
    ExecutingPipeline,
}

pub struct App {
    pub state: AppState,
    pub pipelines: Vec<PipelineInfo>,
    pub selected_index: usize,
    pub should_quit: bool,
    pub execution_state: Option<ExecutionState>,
    pub event_receiver: Option<UnboundedReceiver<ExecutionEvent>>,
    pub discovery_errors: Vec<DiscoveryError>,
    grpc_client: Option<PipelineServiceClient<tonic::transport::Channel>>,
    pending_execution: bool,
}

#[derive(Debug, Clone)]
pub struct DiscoveryError {
    pub file_name: String,
    pub error: String,
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
    pub async fn new() -> Result<Self> {
        let mut client = PipelineServiceClient::connect("http://[::1]:50051").await?;
        let (pipelines, discovery_errors) = Self::discover_pipelines(&mut client).await;
        Ok(Self {
            state: AppState::PipelineList,
            pipelines,
            selected_index: 0,
            should_quit: false,
            execution_state: None,
            event_receiver: None,
            discovery_errors,
            grpc_client: Some(client),
            pending_execution: false,
        })
    }

    pub async fn run(&mut self, mut terminal: DefaultTerminal) -> Result<()> {
        while !self.should_quit {
            terminal.draw(|frame| ui::render(self, frame))?;
            self.handle_events()?;
            
            // Handle pending execution request
            if self.pending_execution {
                self.pending_execution = false;
                self.execute_selected_pipeline().await?;
            }
            
            self.process_execution_events().await;
        }
        Ok(())
    }

    async fn discover_pipelines(
        client: &mut PipelineServiceClient<tonic::transport::Channel>,
    ) -> (Vec<PipelineInfo>, Vec<DiscoveryError>) {
        let mut pipelines = Vec::new();
        let mut errors = Vec::new();

        // Explicitly use current working directory where user ran the command
        let current_dir = match std::env::current_dir() {
            Ok(dir) => dir,
            Err(e) => {
                errors.push(DiscoveryError {
                    file_name: "<current directory>".to_string(),
                    error: format!("Failed to get current directory: {}", e),
                });
                return (pipelines, errors);
            }
        };

        let entries = match std::fs::read_dir(&current_dir) {
            Ok(entries) => entries,
            Err(e) => {
                errors.push(DiscoveryError {
                    file_name: current_dir.display().to_string(),
                    error: format!("Failed to read directory: {}", e),
                });
                return (pipelines, errors);
            }
        };

        for entry in entries.flatten() {
            let path = entry.path();
            
            // Skip directories, only process files
            if !path.is_file() {
                continue;
            }
            
            if let Some(ext) = path.extension() {
                if ext == "yaml" || ext == "yml" {
                    let file_name = path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown")
                        .to_string();
                    
                    // Use absolute path for gRPC request
                    let absolute_path = std::fs::canonicalize(&path)
                        .unwrap_or_else(|_| path.clone())
                        .to_string_lossy()
                        .to_string();
                    
                    let parse_request = ParsePipelineRequest {
                        source: Some(parse_pipeline_request::Source::FilePath(
                            absolute_path,
                        )),
                    };

                    match client.parse_pipeline(parse_request).await {
                        Ok(response) => {
                            if let Some(pipeline) = response.into_inner().pipeline {
                                pipelines.push(PipelineInfo {
                                    name: pipeline.name.clone(),
                                    path: path.clone(),
                                    description: pipeline.description.clone(),
                                });
                            } else {
                                errors.push(DiscoveryError {
                                    file_name,
                                    error: "No pipeline returned from service".to_string(),
                                });
                            }
                        }
                        Err(e) => {
                            // Extract just the meaningful error message, strip the gRPC prefix
                            let full_msg = format!("{}", e);
                            
                            // Try to extract just the message part
                            let error_msg = if let Some(start) = full_msg.find("message: \"") {
                                // Extract message between quotes
                                let msg_start = start + 10; // length of "message: \""
                                if let Some(end) = full_msg[msg_start..].find("\", details") {
                                    let extracted = full_msg[msg_start..msg_start + end].to_string();
                                    // Further clean up: remove "Failed to parse pipeline from file: " prefix
                                    if let Some(yaml_err_start) = extracted.find("YAML error:") {
                                        extracted[yaml_err_start..].to_string()
                                    } else {
                                        extracted
                                    }
                                } else if let Some(end) = full_msg[msg_start..].find('\"') {
                                    let extracted = full_msg[msg_start..msg_start + end].to_string();
                                    if let Some(yaml_err_start) = extracted.find("YAML error:") {
                                        extracted[yaml_err_start..].to_string()
                                    } else {
                                        extracted
                                    }
                                } else {
                                    full_msg.clone()
                                }
                            } else {
                                full_msg.clone()
                            };
                            
                            errors.push(DiscoveryError {
                                file_name,
                                error: error_msg,
                            });
                        }
                    }
                }
            }
        }

        pipelines.sort_by(|a, b| a.name.cmp(&b.name));
        (pipelines, errors)
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
    
    pub fn request_execute_pipeline(&mut self) {
        self.pending_execution = true;
    }

    pub async fn execute_selected_pipeline(&mut self) -> Result<()> {
        if self.pipelines.is_empty() {
            return Ok(());
        }

        let pipeline_info = &self.pipelines[self.selected_index];

        let client = self
            .grpc_client
            .as_mut()
            .ok_or_else(|| color_eyre::eyre::eyre!("gRPC client not initialized"))?;

        // Use absolute path for gRPC request
        let absolute_path = std::fs::canonicalize(&pipeline_info.path)
            .unwrap_or_else(|_| pipeline_info.path.clone())
            .to_string_lossy()
            .to_string();

        let parse_request = ParsePipelineRequest {
            source: Some(parse_pipeline_request::Source::FilePath(absolute_path)),
        };

        let response = client.parse_pipeline(parse_request).await?;
        let pipeline = response
            .into_inner()
            .pipeline
            .ok_or_else(|| color_eyre::eyre::eyre!("No pipeline returned"))?;

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

        let execute_request = ExecutePipelineRequest {
            pipeline: Some(pipeline),
            working_dir,
        };

        let stream = client.execute_pipeline(execute_request).await?.into_inner();

        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        self.event_receiver = Some(rx);

        tokio::spawn(async move {
            let mut stream = stream;
            while let Ok(Some(event)) = stream.message().await {
                if tx.send(event).is_err() {
                    break;
                }
            }
        });

        Ok(())
    }

    pub async fn process_execution_events(&mut self) {
        let Some(rx) = &mut self.event_receiver else {
            return;
        };

        let mut should_close_receiver = false;

        while let Ok(event) = rx.try_recv() {
            if let Some(exec_state) = &mut self.execution_state {
                if let Some(e) = event.event {
                    match e {
                        Event::PipelineStarted(started) => {
                            exec_state
                                .output_lines
                                .push(format!("Pipeline '{}' started", started.name));
                        }
                        Event::StepStarted(started) => {
                            exec_state.current_step = started.step_index as usize + 1;
                            exec_state.output_lines.push(format!(
                                "\n[Step {}/{}] {}",
                                started.step_index + 1,
                                exec_state.total_steps,
                                started.step_name
                            ));
                        }
                        Event::StepOutput(output) => {
                            for line in output.output.lines() {
                                exec_state.output_lines.push(format!("  {}", line));
                            }
                        }
                        Event::StepCompleted(completed) => {
                            if let Some(result) = completed.result {
                                let status_enum = StepStatus::try_from(result.status)
                                    .unwrap_or(StepStatus::Pending);
                                let status = match status_enum {
                                    StepStatus::Success => "✓",
                                    StepStatus::Failed => "✗",
                                    _ => "?",
                                };
                                exec_state.output_lines.push(format!(
                                    "  {} Completed in {:.2}s",
                                    status,
                                    result.duration_ms as f64 / 1000.0
                                ));
                            }
                        }
                        Event::PipelineCompleted(completed) => {
                            exec_state.is_complete = true;
                            exec_state.success = completed.success;
                            if completed.success {
                                exec_state.output_lines.push(format!(
                                    "\n✓ Pipeline completed successfully! ({} steps)",
                                    completed.total_steps
                                ));
                            } else {
                                exec_state.output_lines.push(format!(
                                    "\n✗ Pipeline failed! ({} of {} steps failed)",
                                    completed.failed_steps, completed.total_steps
                                ));
                            }
                            should_close_receiver = true;
                        }
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
