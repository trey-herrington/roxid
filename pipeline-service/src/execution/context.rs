// Runtime Execution Context
// Manages execution state and context for expression evaluation

use crate::expression::{
    DependenciesContext, ExpressionContext, ExpressionEngine, JobContext, JobDependency,
    JobStatusContext, PipelineContext, StageContext, StageDependency, StepContext,
    StepStatusContext,
};
use crate::parser::models::{
    ExecutionContext, Job, JobResult, JobStatus, Pipeline, Stage, StageResult, StageStatus,
    StepResult, StepStatus, Value, Variable,
};

use std::collections::HashMap;

/// Runtime context during pipeline execution
#[derive(Debug, Clone)]
pub struct RuntimeContext {
    /// Base execution context (pipeline name, working dir, etc.)
    pub base: ExecutionContext,

    /// Current stage being executed
    pub current_stage: Option<String>,

    /// Current job being executed
    pub current_job: Option<String>,

    /// Completed stage results
    pub stage_results: HashMap<String, StageResult>,

    /// Completed job results (indexed by "stage.job" or just "job" for implicit stage)
    pub job_results: HashMap<String, JobResult>,

    /// Current job's step results
    pub step_results: Vec<StepResult>,

    /// All variables (merged from pipeline, stage, job levels)
    pub variables: HashMap<String, Value>,

    /// All parameters
    pub parameters: HashMap<String, Value>,

    /// Environment variables
    pub env: HashMap<String, Value>,

    /// Output variables from steps (step_name -> output_name -> value)
    pub step_outputs: HashMap<String, HashMap<String, Value>>,
}

impl RuntimeContext {
    /// Create a new runtime context from base execution context
    pub fn new(base: ExecutionContext) -> Self {
        let variables: HashMap<String, Value> = base
            .variables
            .iter()
            .map(|(k, v)| (k.clone(), Value::String(v.clone())))
            .collect();

        let parameters: HashMap<String, Value> = base
            .parameters
            .iter()
            .map(|(k, v)| (k.clone(), yaml_to_value(v)))
            .collect();

        Self {
            base,
            current_stage: None,
            current_job: None,
            stage_results: HashMap::new(),
            job_results: HashMap::new(),
            step_results: Vec::new(),
            variables,
            parameters,
            env: HashMap::new(),
            step_outputs: HashMap::new(),
        }
    }

    /// Create a new runtime context from a pipeline
    pub fn from_pipeline(pipeline: &Pipeline, working_dir: String) -> Self {
        let base = ExecutionContext::new(
            pipeline
                .name
                .clone()
                .unwrap_or_else(|| "unnamed".to_string()),
            working_dir,
        );

        let mut ctx = Self::new(base);

        // Merge pipeline-level variables
        ctx.merge_variables(&pipeline.variables);

        // Merge pipeline-level parameters
        for param in &pipeline.parameters {
            if let Some(default) = &param.default {
                ctx.parameters
                    .entry(param.name.clone())
                    .or_insert_with(|| yaml_to_value(default));
            }
        }

        ctx
    }

    /// Enter a stage (set current stage and merge variables)
    pub fn enter_stage(&mut self, stage: &Stage) {
        self.current_stage = stage.stage.clone();
        self.current_job = None;
        self.step_results.clear();
        self.step_outputs.clear();

        // Merge stage-level variables
        self.merge_variables(&stage.variables);
    }

    /// Exit current stage with result
    pub fn exit_stage(&mut self, result: StageResult) {
        if let Some(stage_name) = self.current_stage.take() {
            self.stage_results.insert(stage_name, result);
        }
    }

    /// Enter a job (set current job and merge variables)
    pub fn enter_job(&mut self, job: &Job) {
        self.current_job = job.identifier().map(|s| s.to_string());
        self.step_results.clear();
        self.step_outputs.clear();

        // Merge job-level variables
        self.merge_variables(&job.variables);
    }

