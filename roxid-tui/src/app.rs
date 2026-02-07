use crate::events::EventHandler;
use crate::ui;

use color_eyre::Result;
use ratatui::DefaultTerminal;

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use pipeline_service::execution::events::{progress_channel, ProgressReceiver};
use pipeline_service::parser::models::{
    ExecutionContext, JobStatus, StageStatus, StepStatus, Variable,
};
use pipeline_service::{
    normalize_pipeline, AzureParser, ExecutionEvent, ExecutionResult, Pipeline, PipelineExecutor,
    TestFileParser, TestRunner, TestSuiteResult,
};

// =============================================================================
// Application States
// =============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppState {
    /// Browse available pipelines
    PipelineList,
    /// Show stages/jobs/steps tree for selected pipeline
    PipelineDetail,
    /// Display running pipeline with real-time progress
    ExecutingPipeline,
    /// Scrollable log viewer for execution output
    ExecutionLog,
    /// View test suite results
    TestResults,
    /// Edit variables and parameters before execution
    VariableEditor,
}

// =============================================================================
// Application
// =============================================================================

pub struct App {
    pub state: AppState,
    pub previous_states: Vec<AppState>,
    pub pipelines: Vec<PipelineInfo>,
    pub selected_index: usize,
    pub should_quit: bool,
    pub discovery_errors: Vec<DiscoveryError>,

    // Pipeline detail state
    pub tree_state: TreeState,

    // Execution state
    pub execution_state: Option<ExecutionState>,
    pub event_receiver: Option<ProgressReceiver>,
    pub pending_execution: bool,

    // Log viewer state
    pub log_viewer: LogViewerState,

    // Test results state
    pub test_state: Option<TestState>,
    pub pending_test_run: bool,

    // Variable editor state
    pub variable_editor: Option<VariableEditorState>,
}

// =============================================================================
// Pipeline Info (discovered pipelines)
// =============================================================================

#[derive(Debug, Clone)]
pub struct PipelineInfo {
    pub name: String,
    #[allow(dead_code)]
    pub path: PathBuf,
    pub pipeline: Pipeline,
    pub stages_count: usize,
    pub jobs_count: usize,
    pub steps_count: usize,
}

#[derive(Debug, Clone)]
pub struct DiscoveryError {
    pub file_name: String,
    pub error: String,
}

// =============================================================================
// Tree State (for PipelineDetail view)
// =============================================================================

#[derive(Debug, Clone, Default)]
pub struct TreeState {
    pub selected_line: usize,
    pub expanded_stages: Vec<bool>,
    pub expanded_jobs: Vec<Vec<bool>>,
    pub total_lines: usize,
}

impl TreeState {
    pub fn from_pipeline(pipeline: &Pipeline) -> Self {
        let expanded_stages = vec![false; pipeline.stages.len()];
        let expanded_jobs = pipeline
            .stages
            .iter()
            .map(|s| vec![false; s.jobs.len()])
            .collect();
        let total_lines = pipeline.stages.len();
        Self {
            selected_line: 0,
            expanded_stages,
            expanded_jobs,
            total_lines,
        }
    }

    /// Recalculate total visible lines based on expansion state
    pub fn recalculate_lines(&mut self, pipeline: &Pipeline) {
        let mut count = 0;
        for (si, stage) in pipeline.stages.iter().enumerate() {
            count += 1; // stage line
            if si < self.expanded_stages.len() && self.expanded_stages[si] {
                for (ji, job) in stage.jobs.iter().enumerate() {
                    count += 1; // job line
                    if ji < self.expanded_jobs[si].len() && self.expanded_jobs[si][ji] {
                        count += job.steps.len(); // step lines
                    }
                }
            }
        }
        self.total_lines = count;
    }
}

// =============================================================================
// Execution State
// =============================================================================

#[derive(Debug)]
pub struct ExecutionState {
    pub pipeline_name: String,
    pub stages: Vec<StageProgress>,
    pub output_lines: Vec<OutputLine>,
    pub is_complete: bool,
    pub success: bool,
    pub duration: Option<Duration>,
    #[allow(dead_code)]
    pub result: Option<ExecutionResult>,
}

#[derive(Debug, Clone)]
pub struct StageProgress {
    pub name: String,
    pub display_name: Option<String>,
    pub status: StageStatus,
    pub jobs: Vec<JobProgress>,
    pub duration: Option<Duration>,
}

#[derive(Debug, Clone)]
pub struct JobProgress {
    pub name: String,
    pub display_name: Option<String>,
    pub status: JobStatus,
    pub steps: Vec<StepProgress>,
    pub duration: Option<Duration>,
}

