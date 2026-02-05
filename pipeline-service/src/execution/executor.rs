// Pipeline Executor
// Orchestrates pipeline execution with DAG-based scheduling

use crate::execution::context::RuntimeContext;
use crate::execution::events::{EventSender, ExecutionEvent, ProgressSender};
use crate::execution::graph::{ExecutionGraph, GraphError, JobNode, StageNode};
use crate::execution::matrix::MatrixExpander;
use crate::parser::models::{
    ExecutionContext, Job, JobResult, JobStatus, Pipeline, StageResult, StageStatus, Step,
    StepResult, StepStatus, StepAction,
};
use crate::runners::shell::ShellRunner;
use crate::runners::task::TaskRunner;
use crate::runners::container::ContainerRunner;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;

/// Result of pipeline execution
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    /// All stage results
    pub stages: Vec<StageResult>,
    /// Total duration
    pub duration: Duration,
    /// Overall success
    pub success: bool,
    /// Final variables state
    pub variables: HashMap<String, String>,
}

/// Configuration for pipeline execution
#[derive(Debug, Clone)]
pub struct ExecutorConfig {
    /// Maximum parallel stages (0 = unlimited)
    pub max_parallel_stages: usize,
    /// Maximum parallel jobs within a stage (0 = unlimited)
    pub max_parallel_jobs: usize,
    /// Default timeout for steps (in minutes)
    pub default_step_timeout: u32,
    /// Whether to continue on error at pipeline level
    pub continue_on_error: bool,
    /// Task cache directory
    pub task_cache_dir: Option<PathBuf>,
    /// Whether to enable container support
    pub enable_containers: bool,
}

impl Default for ExecutorConfig {
    fn default() -> Self {
        Self {
            max_parallel_stages: 0,
            max_parallel_jobs: 0,
            default_step_timeout: 60,
            continue_on_error: false,
            task_cache_dir: None,
            enable_containers: false,
        }
    }
}

/// Pipeline executor
pub struct PipelineExecutor {
    /// Execution graph
    graph: ExecutionGraph,
    /// Configuration
    config: ExecutorConfig,
    /// Progress event sender
    event_tx: Option<ProgressSender>,
    /// Shell runner for script steps
    shell_runner: ShellRunner,
    /// Task runner for Azure DevOps tasks
    task_runner: Option<TaskRunner>,
    /// Container runner for Docker-based jobs
    container_runner: Option<ContainerRunner>,
}

impl PipelineExecutor {
    /// Create a new executor from a pipeline
    pub fn from_pipeline(pipeline: &Pipeline) -> Result<Self, GraphError> {
        let graph = ExecutionGraph::from_pipeline(pipeline)?;
        Ok(Self {
            graph,
            config: ExecutorConfig::default(),
            event_tx: None,
            shell_runner: ShellRunner::new(),
            task_runner: None,
            container_runner: None,
        })
    }

    /// Create a new executor from an execution graph
    pub fn new(graph: ExecutionGraph) -> Self {
        Self {
            graph,
            config: ExecutorConfig::default(),
            event_tx: None,
            shell_runner: ShellRunner::new(),
            task_runner: None,
            container_runner: None,
        }
    }

    /// Set executor configuration
    pub fn with_config(mut self, config: ExecutorConfig) -> Self {
        // Set up task runner if cache dir is specified
        if let Some(cache_dir) = &config.task_cache_dir {
            self.task_runner = Some(TaskRunner::new(cache_dir.clone()));
        }
        
        // Set up container runner if enabled
        if config.enable_containers {
            self.container_runner = Some(ContainerRunner::new());
        }
        
        self.config = config;
        self
    }

    /// Set progress event sender
    pub fn with_progress(mut self, tx: ProgressSender) -> Self {
        self.event_tx = Some(tx);
        self
    }
    
    /// Enable task execution with the specified cache directory
    pub fn with_task_runner(mut self, cache_dir: PathBuf) -> Self {
        self.task_runner = Some(TaskRunner::new(cache_dir));
        self
    }
    
    /// Enable container execution
    pub fn with_container_runner(mut self) -> Self {
        self.container_runner = Some(ContainerRunner::new());
        self
    }