    /// Exit current job with result
    pub fn exit_job(&mut self, result: JobResult) {
        let key = match (&self.current_stage, &self.current_job) {
            (Some(stage), Some(job)) => format!("{}.{}", stage, job),
            (None, Some(job)) => job.clone(),
            _ => return,
        };
        self.job_results.insert(key, result);
        self.current_job = None;
    }

    /// Record a step result
    pub fn record_step_result(&mut self, result: StepResult) {
        // Store step outputs
        if let Some(step_name) = &result.step_name {
            if !result.outputs.is_empty() {
                self.step_outputs.insert(
                    step_name.clone(),
                    result
                        .outputs
                        .iter()
                        .map(|(k, v)| (k.clone(), Value::String(v.clone())))
                        .collect(),
                );
            }
        }

        self.step_results.push(result);
    }

    /// Set a variable during execution
    pub fn set_variable(&mut self, name: String, value: Value) {
        self.variables.insert(name, value);
    }

    /// Set an output variable for the current step
    pub fn set_step_output(&mut self, step_name: String, output_name: String, value: Value) {
        self.step_outputs
            .entry(step_name)
            .or_default()
            .insert(output_name, value);
    }

    /// Set an environment variable
    pub fn set_env(&mut self, name: String, value: Value) {
        self.env.insert(name, value);
    }

    /// Merge variables from a variable list
    fn merge_variables(&mut self, variables: &[Variable]) {
        for var in variables {
            match var {
                Variable::KeyValue { name, value, .. } => {
                    let trimmed = value.trim();
                    if trimmed.starts_with("$[") && trimmed.ends_with(']') {
                        // Runtime expression ($[...]): evaluate the inner expression
                        let inner = &trimmed[2..trimmed.len() - 1];
                        let engine = self.expression_engine();
                        match engine.evaluate_runtime(inner) {
                            Ok(result) => {
                                self.variables.insert(name.clone(), result);
                            }
                            Err(_) => {
                                // If evaluation fails, store the raw string
                                self.variables
                                    .insert(name.clone(), Value::String(value.clone()));
                            }
                        }
                    } else if trimmed.starts_with("${{") && trimmed.ends_with("}}") {
                        // Compile-time expression (${{ expr }}): evaluate it now since
                        // template resolution may not have processed pipeline-level variables.
                        let inner = &trimmed[3..trimmed.len() - 2].trim();
                        let engine = self.expression_engine();
                        match engine.evaluate_compile_time(inner) {
                            Ok(result) => {
                                self.variables.insert(name.clone(), result);
                            }
                            Err(_) => {
                                self.variables
                                    .insert(name.clone(), Value::String(value.clone()));
                            }
                        }
                    } else if trimmed.contains("${{") {
                        // Value contains inline compile-time expressions; use substitute_macros
                        // which handles ${{ }}, $[ ], and $() patterns within a string.
                        let engine = self.expression_engine();
                        match engine.substitute_macros(trimmed) {
                            Ok(result) => {
                                self.variables.insert(name.clone(), Value::String(result));
                            }
                            Err(_) => {
                                self.variables
                                    .insert(name.clone(), Value::String(value.clone()));
                            }
                        }
                    } else {
                        self.variables
                            .insert(name.clone(), Value::String(value.clone()));
                    }
                }
                Variable::Group { .. } => {
                    // Variable groups would need to be resolved from external source
                    // For now, skip them
                }
                Variable::Template { .. } => {
                    // Template variables would be expanded earlier
                    // For now, skip them
                }
            }
        }
    }

    /// Merge pipeline-level variables (public entry point for the executor)
    pub fn merge_pipeline_variables(&mut self, variables: &[Variable]) {
        self.merge_variables(variables);
    }