#[derive(Debug, Clone)]
pub struct StepProgress {
    #[allow(dead_code)]
    pub name: Option<String>,
    #[allow(dead_code)]
    pub display_name: Option<String>,
    pub status: StepStatus,
    pub duration: Option<Duration>,
    pub exit_code: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct OutputLine {
    pub text: String,
    pub kind: OutputKind,
    pub stage_name: Option<String>,
    pub job_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutputKind {
    Info,
    Output,
    Error,
    Success,
    Failure,
    Warning,
    StepHeader,
    StageHeader,
    JobHeader,
}

// =============================================================================
// Log Viewer State
// =============================================================================

#[derive(Debug, Clone, Default)]
pub struct LogViewerState {
    pub scroll_offset: usize,
    pub search_query: String,
    pub search_active: bool,
    pub search_matches: Vec<usize>,
    pub current_match: usize,
    pub filter_stage: Option<String>,
    pub filter_job: Option<String>,
}

impl LogViewerState {
    pub fn reset(&mut self) {
        self.scroll_offset = 0;
        self.search_query.clear();
        self.search_active = false;
        self.search_matches.clear();
        self.current_match = 0;
        self.filter_stage = None;
        self.filter_job = None;
    }
}

// =============================================================================
// Test State
// =============================================================================

#[derive(Debug)]
pub struct TestState {
    pub results: Vec<TestSuiteResult>,
    #[allow(dead_code)]
    pub selected_suite: usize,
    pub selected_test: usize,
    pub is_running: bool,
    pub total_passed: usize,
    pub total_failed: usize,
    pub total_skipped: usize,
}

// =============================================================================
// Variable Editor State
// =============================================================================

#[derive(Debug, Clone)]
pub struct VariableEditorState {
    pub variables: Vec<EditableVariable>,
    pub parameters: Vec<EditableParameter>,
    pub selected_index: usize,
    pub editing: bool,
    pub edit_buffer: String,
    pub in_parameters_section: bool,
}

#[derive(Debug, Clone)]
pub struct EditableVariable {
    pub name: String,
    pub value: String,
    pub readonly: bool,
}

#[derive(Debug, Clone)]
pub struct EditableParameter {
    pub name: String,
    pub display_name: Option<String>,
    pub value: String,
    pub default: Option<String>,
}

// =============================================================================
// App Implementation
// =============================================================================

impl App {
    pub fn new() -> Result<Self> {
        let (pipelines, discovery_errors) = Self::discover_pipelines();
        Ok(Self {
            state: AppState::PipelineList,
            previous_states: Vec::new(),
            pipelines,
            selected_index: 0,
            should_quit: false,
            discovery_errors,
            tree_state: TreeState::default(),
            execution_state: None,
            event_receiver: None,
            pending_execution: false,
            log_viewer: LogViewerState::default(),
            test_state: None,
            pending_test_run: false,
            variable_editor: None,
        })
    }

    pub async fn run(&mut self, mut terminal: DefaultTerminal) -> Result<()> {
        while !self.should_quit {
            terminal.draw(|frame| ui::render(self, frame))?;
            self.handle_events()?;

            // Handle pending execution
            if self.pending_execution {
                self.pending_execution = false;
                self.execute_selected_pipeline().await;
            }

            // Handle pending test run
            if self.pending_test_run {
                self.pending_test_run = false;
                self.run_tests().await;
            }

            // Process execution events
            self.process_execution_events();
        }
        Ok(())
    }

    // =========================================================================
    // Pipeline Discovery
    // =========================================================================

    fn discover_pipelines() -> (Vec<PipelineInfo>, Vec<DiscoveryError>) {
        let mut pipelines = Vec::new();
        let mut errors = Vec::new();

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

                    match AzureParser::parse_file(&path) {
                        Ok(raw_pipeline) => {
                            let pipeline = normalize_pipeline(raw_pipeline);
                            let name = pipeline.name.clone().unwrap_or_else(|| file_name.clone());

                            let stages_count = pipeline.stages.len();
                            let jobs_count: usize =
                                pipeline.stages.iter().map(|s| s.jobs.len()).sum();
                            let steps_count: usize = pipeline
                                .stages
                                .iter()
                                .flat_map(|s| &s.jobs)
                                .map(|j| j.steps.len())
                                .sum();

                            pipelines.push(PipelineInfo {
                                name,
                                path: path.clone(),
                                pipeline,
                                stages_count,
                                jobs_count,
                                steps_count,
                            });
                        }
                        Err(e) => {
                            errors.push(DiscoveryError {
                                file_name,
                                error: e.message,
                            });
                        }
                    }
                }
            }
        }