    /// Execute the pipeline
    pub async fn execute(&self, context: ExecutionContext) -> ExecutionResult {
        let start = Instant::now();
        let mut runtime = RuntimeContext::new(context);
        let mut stage_results = Vec::new();
        let mut overall_success = true;

        // Send pipeline started event
        self.event_tx.send_event(ExecutionEvent::pipeline_started(
            &runtime.base.pipeline_name,
            self.graph.stages.len(),
        ));

        // Execute stages in topological order, respecting parallelism
        let parallel_stages = self.graph.parallel_stages();

        for stage_level in parallel_stages {
            // Execute stages at this level (potentially in parallel)
            let level_results = self
                .execute_stage_level(&stage_level, &mut runtime)
                .await;

            for result in level_results {
                let failed = result.status == StageStatus::Failed;
                stage_results.push(result);

                if failed {
                    overall_success = false;
                    if !self.config.continue_on_error {
                        break;
                    }
                }
            }

            if !overall_success && !self.config.continue_on_error {
                break;
            }
        }

        let duration = start.elapsed();

        // Send pipeline completed event
        self.event_tx.send_event(ExecutionEvent::pipeline_completed(
            &runtime.base.pipeline_name,
            overall_success,
            duration,
        ));

        ExecutionResult {
            stages: stage_results,
            duration,
            success: overall_success,
            variables: runtime
                .variables
                .iter()
                .map(|(k, v)| (k.clone(), v.as_string()))
                .collect(),
        }
    }

    /// Execute a level of stages (can run in parallel)
    async fn execute_stage_level(
        &self,
        stages: &[&StageNode],
        runtime: &mut RuntimeContext,
    ) -> Vec<StageResult> {
        // For now, execute sequentially
        // TODO: Add parallel execution with semaphore limiting
        let mut results = Vec::new();

        for stage in stages {
            let result = self.execute_stage(stage, runtime).await;
            results.push(result);
        }

        results
    }

    /// Execute a single stage
    async fn execute_stage(
        &self,
        stage_node: &StageNode,
        runtime: &mut RuntimeContext,
    ) -> StageResult {
        let start = Instant::now();
        let stage = &stage_node.stage;
        let stage_name = &stage.stage;

        // Check dependencies
        if !stage_node.dependencies.is_empty()
            && !runtime.dependencies_succeeded(&stage_node.dependencies, true)
        {
            self.event_tx.send_event(ExecutionEvent::StageSkipped {
                stage_name: stage_name.clone(),
                reason: "Dependencies failed".to_string(),
            });

            return StageResult {
                stage_name: stage_name.clone(),
                display_name: stage.display_name.clone(),
                status: StageStatus::Skipped,
                jobs: Vec::new(),
                duration: start.elapsed(),
            };
        }

        // Evaluate condition
        if let Some(condition) = &stage.condition {
            match runtime.evaluate_condition(condition) {
                Ok(true) => {} // Continue
                Ok(false) => {
                    self.event_tx.send_event(ExecutionEvent::StageSkipped {
                        stage_name: stage_name.clone(),
                        reason: format!("Condition '{}' evaluated to false", condition),
                    });

                    return StageResult {
                        stage_name: stage_name.clone(),
                        display_name: stage.display_name.clone(),
                        status: StageStatus::Skipped,
                        jobs: Vec::new(),
                        duration: start.elapsed(),
                    };
                }
                Err(e) => {
                    self.event_tx.send_event(ExecutionEvent::error(
                        format!("Condition evaluation failed: {}", e),
                        Some(stage_name.clone()),
                        None,
                    ));

                    return StageResult {
                        stage_name: stage_name.clone(),
                        display_name: stage.display_name.clone(),
                        status: StageStatus::Failed,
                        jobs: Vec::new(),
                        duration: start.elapsed(),
                    };
                }
            }
        }

        // Enter stage
        runtime.enter_stage(stage);

        self.event_tx.send_event(ExecutionEvent::stage_started(
            stage_name,
            stage.display_name.clone(),
            stage_node.jobs.len(),
        ));

        // Execute jobs
        let mut job_results = Vec::new();
        let mut stage_status = StageStatus::Succeeded;

        // Get jobs in topological order with parallel levels
        let parallel_jobs = self.graph.parallel_jobs(stage_node);

        for job_level in parallel_jobs {
            // Execute jobs at this level
            let level_results = self
                .execute_job_level(&job_level, stage_name, runtime)
                .await;

            for result in level_results {
                if result.status == JobStatus::Failed {
                    stage_status = StageStatus::Failed;
                } else if result.status == JobStatus::SucceededWithIssues
                    && stage_status == StageStatus::Succeeded
                {
                    stage_status = StageStatus::SucceededWithIssues;
                }
                job_results.push(result);
            }

            if stage_status == StageStatus::Failed && !self.config.continue_on_error {
                break;
            }
        }

        let duration = start.elapsed();

        // Exit stage
        let result = StageResult {
            stage_name: stage_name.clone(),
            display_name: stage.display_name.clone(),
            status: stage_status.clone(),
            jobs: job_results,
            duration,
        };

        runtime.exit_stage(result.clone());

        self.event_tx.send_event(ExecutionEvent::stage_completed(
            stage_name,
            stage_status,
            duration,
        ));

        result
    }