    /// Build an ExpressionContext for evaluating conditions
    pub fn to_expression_context(&self) -> ExpressionContext {
        let mut ctx = ExpressionContext {
            variables: self.variables.clone(),
            parameters: self.parameters.clone(),
            pipeline: PipelineContext {
                name: Some(self.base.pipeline_name.clone()),
                workspace: Some(self.base.working_dir.clone()),
            },
            ..Default::default()
        };

        // Stage context
        if let Some(stage_name) = &self.current_stage {
            ctx.stage = Some(StageContext {
                name: stage_name.clone(),
                display_name: None, // Could be enhanced
            });
        }

        // Job context
        if let Some(job_name) = &self.current_job {
            ctx.job = Some(JobContext {
                name: job_name.clone(),
                display_name: None,
                agent: Default::default(),
                status: self.current_job_status(),
            });
        }

        // Step outputs
        for (step_name, outputs) in &self.step_outputs {
            let step_status = self
                .step_results
                .iter()
                .find(|r| r.step_name.as_deref() == Some(step_name))
                .map(|r| StepStatusContext {
                    succeeded: r.status == StepStatus::Succeeded
                        || r.status == StepStatus::SucceededWithIssues,
                    failed: r.status == StepStatus::Failed,
                    skipped: r.status == StepStatus::Skipped,
                })
                .unwrap_or_default();

            ctx.steps.insert(
                step_name.clone(),
                StepContext {
                    outputs: outputs.clone(),
                    status: step_status,
                },
            );
        }

        // Dependencies
        ctx.dependencies = self.build_dependencies_context();

        // Environment
        ctx.env = self.env.clone();

        ctx
    }

    /// Create an expression engine with current context
    pub fn expression_engine(&self) -> ExpressionEngine {
        ExpressionEngine::new(self.to_expression_context())
    }

    /// Evaluate a condition expression
    pub fn evaluate_condition(&self, condition: &str) -> Result<bool, String> {
        let engine = self.expression_engine();
        engine
            .evaluate_runtime(condition)
            .map(|v| v.is_truthy())
            .map_err(|e| e.message)
    }

    /// Substitute variables in a string ($(var) syntax)
    pub fn substitute_variables(&self, text: &str) -> Result<String, String> {
        let engine = self.expression_engine();
        engine.substitute_macros(text).map_err(|e| e.message)
    }

    /// Get current job status context
    fn current_job_status(&self) -> JobStatusContext {
        // Determine job status based on step results
        let has_failed = self
            .step_results
            .iter()
            .any(|r| r.status == StepStatus::Failed);

        JobStatusContext {
            succeeded: !has_failed && !self.step_results.is_empty(),
            failed: has_failed,
            canceled: false, // Would need cancellation tracking
        }
    }

    /// Build dependencies context from completed stages/jobs
    fn build_dependencies_context(&self) -> DependenciesContext {
        let mut ctx = DependenciesContext::default();

        // Stage dependencies
        for (stage_name, result) in &self.stage_results {
            let mut outputs = HashMap::new();

            // Collect job outputs within this stage
            for (job_key, job_result) in &self.job_results {
                if job_key.starts_with(&format!("{}.", stage_name)) {
                    let job_name = job_key.strip_prefix(&format!("{}.", stage_name)).unwrap();
                    outputs.insert(
                        job_name.to_string(),
                        job_result
                            .outputs
                            .iter()
                            .map(|(k, v)| (k.clone(), Value::String(v.clone())))
                            .collect(),
                    );
                }
            }

            ctx.stages.insert(
                stage_name.clone(),
                StageDependency {
                    outputs,
                    result: status_to_string(&result.status),
                },
            );
        }

        // Job dependencies (within current stage)
        if let Some(current_stage) = &self.current_stage {
            for (job_key, job_result) in &self.job_results {
                // Only include jobs from current stage for job-level dependencies
                if let Some(job_name) = job_key.strip_prefix(&format!("{}.", current_stage)) {
                    ctx.jobs.insert(
                        job_name.to_string(),
                        JobDependency {
                            outputs: job_result
                                .outputs
                                .iter()
                                .map(|(k, v)| (k.clone(), Value::String(v.clone())))
                                .collect(),
                            result: job_status_to_string(&job_result.status),
                        },
                    );
                }
            }
        }

        ctx
    }