        pipelines.sort_by(|a, b| a.name.cmp(&b.name));
        (pipelines, errors)
    }

    // =========================================================================
    // Navigation
    // =========================================================================

    pub fn move_up(&mut self) {
        match self.state {
            AppState::PipelineList => {
                if self.selected_index > 0 {
                    self.selected_index -= 1;
                }
            }
            AppState::PipelineDetail => {
                if self.tree_state.selected_line > 0 {
                    self.tree_state.selected_line -= 1;
                }
            }
            AppState::VariableEditor => {
                if let Some(editor) = &mut self.variable_editor {
                    if editor.selected_index > 0 {
                        editor.selected_index -= 1;
                    }
                }
            }
            AppState::TestResults => {
                if let Some(test_state) = &mut self.test_state {
                    if test_state.selected_test > 0 {
                        test_state.selected_test -= 1;
                    }
                }
            }
            AppState::ExecutionLog => {
                if self.log_viewer.scroll_offset > 0 {
                    self.log_viewer.scroll_offset -= 1;
                }
            }
            _ => {}
        }
    }

    pub fn move_down(&mut self) {
        match self.state {
            AppState::PipelineList => {
                if self.selected_index < self.pipelines.len().saturating_sub(1) {
                    self.selected_index += 1;
                }
            }
            AppState::PipelineDetail => {
                if self.tree_state.selected_line < self.tree_state.total_lines.saturating_sub(1) {
                    self.tree_state.selected_line += 1;
                }
            }
            AppState::VariableEditor => {
                if let Some(editor) = &mut self.variable_editor {
                    let total = editor.variables.len() + editor.parameters.len();
                    if editor.selected_index < total.saturating_sub(1) {
                        editor.selected_index += 1;
                    }
                }
            }
            AppState::TestResults => {
                if let Some(test_state) = &mut self.test_state {
                    let total_tests: usize =
                        test_state.results.iter().map(|s| s.results.len()).sum();
                    if test_state.selected_test < total_tests.saturating_sub(1) {
                        test_state.selected_test += 1;
                    }
                }
            }
            AppState::ExecutionLog => {
                if let Some(exec) = &self.execution_state {
                    let max = exec.output_lines.len().saturating_sub(1);
                    if self.log_viewer.scroll_offset < max {
                        self.log_viewer.scroll_offset += 1;
                    }
                }
            }
            _ => {}
        }
    }

    pub fn page_up(&mut self) {
        match self.state {
            AppState::ExecutionLog => {
                self.log_viewer.scroll_offset = self.log_viewer.scroll_offset.saturating_sub(20);
            }
            _ => {
                for _ in 0..10 {
                    self.move_up();
                }
            }
        }
    }

    pub fn page_down(&mut self) {
        match self.state {
            AppState::ExecutionLog => {
                if let Some(exec) = &self.execution_state {
                    let max = exec.output_lines.len().saturating_sub(1);
                    self.log_viewer.scroll_offset = (self.log_viewer.scroll_offset + 20).min(max);
                }
            }
            _ => {
                for _ in 0..10 {
                    self.move_down();
                }
            }
        }
    }

    pub fn push_state(&mut self, new_state: AppState) {
        let old = self.state.clone();
        self.previous_states.push(old);
        self.state = new_state;
    }

    pub fn go_back(&mut self) {
        if let Some(prev) = self.previous_states.pop() {
            self.state = prev;
        } else {
            self.should_quit = true;
        }
    }

    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    // =========================================================================
    // Pipeline Detail
    // =========================================================================

    pub fn enter_pipeline_detail(&mut self) {
        if self.pipelines.is_empty() {
            return;
        }
        let pipeline = &self.pipelines[self.selected_index].pipeline;
        self.tree_state = TreeState::from_pipeline(pipeline);
        self.push_state(AppState::PipelineDetail);
    }

    pub fn toggle_tree_node(&mut self) {
        if self.pipelines.is_empty() {
            return;
        }
        let pipeline = &self.pipelines[self.selected_index].pipeline;

        // Walk the visible lines to find what the selected line corresponds to
        let mut line = 0;
        for (si, stage) in pipeline.stages.iter().enumerate() {
            if line == self.tree_state.selected_line {
                // Toggle this stage
                if si < self.tree_state.expanded_stages.len() {
                    self.tree_state.expanded_stages[si] = !self.tree_state.expanded_stages[si];
                }
                self.tree_state.recalculate_lines(pipeline);
                return;
            }
            line += 1;

            if si < self.tree_state.expanded_stages.len() && self.tree_state.expanded_stages[si] {
                for (ji, job) in stage.jobs.iter().enumerate() {
                    if line == self.tree_state.selected_line {
                        // Toggle this job
                        if ji < self.tree_state.expanded_jobs[si].len() {
                            self.tree_state.expanded_jobs[si][ji] =
                                !self.tree_state.expanded_jobs[si][ji];
                        }
                        self.tree_state.recalculate_lines(pipeline);
                        return;
                    }
                    line += 1;

                    if ji < self.tree_state.expanded_jobs[si].len()
                        && self.tree_state.expanded_jobs[si][ji]
                    {
                        for _step in &job.steps {
                            if line == self.tree_state.selected_line {
                                // Steps are leaf nodes, no toggle
                                return;
                            }
                            line += 1;
                        }
                    }
                }
            }
        }
    }

    // =========================================================================
    // Variable Editor
    // =========================================================================

    pub fn open_variable_editor(&mut self) {
        if self.pipelines.is_empty() {
            return;
        }
        let pipeline = &self.pipelines[self.selected_index].pipeline;

        let variables: Vec<EditableVariable> = pipeline
            .variables
            .iter()
            .filter_map(|v| match v {
                Variable::KeyValue {
                    name,
                    value,
                    readonly,
                } => Some(EditableVariable {
                    name: name.clone(),
                    value: value.clone(),
                    readonly: *readonly,
                }),
                _ => None,
            })
            .collect();

        let parameters: Vec<EditableParameter> = pipeline
            .parameters
            .iter()
            .map(|p| EditableParameter {
                name: p.name.clone(),
                display_name: p.display_name.clone(),
                value: p
                    .default
                    .as_ref()
                    .map(|v| format!("{:?}", v))
                    .unwrap_or_default(),
                default: p.default.as_ref().map(|v| format!("{:?}", v)),
            })
            .collect();

        self.variable_editor = Some(VariableEditorState {
            variables,
            parameters,
            selected_index: 0,
            editing: false,
            edit_buffer: String::new(),
            in_parameters_section: false,
        });
        self.push_state(AppState::VariableEditor);
    }

    pub fn start_editing_variable(&mut self) {
        if let Some(editor) = &mut self.variable_editor {
            let var_len = editor.variables.len();
            if editor.selected_index < var_len {
                let var = &editor.variables[editor.selected_index];
                if !var.readonly {
                    editor.edit_buffer = var.value.clone();
                    editor.editing = true;
                    editor.in_parameters_section = false;
                }
            } else {
                let param_idx = editor.selected_index - var_len;
                if param_idx < editor.parameters.len() {
                    editor.edit_buffer = editor.parameters[param_idx].value.clone();
                    editor.editing = true;
                    editor.in_parameters_section = true;
                }
            }
        }
    }

    pub fn confirm_edit(&mut self) {
        if let Some(editor) = &mut self.variable_editor {
            if editor.editing {
                let var_len = editor.variables.len();
                if !editor.in_parameters_section && editor.selected_index < var_len {
                    editor.variables[editor.selected_index].value = editor.edit_buffer.clone();
                } else {
                    let param_idx = editor.selected_index - var_len;
                    if param_idx < editor.parameters.len() {
                        editor.parameters[param_idx].value = editor.edit_buffer.clone();
                    }
                }
                editor.editing = false;
                editor.edit_buffer.clear();
            }
        }
    }

    pub fn cancel_edit(&mut self) {
        if let Some(editor) = &mut self.variable_editor {
            editor.editing = false;
            editor.edit_buffer.clear();
        }
    }

    pub fn edit_buffer_push(&mut self, c: char) {
        if let Some(editor) = &mut self.variable_editor {
            if editor.editing {
                editor.edit_buffer.push(c);
            }
        }
    }

    pub fn edit_buffer_pop(&mut self) {
        if let Some(editor) = &mut self.variable_editor {
            if editor.editing {
                editor.edit_buffer.pop();
            }
        }
    }

    // =========================================================================
    // Execution
    // =========================================================================

    pub fn request_execute_pipeline(&mut self) {
        self.pending_execution = true;
    }

    async fn execute_selected_pipeline(&mut self) {
        if self.pipelines.is_empty() {
            return;
        }

        let pipeline_info = &self.pipelines[self.selected_index];
        let pipeline = pipeline_info.pipeline.clone();
        let pipeline_name = pipeline_info.name.clone();

        // Build execution context with variable overrides
        let working_dir = std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .to_string_lossy()
            .to_string();

        let mut variables = HashMap::new();
        if let Some(editor) = &self.variable_editor {
            for var in &editor.variables {
                variables.insert(var.name.clone(), var.value.clone());
            }
        }

        let mut parameters: HashMap<String, serde_yaml::Value> = HashMap::new();
        if let Some(editor) = &self.variable_editor {
            for param in &editor.parameters {
                if let Ok(val) = serde_yaml::from_str::<serde_yaml::Value>(&param.value) {
                    parameters.insert(param.name.clone(), val);
                } else {
                    parameters.insert(
                        param.name.clone(),
                        serde_yaml::Value::String(param.value.clone()),
                    );
                }
            }
        }

        let context = ExecutionContext::new(pipeline_name.clone(), working_dir)
            .with_variables(variables)
            .with_parameters(parameters);

        // Initialize execution state from the pipeline structure
        let stages: Vec<StageProgress> = pipeline
            .stages
            .iter()
            .map(|s| StageProgress {
                name: s.stage.clone(),
                display_name: s.display_name.clone(),
                status: StageStatus::Pending,
                jobs: s
                    .jobs
                    .iter()
                    .map(|j| JobProgress {
                        name: j.identifier().unwrap_or("job").to_string(),
                        display_name: j.display_name.clone(),
                        status: JobStatus::Pending,
                        steps: j
                            .steps
                            .iter()
                            .map(|step| StepProgress {
                                name: step.name.clone(),
                                display_name: step.display_name.clone(),
                                status: StepStatus::Pending,
                                duration: None,
                                exit_code: None,
                            })
                            .collect(),
                        duration: None,
                    })
                    .collect(),
                duration: None,
            })
            .collect();

        self.execution_state = Some(ExecutionState {
            pipeline_name: pipeline_name.clone(),
            stages,
            output_lines: Vec::new(),
            is_complete: false,
            success: false,
            duration: None,
            result: None,
        });

        self.push_state(AppState::ExecutingPipeline);

        // Create progress channel and spawn executor
        let (tx, rx) = progress_channel();
        self.event_receiver = Some(rx);

        tokio::spawn(async move {
            match PipelineExecutor::from_pipeline(&pipeline) {
                Ok(executor) => {
                    let executor = executor.with_progress(tx);
                    let _result = executor.execute(context).await;
                    // ExecutionResult events have already been sent through the channel
                }
                Err(e) => {
                    let _ = tx.send(ExecutionEvent::Error {
                        message: format!("Failed to build execution graph: {}", e.message),
                        stage_name: None,
                        job_name: None,
                        step_index: None,
                    });
                    let _ = tx.send(ExecutionEvent::PipelineCompleted {
                        pipeline_name,
                        success: false,
                        duration: Duration::from_secs(0),
                    });
                }
            }
        });
    }

    pub fn process_execution_events(&mut self) {
        let Some(rx) = &mut self.event_receiver else {
            return;
        };

        let mut should_close = false;

        while let Ok(event) = rx.try_recv() {
            if let Some(exec) = &mut self.execution_state {
                match &event {
                    ExecutionEvent::PipelineStarted {
                        pipeline_name,
                        total_stages,
                    } => {
                        exec.output_lines.push(OutputLine {
                            text: format!(
                                "Pipeline '{}' started ({} stages)",
                                pipeline_name, total_stages
                            ),
                            kind: OutputKind::Info,
                            stage_name: None,
                            job_name: None,
                        });
                    }

                    ExecutionEvent::PipelineCompleted {
                        success, duration, ..
                    } => {
                        exec.is_complete = true;
                        exec.success = *success;
                        exec.duration = Some(*duration);

                        let text = if *success {
                            format!(
                                "Pipeline completed successfully in {:.2}s",
                                duration.as_secs_f64()
                            )
                        } else {
                            format!("Pipeline failed after {:.2}s", duration.as_secs_f64())
                        };
                        let kind = if *success {
                            OutputKind::Success
                        } else {
                            OutputKind::Failure
                        };
                        exec.output_lines.push(OutputLine {
                            text,
                            kind,
                            stage_name: None,
                            job_name: None,
                        });
                        should_close = true;
                    }

                    ExecutionEvent::StageStarted {
                        stage_name,
                        display_name,
                        total_jobs,
                    } => {
                        if let Some(stage) = exec.stages.iter_mut().find(|s| s.name == *stage_name)
                        {
                            stage.status = StageStatus::Running;
                        }
                        let label = display_name.as_deref().unwrap_or(stage_name);
                        exec.output_lines.push(OutputLine {
                            text: format!("Stage '{}' ({} jobs)", label, total_jobs),
                            kind: OutputKind::StageHeader,
                            stage_name: Some(stage_name.clone()),
                            job_name: None,
                        });
                    }

                    ExecutionEvent::StageCompleted {
                        stage_name,
                        status,
                        duration,
                    } => {
                        if let Some(stage) = exec.stages.iter_mut().find(|s| s.name == *stage_name)
                        {
                            stage.status = status.clone();
                            stage.duration = Some(*duration);
                        }
                        let symbol = match status {
                            StageStatus::Succeeded => "OK",
                            StageStatus::Failed => "FAIL",
                            _ => "DONE",
                        };
                        exec.output_lines.push(OutputLine {
                            text: format!(
                                "  Stage '{}' {} ({:.2}s)",
                                stage_name,
                                symbol,
                                duration.as_secs_f64()
                            ),
                            kind: if *status == StageStatus::Succeeded {
                                OutputKind::Success
                            } else {
                                OutputKind::Failure
                            },
                            stage_name: Some(stage_name.clone()),
                            job_name: None,
                        });
                    }

                    ExecutionEvent::StageSkipped {
                        stage_name, reason, ..
                    } => {
                        if let Some(stage) = exec.stages.iter_mut().find(|s| s.name == *stage_name)
                        {
                            stage.status = StageStatus::Skipped;
                        }
                        exec.output_lines.push(OutputLine {
                            text: format!("  Stage '{}' skipped: {}", stage_name, reason),
                            kind: OutputKind::Warning,
                            stage_name: Some(stage_name.clone()),
                            job_name: None,
                        });
                    }

                    ExecutionEvent::JobStarted {
                        stage_name,
                        job_name,
                        display_name,
                        total_steps,
                        ..
                    } => {
                        if let Some(stage) = exec.stages.iter_mut().find(|s| s.name == *stage_name)
                        {
                            if let Some(job) = stage.jobs.iter_mut().find(|j| j.name == *job_name) {
                                job.status = JobStatus::Running;
                            }
                        }
                        let label = display_name.as_deref().unwrap_or(job_name);
                        exec.output_lines.push(OutputLine {
                            text: format!("    Job '{}' ({} steps)", label, total_steps),
                            kind: OutputKind::JobHeader,
                            stage_name: Some(stage_name.clone()),
                            job_name: Some(job_name.clone()),
                        });
                    }

                    ExecutionEvent::JobCompleted {
                        stage_name,
                        job_name,
                        status,
                        duration,
                        ..
                    } => {
                        if let Some(stage) = exec.stages.iter_mut().find(|s| s.name == *stage_name)
                        {
                            if let Some(job) = stage.jobs.iter_mut().find(|j| j.name == *job_name) {
                                job.status = status.clone();
                                job.duration = Some(*duration);
                            }
                        }
                        let symbol = match status {
                            JobStatus::Succeeded => "OK",
                            JobStatus::Failed => "FAIL",
                            _ => "DONE",
                        };
                        exec.output_lines.push(OutputLine {
                            text: format!(
                                "    Job '{}' {} ({:.2}s)",
                                job_name,
                                symbol,
                                duration.as_secs_f64()
                            ),
                            kind: if *status == JobStatus::Succeeded {
                                OutputKind::Success
                            } else {
                                OutputKind::Failure
                            },
                            stage_name: Some(stage_name.clone()),
                            job_name: Some(job_name.clone()),
                        });
                    }

                    ExecutionEvent::JobSkipped {
                        stage_name,
                        job_name,
                        reason,
                    } => {
                        if let Some(stage) = exec.stages.iter_mut().find(|s| s.name == *stage_name)
                        {
                            if let Some(job) = stage.jobs.iter_mut().find(|j| j.name == *job_name) {
                                job.status = JobStatus::Skipped;
                            }
                        }
                        exec.output_lines.push(OutputLine {
                            text: format!("    Job '{}' skipped: {}", job_name, reason),
                            kind: OutputKind::Warning,
                            stage_name: Some(stage_name.clone()),
                            job_name: Some(job_name.clone()),
                        });
                    }

                    ExecutionEvent::StepStarted {
                        stage_name,
                        job_name,
                        step_name,
                        display_name,
                        step_index,
                    } => {
                        if let Some(stage) = exec.stages.iter_mut().find(|s| s.name == *stage_name)
                        {
                            if let Some(job) = stage.jobs.iter_mut().find(|j| j.name == *job_name) {
                                if let Some(step) = job.steps.get_mut(*step_index) {
                                    step.status = StepStatus::Running;
                                }
                            }
                        }
                        let label = display_name
                            .as_deref()
                            .or(step_name.as_deref())
                            .unwrap_or("step");
                        exec.output_lines.push(OutputLine {
                            text: format!("      [Step {}] {}", step_index + 1, label),
                            kind: OutputKind::StepHeader,
                            stage_name: Some(stage_name.clone()),
                            job_name: Some(job_name.clone()),
                        });
                    }

                    ExecutionEvent::StepOutput {
                        stage_name,
                        job_name,
                        output,
                        is_error,
                        ..
                    } => {
                        for line in output.lines() {
                            exec.output_lines.push(OutputLine {
                                text: format!("        {}", line),
                                kind: if *is_error {
                                    OutputKind::Error
                                } else {
                                    OutputKind::Output
                                },
                                stage_name: Some(stage_name.clone()),
                                job_name: Some(job_name.clone()),
                            });
                        }
                    }

                    ExecutionEvent::StepCompleted {
                        stage_name,
                        job_name,
                        step_index,
                        status,
                        duration,
                        exit_code,
                        ..
                    } => {
                        if let Some(stage) = exec.stages.iter_mut().find(|s| s.name == *stage_name)
                        {
                            if let Some(job) = stage.jobs.iter_mut().find(|j| j.name == *job_name) {
                                if let Some(step) = job.steps.get_mut(*step_index) {
                                    step.status = status.clone();
                                    step.duration = Some(*duration);
                                    step.exit_code = *exit_code;
                                }
                            }
                        }
                        let symbol = match status {
                            StepStatus::Succeeded => "OK",
                            StepStatus::Failed => "FAIL",
                            StepStatus::Skipped => "SKIP",
                            _ => "DONE",
                        };
                        exec.output_lines.push(OutputLine {
                            text: format!("        {} ({:.2}s)", symbol, duration.as_secs_f64()),
                            kind: if *status == StepStatus::Succeeded {
                                OutputKind::Success
                            } else if *status == StepStatus::Failed {
                                OutputKind::Failure
                            } else {
                                OutputKind::Warning
                            },
                            stage_name: Some(stage_name.clone()),
                            job_name: Some(job_name.clone()),
                        });
                    }

                    ExecutionEvent::StepSkipped {
                        stage_name,
                        job_name,
                        step_name,
                        step_index,
                        reason,
                    } => {
                        if let Some(stage) = exec.stages.iter_mut().find(|s| s.name == *stage_name)
                        {
                            if let Some(job) = stage.jobs.iter_mut().find(|j| j.name == *job_name) {
                                if let Some(step) = job.steps.get_mut(*step_index) {
                                    step.status = StepStatus::Skipped;
                                }
                            }
                        }
                        let label = step_name.as_deref().unwrap_or("step");
                        exec.output_lines.push(OutputLine {
                            text: format!("        {} skipped: {}", label, reason),
                            kind: OutputKind::Warning,
                            stage_name: Some(stage_name.clone()),
                            job_name: Some(job_name.clone()),
                        });
                    }

                    ExecutionEvent::VariableSet {
                        stage_name,
                        job_name,
                        name,
                        value,
                        is_secret,
                        ..
                    } => {
                        let display_value = if *is_secret { "***" } else { value.as_str() };
                        exec.output_lines.push(OutputLine {
                            text: format!("        [var] {} = {}", name, display_value),
                            kind: OutputKind::Info,
                            stage_name: Some(stage_name.clone()),
                            job_name: Some(job_name.clone()),
                        });
                    }

                    ExecutionEvent::Log {
                        level,
                        message,
                        stage_name,
                        job_name,
                    } => {
                        use pipeline_service::execution::events::LogLevel;
                        let kind = match level {
                            LogLevel::Error => OutputKind::Error,
                            LogLevel::Warning => OutputKind::Warning,
                            _ => OutputKind::Info,
                        };
                        exec.output_lines.push(OutputLine {
                            text: format!(
                                "  [{}] {}",
                                format!("{:?}", level).to_uppercase(),
                                message
                            ),
                            kind,
                            stage_name: stage_name.clone(),
                            job_name: job_name.clone(),
                        });
                    }

                    ExecutionEvent::Error {
                        message,
                        stage_name,
                        job_name,
                        ..
                    } => {
                        exec.output_lines.push(OutputLine {
                            text: format!("  ERROR: {}", message),
                            kind: OutputKind::Error,
                            stage_name: stage_name.clone(),
                            job_name: job_name.clone(),
                        });
                    }
                }
            }
        }

        if should_close {
            self.event_receiver = None;
        }
    }

    // =========================================================================
    // Log Viewer
    // =========================================================================

    pub fn open_log_viewer(&mut self) {
        if self.execution_state.is_some() {
            self.log_viewer.reset();
            self.push_state(AppState::ExecutionLog);
        }
    }

    pub fn start_search(&mut self) {
        if self.state == AppState::ExecutionLog {
            self.log_viewer.search_active = true;
            self.log_viewer.search_query.clear();
            self.log_viewer.search_matches.clear();
            self.log_viewer.current_match = 0;
        }
    }

    pub fn search_push_char(&mut self, c: char) {
        if self.log_viewer.search_active {
            self.log_viewer.search_query.push(c);
            self.update_search_matches();
        }
    }

    pub fn search_pop_char(&mut self) {
        if self.log_viewer.search_active {
            self.log_viewer.search_query.pop();
            self.update_search_matches();
        }
    }

    pub fn confirm_search(&mut self) {
        self.log_viewer.search_active = false;
        if !self.log_viewer.search_matches.is_empty() {
            self.log_viewer.scroll_offset =
                self.log_viewer.search_matches[self.log_viewer.current_match];
        }
    }

    pub fn next_search_match(&mut self) {
        if !self.log_viewer.search_matches.is_empty() {
            self.log_viewer.current_match =
                (self.log_viewer.current_match + 1) % self.log_viewer.search_matches.len();
            self.log_viewer.scroll_offset =
                self.log_viewer.search_matches[self.log_viewer.current_match];
        }
    }

    fn update_search_matches(&mut self) {
        self.log_viewer.search_matches.clear();
        if let Some(exec) = &self.execution_state {
            let query = self.log_viewer.search_query.to_lowercase();
            if !query.is_empty() {
                for (i, line) in exec.output_lines.iter().enumerate() {
                    if line.text.to_lowercase().contains(&query) {
                        self.log_viewer.search_matches.push(i);
                    }
                }
            }
        }
        self.log_viewer.current_match = 0;
    }

    // =========================================================================
    // Test Runner
    // =========================================================================

    pub fn request_test_run(&mut self) {
        self.pending_test_run = true;
    }

    async fn run_tests(&mut self) {
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let test_files = TestFileParser::discover(&current_dir);

        if test_files.is_empty() {
            self.test_state = Some(TestState {
                results: Vec::new(),
                selected_suite: 0,
                selected_test: 0,
                is_running: false,
                total_passed: 0,
                total_failed: 0,
                total_skipped: 0,
            });
            self.push_state(AppState::TestResults);
            return;
        }

        self.test_state = Some(TestState {
            results: Vec::new(),
            selected_suite: 0,
            selected_test: 0,
            is_running: true,
            total_passed: 0,
            total_failed: 0,
            total_skipped: 0,
        });
        self.push_state(AppState::TestResults);

        let runner = TestRunner::new();
        let mut results = Vec::new();

        for file in &test_files {
            match runner.run_file(file).await {
                Ok(suite_result) => results.push(suite_result),
                Err(_e) => {
                    // Skip files that fail to parse/run
                }
            }
        }

        let total_passed: usize = results.iter().map(|r| r.passed).sum();
        let total_failed: usize = results.iter().map(|r| r.failed).sum();
        let total_skipped: usize = results.iter().map(|r| r.skipped).sum();

        if let Some(test_state) = &mut self.test_state {
            test_state.results = results;
            test_state.is_running = false;
            test_state.total_passed = total_passed;
            test_state.total_failed = total_failed;
            test_state.total_skipped = total_skipped;
        }
    }

    // =========================================================================
    // Helpers
    // =========================================================================

    /// Get the currently selected pipeline, if any
    pub fn selected_pipeline(&self) -> Option<&PipelineInfo> {
        self.pipelines.get(self.selected_index)
    }

    /// Get filtered output lines for the log viewer
    pub fn filtered_output_lines(&self) -> Vec<&OutputLine> {
        if let Some(exec) = &self.execution_state {
            exec.output_lines
                .iter()
                .filter(|line| {
                    if let Some(filter_stage) = &self.log_viewer.filter_stage {
                        if let Some(stage) = &line.stage_name {
                            if stage != filter_stage {
                                return false;
                            }
                        }
                    }
                    if let Some(filter_job) = &self.log_viewer.filter_job {
                        if let Some(job) = &line.job_name {
                            if job != filter_job {
                                return false;
                            }
                        }
                    }
                    true
                })
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Calculate overall execution progress as a ratio (0.0 to 1.0)
    pub fn execution_progress(&self) -> f64 {
        if let Some(exec) = &self.execution_state {
            if exec.stages.is_empty() {
                return 0.0;
            }
            let total = exec.stages.len() as f64;
            let done = exec
                .stages
                .iter()
                .filter(|s| {
                    matches!(
                        s.status,
                        StageStatus::Succeeded
                            | StageStatus::Failed
                            | StageStatus::Skipped
                            | StageStatus::Canceled
                    )
                })
                .count() as f64;
            done / total
        } else {
            0.0
        }
    }
}