    /// Execute a level of jobs (can run in parallel)
    async fn execute_job_level(
        &self,
        jobs: &[&JobNode],
        stage_name: &str,
        runtime: &mut RuntimeContext,
    ) -> Vec<JobResult> {
        // For now, execute sequentially
        // TODO: Add parallel execution with semaphore and max_parallel
        let mut results = Vec::new();

        for job_node in jobs {
            let result = self.execute_job(job_node, stage_name, runtime).await;
            results.push(result);
        }

        results
    }

    /// Execute a single job (potentially with matrix expansion)
    async fn execute_job(
        &self,
        job_node: &JobNode,
        stage_name: &str,
        runtime: &mut RuntimeContext,
    ) -> JobResult {
        let job = &job_node.job;
        let job_name = job.identifier().unwrap_or("unknown").to_string();
        let start = Instant::now();

        // Check dependencies
        if !job_node.dependencies.is_empty()
            && !runtime.dependencies_succeeded(&job_node.dependencies, false)
        {
            self.event_tx.send_event(ExecutionEvent::JobSkipped {
                stage_name: stage_name.to_string(),
                job_name: job_name.clone(),
                reason: "Dependencies failed".to_string(),
            });

            return JobResult {
                job_name,
                display_name: job.display_name.clone(),
                status: JobStatus::Skipped,
                steps: Vec::new(),
                duration: start.elapsed(),
                outputs: HashMap::new(),
            };
        }

        // Evaluate condition
        if let Some(condition) = &job.condition {
            match runtime.evaluate_condition(condition) {
                Ok(true) => {}
                Ok(false) => {
                    self.event_tx.send_event(ExecutionEvent::JobSkipped {
                        stage_name: stage_name.to_string(),
                        job_name: job_name.clone(),
                        reason: format!("Condition '{}' evaluated to false", condition),
                    });

                    return JobResult {
                        job_name,
                        display_name: job.display_name.clone(),
                        status: JobStatus::Skipped,
                        steps: Vec::new(),
                        duration: start.elapsed(),
                        outputs: HashMap::new(),
                    };
                }
                Err(e) => {
                    self.event_tx.send_event(ExecutionEvent::error(
                        format!("Condition evaluation failed: {}", e),
                        Some(stage_name.to_string()),
                        Some(job_name.clone()),
                    ));

                    return JobResult {
                        job_name,
                        display_name: job.display_name.clone(),
                        status: JobStatus::Failed,
                        steps: Vec::new(),
                        duration: start.elapsed(),
                        outputs: HashMap::new(),
                    };
                }
            }
        }

        // Handle matrix expansion
        if let Some(strategy) = &job.strategy {
            let instances = MatrixExpander::expand(strategy);
            if !instances.is_empty() {
                // Execute matrix instances
                return self
                    .execute_matrix_job(job_node, stage_name, &instances, runtime)
                    .await;
            }
        }

        // Execute single job instance
        self.execute_job_instance(job, stage_name, &job_name, None, runtime)
            .await
    }

    /// Execute a job with matrix expansion
    async fn execute_matrix_job(
        &self,
        job_node: &JobNode,
        stage_name: &str,
        instances: &[super::matrix::MatrixInstance],
        runtime: &mut RuntimeContext,
    ) -> JobResult {
        let job = &job_node.job;
        let job_name = job.identifier().unwrap_or("unknown").to_string();
        let start = Instant::now();

        let max_parallel = job
            .strategy
            .as_ref()
            .and_then(|s| s.max_parallel)
            .unwrap_or(instances.len() as u32);

        let _semaphore = Arc::new(Semaphore::new(max_parallel as usize));
        let mut all_steps = Vec::new();
        let mut overall_status = JobStatus::Succeeded;

        // Execute each matrix instance
        for instance in instances {
            // Apply matrix variables to runtime
            for (var_name, var_value) in &instance.variables {
                runtime.set_variable(var_name.clone(), var_value.clone());
            }

            let instance_result = self
                .execute_job_instance(
                    job,
                    stage_name,
                    &job_name,
                    Some(&instance.name),
                    runtime,
                )
                .await;

            all_steps.extend(instance_result.steps);

            if instance_result.status == JobStatus::Failed {
                overall_status = JobStatus::Failed;
                if !job.continue_on_error {
                    break;
                }
            } else if instance_result.status == JobStatus::SucceededWithIssues
                && overall_status == JobStatus::Succeeded
            {
                overall_status = JobStatus::SucceededWithIssues;
            }
        }

        JobResult {
            job_name,
            display_name: job.display_name.clone(),
            status: overall_status,
            steps: all_steps,
            duration: start.elapsed(),
            outputs: runtime
                .step_outputs
                .values()
                .flat_map(|m| m.iter())
                .map(|(k, v)| (k.clone(), v.as_string()))
                .collect(),
        }
    }