    /// Check if all dependencies succeeded
    pub fn dependencies_succeeded(&self, deps: &[String], is_stage: bool) -> bool {
        if is_stage {
            deps.iter().all(|dep| {
                self.stage_results
                    .get(dep)
                    .map(|r| {
                        r.status == StageStatus::Succeeded
                            || r.status == StageStatus::SucceededWithIssues
                    })
                    .unwrap_or(false)
            })
        } else {
            // Job dependencies (within current stage)
            let stage_prefix = self
                .current_stage
                .as_ref()
                .map(|s| format!("{}.", s))
                .unwrap_or_default();

            deps.iter().all(|dep| {
                let key = format!("{}{}", stage_prefix, dep);
                self.job_results
                    .get(&key)
                    .map(|r| {
                        r.status == JobStatus::Succeeded
                            || r.status == JobStatus::SucceededWithIssues
                    })
                    .unwrap_or(false)
            })
        }
    }

    /// Get environment variables as string map for process execution
    pub fn env_as_strings(&self) -> HashMap<String, String> {
        let mut env = HashMap::new();

        // Add base environment
        for (k, v) in &self.base.env {
            env.insert(k.clone(), v.clone());
        }

        // Add context environment
        for (k, v) in &self.env {
            env.insert(k.clone(), v.as_string());
        }

        // Add common Azure DevOps variables
        env.insert(
            "BUILD_SOURCESDIRECTORY".to_string(),
            self.base.working_dir.clone(),
        );
        env.insert(
            "SYSTEM_DEFAULTWORKINGDIRECTORY".to_string(),
            self.base.working_dir.clone(),
        );
        env.insert(
            "PIPELINE_WORKSPACE".to_string(),
            self.base.working_dir.clone(),
        );

        if let Some(stage) = &self.current_stage {
            env.insert("SYSTEM_STAGENAME".to_string(), stage.clone());
            env.insert("SYSTEM_STAGEDISPLAYNAME".to_string(), stage.clone());
        }

        if let Some(job) = &self.current_job {
            env.insert("SYSTEM_JOBNAME".to_string(), job.clone());
            env.insert("SYSTEM_JOBDISPLAYNAME".to_string(), job.clone());
        }

        env
    }
}

/// Convert serde_yaml::Value to our Value type
fn yaml_to_value(yaml: &serde_yaml::Value) -> Value {
    match yaml {
        serde_yaml::Value::Null => Value::Null,
        serde_yaml::Value::Bool(b) => Value::Bool(*b),
        serde_yaml::Value::Number(n) => {
            Value::Number(n.as_f64().unwrap_or(n.as_i64().unwrap_or(0) as f64))
        }
        serde_yaml::Value::String(s) => Value::String(s.clone()),
        serde_yaml::Value::Sequence(seq) => Value::Array(seq.iter().map(yaml_to_value).collect()),
        serde_yaml::Value::Mapping(map) => Value::Object(
            map.iter()
                .filter_map(|(k, v)| k.as_str().map(|key| (key.to_string(), yaml_to_value(v))))
                .collect(),
        ),
        serde_yaml::Value::Tagged(_) => Value::Null,
    }
}

fn status_to_string(status: &StageStatus) -> String {
    match status {
        StageStatus::Succeeded => "Succeeded".to_string(),
        StageStatus::SucceededWithIssues => "SucceededWithIssues".to_string(),
        StageStatus::Failed => "Failed".to_string(),
        StageStatus::Canceled => "Canceled".to_string(),
        StageStatus::Skipped => "Skipped".to_string(),
        StageStatus::Pending | StageStatus::Running => "InProgress".to_string(),
    }
}