    /// Execute a single job instance
    async fn execute_job_instance(
        &self,
        job: &Job,
        stage_name: &str,
        job_name: &str,
        matrix_instance: Option<&str>,
        runtime: &mut RuntimeContext,
    ) -> JobResult {
        let start = Instant::now();

        runtime.enter_job(job);

        self.event_tx.send_event(ExecutionEvent::job_started(
            stage_name,
            job_name,
            job.display_name.clone(),
            matrix_instance.map(String::from),
            job.steps.len(),
        ));

        let mut step_results = Vec::new();
        let mut job_status = JobStatus::Succeeded;
        let mut should_run = true;

        for (step_index, step) in job.steps.iter().enumerate() {
            if !should_run && !should_always_run(step) {
                // Skip remaining steps if a previous step failed
                let skipped = StepResult {
                    step_name: step.name.clone(),
                    display_name: step.display_name.clone(),
                    status: StepStatus::Skipped,
                    output: String::new(),
                    error: None,
                    duration: Duration::ZERO,
                    exit_code: None,
                    outputs: HashMap::new(),
                };
                step_results.push(skipped);
                continue;
            }

            let result = self
                .execute_step(step, step_index, stage_name, job_name, runtime)
                .await;

            runtime.record_step_result(result.clone());

            match result.status {
                StepStatus::Failed => {
                    if !step.continue_on_error {
                        should_run = false;
                        job_status = JobStatus::Failed;
                    } else {
                        job_status = JobStatus::SucceededWithIssues;
                    }
                }
                StepStatus::SucceededWithIssues => {
                    if job_status == JobStatus::Succeeded {
                        job_status = JobStatus::SucceededWithIssues;
                    }
                }
                _ => {}
            }

            step_results.push(result);
        }

        let duration = start.elapsed();

        let result = JobResult {
            job_name: job_name.to_string(),
            display_name: job.display_name.clone(),
            status: job_status.clone(),
            steps: step_results,
            duration,
            outputs: runtime
                .step_outputs
                .values()
                .flat_map(|m| m.iter())
                .map(|(k, v)| (k.clone(), v.as_string()))
                .collect(),
        };

        runtime.exit_job(result.clone());

        self.event_tx.send_event(ExecutionEvent::job_completed(
            stage_name,
            job_name,
            matrix_instance.map(String::from),
            job_status,
            duration,
        ));

        result
    }

    /// Execute a single step
    async fn execute_step(
        &self,
        step: &Step,
        step_index: usize,
        stage_name: &str,
        job_name: &str,
        runtime: &mut RuntimeContext,
    ) -> StepResult {
        let start = Instant::now();
        let step_name = step.name.clone();

        // Check if step is enabled
        if !step.enabled {
            self.event_tx.send_event(ExecutionEvent::StepSkipped {
                stage_name: stage_name.to_string(),
                job_name: job_name.to_string(),
                step_name: step_name.clone(),
                step_index,
                reason: "Step is disabled".to_string(),
            });

            return StepResult {
                step_name,
                display_name: step.display_name.clone(),
                status: StepStatus::Skipped,
                output: String::new(),
                error: None,
                duration: start.elapsed(),
                exit_code: None,
                outputs: HashMap::new(),
            };
        }

        // Evaluate condition
        if let Some(condition) = &step.condition {
            match runtime.evaluate_condition(condition) {
                Ok(true) => {}
                Ok(false) => {
                    self.event_tx.send_event(ExecutionEvent::StepSkipped {
                        stage_name: stage_name.to_string(),
                        job_name: job_name.to_string(),
                        step_name: step_name.clone(),
                        step_index,
                        reason: format!("Condition '{}' evaluated to false", condition),
                    });

                    return StepResult {
                        step_name,
                        display_name: step.display_name.clone(),
                        status: StepStatus::Skipped,
                        output: String::new(),
                        error: None,
                        duration: start.elapsed(),
                        exit_code: None,
                        outputs: HashMap::new(),
                    };
                }
                Err(e) => {
                    return StepResult {
                        step_name,
                        display_name: step.display_name.clone(),
                        status: StepStatus::Failed,
                        output: String::new(),
                        error: Some(format!("Condition evaluation failed: {}", e)),
                        duration: start.elapsed(),
                        exit_code: None,
                        outputs: HashMap::new(),
                    };
                }
            }
        }

        // Send step started event
        self.event_tx.send_event(ExecutionEvent::step_started(
            stage_name,
            job_name,
            step_name.clone(),
            step.display_name.clone(),
            step_index,
        ));

        // Execute the step based on its action type
        let result = self
            .execute_step_action(&step.action, step, step_index, stage_name, job_name, runtime)
            .await;

        // Send step completed event
        self.event_tx.send_event(ExecutionEvent::step_completed(
            stage_name,
            job_name,
            step_name.clone(),
            step_index,
            result.status.clone(),
            result.duration,
            result.exit_code,
        ));

        result
    }

    /// Execute a step action
    async fn execute_step_action(
        &self,
        action: &StepAction,
        step: &Step,
        step_index: usize,
        stage_name: &str,
        job_name: &str,
        runtime: &mut RuntimeContext,
    ) -> StepResult {
        let start = Instant::now();
        let step_name = step.name.clone();

        match action {
            StepAction::Script(script_step) => {
                self.execute_script(
                    &script_step.script,
                    script_step.working_directory.as_deref(),
                    script_step.fail_on_stderr,
                    step,
                    step_index,
                    stage_name,
                    job_name,
                    runtime,
                )
                .await
            }
            StepAction::Bash(bash_step) => {
                self.execute_bash(
                    &bash_step.bash,
                    bash_step.working_directory.as_deref(),
                    bash_step.fail_on_stderr,
                    step,
                    step_index,
                    stage_name,
                    job_name,
                    runtime,
                )
                .await
            }
            StepAction::Pwsh(pwsh_step) => {
                self.execute_pwsh(
                    &pwsh_step.pwsh,
                    pwsh_step.working_directory.as_deref(),
                    pwsh_step.fail_on_stderr,
                    step,
                    step_index,
                    stage_name,
                    job_name,
                    runtime,
                )
                .await
            }
            StepAction::PowerShell(ps_step) => {
                self.execute_powershell(
                    &ps_step.powershell,
                    ps_step.working_directory.as_deref(),
                    ps_step.fail_on_stderr,
                    step,
                    step_index,
                    stage_name,
                    job_name,
                    runtime,
                )
                .await
            }
            StepAction::Task(task_step) => {
                // Execute task using TaskRunner
                if let Some(task_runner) = &self.task_runner {
                    let working_dir = std::path::PathBuf::from(&runtime.base.working_dir);
                    let env = runtime.env_as_strings();
                    
                    match task_runner.execute_task(
                        &task_step.task,
                        &task_step.inputs,
                        &env,
                        &working_dir,
                    ).await {
                        Ok(mut result) => {
                            result.step_name = step_name;
                            result.display_name = step.display_name.clone();
                            
                            // Send output event
                            if !result.output.is_empty() {
                                self.event_tx.send_event(ExecutionEvent::step_output(
                                    stage_name,
                                    job_name,
                                    result.step_name.clone(),
                                    step_index,
                                    &result.output,
                                    false,
                                ));
                            }
                            
                            result
                        }
                        Err(e) => StepResult {
                            step_name,
                            display_name: step.display_name.clone(),
                            status: StepStatus::Failed,
                            output: String::new(),
                            error: Some(format!("Task execution failed: {}", e)),
                            duration: start.elapsed(),
                            exit_code: None,
                            outputs: HashMap::new(),
                        }
                    }
                } else {
                    // Task runner not configured - log a warning
                    self.event_tx.send_event(ExecutionEvent::step_output(
                        stage_name,
                        job_name,
                        step_name.clone(),
                        step_index,
                        format!("Task runner not configured. Task: {}", task_step.task),
                        true,
                    ));

                    StepResult {
                        step_name,
                        display_name: step.display_name.clone(),
                        status: StepStatus::Skipped,
                        output: format!("Task: {} (skipped - task runner not configured)", task_step.task),
                        error: None,
                        duration: start.elapsed(),
                        exit_code: None,
                        outputs: HashMap::new(),
                    }
                }
            }
            StepAction::Checkout(_) => {
                // Checkout - for now, assume already checked out
                StepResult {
                    step_name,
                    display_name: step.display_name.clone(),
                    status: StepStatus::Succeeded,
                    output: "Checkout: Using existing working directory".to_string(),
                    error: None,
                    duration: start.elapsed(),
                    exit_code: Some(0),
                    outputs: HashMap::new(),
                }
            }
            StepAction::Template(_) => {
                // Templates should be expanded earlier
                StepResult {
                    step_name,
                    display_name: step.display_name.clone(),
                    status: StepStatus::Skipped,
                    output: "Template step (should be expanded)".to_string(),
                    error: None,
                    duration: start.elapsed(),
                    exit_code: None,
                    outputs: HashMap::new(),
                }
            }
            StepAction::Download(_) | StepAction::Publish(_) => {
                // Download/Publish - placeholder for now
                StepResult {
                    step_name,
                    display_name: step.display_name.clone(),
                    status: StepStatus::Succeeded,
                    output: "Artifact operation (placeholder)".to_string(),
                    error: None,
                    duration: start.elapsed(),
                    exit_code: Some(0),
                    outputs: HashMap::new(),
                }
            }
            StepAction::GetPackage(_) | StepAction::ReviewApp(_) => {
                // Other steps - placeholder
                StepResult {
                    step_name,
                    display_name: step.display_name.clone(),
                    status: StepStatus::Skipped,
                    output: "Step type not implemented".to_string(),
                    error: None,
                    duration: start.elapsed(),
                    exit_code: None,
                    outputs: HashMap::new(),
                }
            }
        }
    }