fn job_status_to_string(status: &JobStatus) -> String {
    match status {
        JobStatus::Succeeded => "Succeeded".to_string(),
        JobStatus::SucceededWithIssues => "SucceededWithIssues".to_string(),
        JobStatus::Failed => "Failed".to_string(),
        JobStatus::Canceled => "Canceled".to_string(),
        JobStatus::Skipped => "Skipped".to_string(),
        JobStatus::Pending | JobStatus::Running => "InProgress".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_runtime_context_creation() {
        let base = ExecutionContext::new("test-pipeline".to_string(), "/work".to_string());
        let ctx = RuntimeContext::new(base);

        assert_eq!(ctx.base.pipeline_name, "test-pipeline");
        assert_eq!(ctx.base.working_dir, "/work");
        assert!(ctx.current_stage.is_none());
        assert!(ctx.current_job.is_none());
    }

    #[test]
    fn test_enter_exit_stage() {
        let base = ExecutionContext::new("test".to_string(), "/work".to_string());
        let mut ctx = RuntimeContext::new(base);

        let stage = Stage {
            stage: Some("Build".to_string()),
            display_name: None,
            depends_on: Default::default(),
            condition: None,
            variables: vec![Variable::KeyValue {
                name: "stage_var".to_string(),
                value: "stage_value".to_string(),
                readonly: false,
            }],
            jobs: Vec::new(),
            lock_behavior: None,
            template: None,
            parameters: HashMap::new(),
            pool: None,
            has_template_directives: false,
        };

        ctx.enter_stage(&stage);
        assert_eq!(ctx.current_stage, Some("Build".to_string()));
        assert_eq!(
            ctx.variables.get("stage_var"),
            Some(&Value::String("stage_value".to_string()))
        );

        let result = StageResult {
            stage_name: "Build".to_string(),
            display_name: None,
            status: StageStatus::Succeeded,
            jobs: Vec::new(),
            duration: Duration::from_secs(10),
        };

        ctx.exit_stage(result);
        assert!(ctx.current_stage.is_none());
        assert!(ctx.stage_results.contains_key("Build"));
    }

    #[test]
    fn test_evaluate_condition() {
        let mut base = ExecutionContext::new("test".to_string(), "/work".to_string());
        base.variables
            .insert("isRelease".to_string(), "true".to_string());

        let ctx = RuntimeContext::new(base);

        assert!(ctx
            .evaluate_condition("eq(variables.isRelease, 'true')")
            .unwrap());
        assert!(!ctx
            .evaluate_condition("eq(variables.isRelease, 'false')")
            .unwrap());
    }

    #[test]
    fn test_substitute_variables() {
        let mut base = ExecutionContext::new("test".to_string(), "/work".to_string());
        base.variables
            .insert("version".to_string(), "1.0.0".to_string());

        let ctx = RuntimeContext::new(base);

        let result = ctx.substitute_variables("Version: $(version)").unwrap();
        assert_eq!(result, "Version: 1.0.0");
    }

    #[test]
    fn test_step_outputs() {
        let base = ExecutionContext::new("test".to_string(), "/work".to_string());
        let mut ctx = RuntimeContext::new(base);

        ctx.set_step_output(
            "GetVersion".to_string(),
            "version".to_string(),
            Value::String("2.0.0".to_string()),
        );

        let expr_ctx = ctx.to_expression_context();
        let step_ctx = expr_ctx.steps.get("GetVersion").unwrap();
        assert_eq!(
            step_ctx.outputs.get("version"),
            Some(&Value::String("2.0.0".to_string()))
        );
    }

    #[test]
    fn test_dependencies_succeeded() {
        let base = ExecutionContext::new("test".to_string(), "/work".to_string());
        let mut ctx = RuntimeContext::new(base);

        // Add a completed stage
        ctx.stage_results.insert(
            "Build".to_string(),
            StageResult {
                stage_name: "Build".to_string(),
                display_name: None,
                status: StageStatus::Succeeded,
                jobs: Vec::new(),
                duration: Duration::from_secs(10),
            },
        );

        assert!(ctx.dependencies_succeeded(&["Build".to_string()], true));
        assert!(!ctx.dependencies_succeeded(&["Test".to_string()], true));
    }
}