    /// Execute a script step
    async fn execute_script(
        &self,
        script: &str,
        working_directory: Option<&str>,
        fail_on_stderr: bool,
        step: &Step,
        step_index: usize,
        stage_name: &str,
        job_name: &str,
        runtime: &mut RuntimeContext,
    ) -> StepResult {
        // Substitute variables in script
        let script = match runtime.substitute_variables(script) {
            Ok(s) => s,
            Err(e) => {
                return StepResult {
                    step_name: step.name.clone(),
                    display_name: step.display_name.clone(),
                    status: StepStatus::Failed,
                    output: String::new(),
                    error: Some(format!("Variable substitution failed: {}", e)),
                    duration: Duration::ZERO,
                    exit_code: None,
                    outputs: HashMap::new(),
                };
            }
        };

        self.run_shell_command(
            &script,
            "sh",
            &["-c"],
            working_directory,
            fail_on_stderr,
            step,
            step_index,
            stage_name,
            job_name,
            runtime,
        )
        .await
    }

    /// Execute a bash step
    async fn execute_bash(
        &self,
        script: &str,
        working_directory: Option<&str>,
        fail_on_stderr: bool,
        step: &Step,
        step_index: usize,
        stage_name: &str,
        job_name: &str,
        runtime: &mut RuntimeContext,
    ) -> StepResult {
        let script = match runtime.substitute_variables(script) {
            Ok(s) => s,
            Err(e) => {
                return StepResult {
                    step_name: step.name.clone(),
                    display_name: step.display_name.clone(),
                    status: StepStatus::Failed,
                    output: String::new(),
                    error: Some(format!("Variable substitution failed: {}", e)),
                    duration: Duration::ZERO,
                    exit_code: None,
                    outputs: HashMap::new(),
                };
            }
        };

        self.run_shell_command(
            &script,
            "bash",
            &["-c"],
            working_directory,
            fail_on_stderr,
            step,
            step_index,
            stage_name,
            job_name,
            runtime,
        )
        .await
    }

    /// Execute a pwsh (PowerShell Core) step
    async fn execute_pwsh(
        &self,
        script: &str,
        working_directory: Option<&str>,
        fail_on_stderr: bool,
        step: &Step,
        step_index: usize,
        stage_name: &str,
        job_name: &str,
        runtime: &mut RuntimeContext,
    ) -> StepResult {
        let script = match runtime.substitute_variables(script) {
            Ok(s) => s,
            Err(e) => {
                return StepResult {
                    step_name: step.name.clone(),
                    display_name: step.display_name.clone(),
                    status: StepStatus::Failed,
                    output: String::new(),
                    error: Some(format!("Variable substitution failed: {}", e)),
                    duration: Duration::ZERO,
                    exit_code: None,
                    outputs: HashMap::new(),
                };
            }
        };

        self.run_shell_command(
            &script,
            "pwsh",
            &["-Command"],
            working_directory,
            fail_on_stderr,
            step,
            step_index,
            stage_name,
            job_name,
            runtime,
        )
        .await
    }

    /// Execute a PowerShell (Windows) step
    async fn execute_powershell(
        &self,
        script: &str,
        working_directory: Option<&str>,
        fail_on_stderr: bool,
        step: &Step,
        step_index: usize,
        stage_name: &str,
        job_name: &str,
        runtime: &mut RuntimeContext,
    ) -> StepResult {
        let script = match runtime.substitute_variables(script) {
            Ok(s) => s,
            Err(e) => {
                return StepResult {
                    step_name: step.name.clone(),
                    display_name: step.display_name.clone(),
                    status: StepStatus::Failed,
                    output: String::new(),
                    error: Some(format!("Variable substitution failed: {}", e)),
                    duration: Duration::ZERO,
                    exit_code: None,
                    outputs: HashMap::new(),
                };
            }
        };

        // On Windows, use powershell.exe; on other platforms, fall back to pwsh
        let (shell, args): (&str, &[&str]) = if cfg!(target_os = "windows") {
            ("powershell.exe", &["-Command"])
        } else {
            ("pwsh", &["-Command"])
        };

        self.run_shell_command(
            &script,
            shell,
            args,
            working_directory,
            fail_on_stderr,
            step,
            step_index,
            stage_name,
            job_name,
            runtime,
        )
        .await
    }

    /// Run a shell command
    async fn run_shell_command(
        &self,
        script: &str,
        shell: &str,
        shell_args: &[&str],
        working_directory: Option<&str>,
        fail_on_stderr: bool,
        step: &Step,
        step_index: usize,
        stage_name: &str,
        job_name: &str,
        runtime: &mut RuntimeContext,
    ) -> StepResult {
        use tokio::process::Command;
        let start = Instant::now();

        let working_dir = working_directory
            .map(|d| d.to_string())
            .unwrap_or_else(|| runtime.base.working_dir.clone());

        // Build environment
        let mut env = runtime.env_as_strings();
        for (k, v) in &step.env {
            // Substitute variables in env values
            let value = runtime.substitute_variables(v).unwrap_or_else(|_| v.clone());
            env.insert(k.clone(), value);
        }

        let mut cmd = Command::new(shell);
        cmd.args(shell_args);
        cmd.arg(script);
        cmd.current_dir(&working_dir);
        cmd.envs(&env);

        // Capture output
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        let output = match cmd.output().await {
            Ok(output) => output,
            Err(e) => {
                return StepResult {
                    step_name: step.name.clone(),
                    display_name: step.display_name.clone(),
                    status: StepStatus::Failed,
                    output: String::new(),
                    error: Some(format!("Failed to execute command: {}", e)),
                    duration: start.elapsed(),
                    exit_code: None,
                    outputs: HashMap::new(),
                };
            }
        };

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        // Send output events
        if !stdout.is_empty() {
            self.event_tx.send_event(ExecutionEvent::step_output(
                stage_name,
                job_name,
                step.name.clone(),
                step_index,
                &stdout,
                false,
            ));
        }

        if !stderr.is_empty() {
            self.event_tx.send_event(ExecutionEvent::step_output(
                stage_name,
                job_name,
                step.name.clone(),
                step_index,
                &stderr,
                true,
            ));
        }

        // Parse output for Azure DevOps logging commands
        let outputs = parse_logging_commands(&stdout, runtime);

        // Determine status
        let exit_code = output.status.code();
        let status = if !output.status.success() {
            StepStatus::Failed
        } else if fail_on_stderr && !stderr.is_empty() {
            StepStatus::Failed
        } else {
            StepStatus::Succeeded
        };

        StepResult {
            step_name: step.name.clone(),
            display_name: step.display_name.clone(),
            status,
            output: stdout,
            error: if stderr.is_empty() {
                None
            } else {
                Some(stderr)
            },
            duration: start.elapsed(),
            exit_code,
            outputs,
        }
    }
}

/// Check if a step should always run (has always() condition)
fn should_always_run(step: &Step) -> bool {
    step.condition
        .as_ref()
        .map(|c| c.contains("always()"))
        .unwrap_or(false)
}

/// Parse Azure DevOps logging commands from output
fn parse_logging_commands(output: &str, runtime: &mut RuntimeContext) -> HashMap<String, String> {
    let mut outputs = HashMap::new();

    for line in output.lines() {
        // ##vso[task.setvariable variable=name]value
        if let Some(rest) = line.strip_prefix("##vso[task.setvariable") {
            if let Some((props, value)) = rest.split_once(']') {
                let mut var_name = None;
                let mut is_output = false;
                let mut is_secret = false;

                for prop in props.split(';') {
                    let prop = prop.trim();
                    if let Some(name) = prop.strip_prefix("variable=") {
                        var_name = Some(name.to_string());
                    } else if prop == "isoutput=true" || prop == "isOutput=true" {
                        is_output = true;
                    } else if prop == "issecret=true" || prop == "isSecret=true" {
                        is_secret = true;
                    }
                }

                if let Some(name) = var_name {
                    let value = value.to_string();
                    if is_output {
                        outputs.insert(name.clone(), value.clone());
                    }
                    if !is_secret {
                        runtime.set_variable(name, crate::parser::models::Value::String(value));
                    }
                }
            }
        }
    }

    outputs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::models::{DependsOn, Job, ScriptStep, Stage, Step};

    fn make_simple_pipeline() -> Pipeline {
        Pipeline {
            name: Some("test-pipeline".to_string()),
            stages: vec![Stage {
                stage: "Build".to_string(),
                display_name: None,
                depends_on: DependsOn::None,
                condition: None,
                variables: Vec::new(),
                jobs: vec![Job {
                    job: Some("BuildJob".to_string()),
                    deployment: None,
                    display_name: None,
                    depends_on: DependsOn::None,
                    condition: None,
                    strategy: None,
                    pool: None,
                    container: None,
                    services: HashMap::new(),
                    variables: Vec::new(),
                    steps: vec![Step {
                        name: Some("echo".to_string()),
                        display_name: Some("Echo Hello".to_string()),
                        condition: None,
                        continue_on_error: false,
                        enabled: true,
                        timeout_in_minutes: None,
                        retry_count_on_task_failure: None,
                        env: HashMap::new(),
                        action: StepAction::Script(ScriptStep {
                            script: "echo Hello".to_string(),
                            working_directory: None,
                            fail_on_stderr: false,
                        }),
                    }],
                    timeout_in_minutes: None,
                    cancel_timeout_in_minutes: None,
                    continue_on_error: false,
                    workspace: None,
                    uses: None,
                    template: None,
                    parameters: HashMap::new(),
                    environment: None,
                }],
                lock_behavior: None,
                template: None,
                parameters: HashMap::new(),
                pool: None,
            }],
            ..Default::default()
        }
    }

    #[test]
    fn test_executor_creation() {
        let pipeline = make_simple_pipeline();
        let executor = PipelineExecutor::from_pipeline(&pipeline).unwrap();

        assert_eq!(executor.graph.stages.len(), 1);
    }

    #[tokio::test]
    async fn test_simple_execution() {
        let pipeline = make_simple_pipeline();
        let executor = PipelineExecutor::from_pipeline(&pipeline).unwrap();

        let context = ExecutionContext::new(
            "test".to_string(),
            std::env::current_dir()
                .unwrap()
                .to_string_lossy()
                .to_string(),
        );

        let result = executor.execute(context).await;

        assert_eq!(result.stages.len(), 1);
        assert_eq!(result.stages[0].stage_name, "Build");
        assert_eq!(result.stages[0].status, StageStatus::Succeeded);
    }

    #[test]
    fn test_parse_logging_commands() {
        let base = ExecutionContext::new("test".to_string(), "/work".to_string());
        let mut runtime = RuntimeContext::new(base);

        let output = r#"
Hello
##vso[task.setvariable variable=version]1.0.0
##vso[task.setvariable variable=output;isoutput=true]result
World
"#;

        let outputs = parse_logging_commands(output, &mut runtime);

        assert_eq!(outputs.get("output"), Some(&"result".to_string()));
        assert_eq!(
            runtime.variables.get("version"),
            Some(&crate::parser::models::Value::String("1.0.0".to_string()))
        );
    }

    #[test]
    fn test_should_always_run() {
        let step_with_always = Step {
            name: None,
            display_name: None,
            condition: Some("always()".to_string()),
            continue_on_error: false,
            enabled: true,
            timeout_in_minutes: None,
            retry_count_on_task_failure: None,
            env: HashMap::new(),
            action: StepAction::Script(ScriptStep {
                script: "echo".to_string(),
                working_directory: None,
                fail_on_stderr: false,
            }),
        };

        let step_without_always = Step {
            name: None,
            display_name: None,
            condition: Some("succeeded()".to_string()),
            continue_on_error: false,
            enabled: true,
            timeout_in_minutes: None,
            retry_count_on_task_failure: None,
            env: HashMap::new(),
            action: StepAction::Script(ScriptStep {
                script: "echo".to_string(),
                working_directory: None,
                fail_on_stderr: false,
            }),
        };

        assert!(should_always_run(&step_with_always));
        assert!(!should_always_run(&step_without_always));
    }
}
