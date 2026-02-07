// Assertion Logic
// Evaluates test assertions against pipeline execution results

use crate::execution::executor::ExecutionResult;
use crate::parser::models::{
    JobResult, JobStatus, StageResult, StageStatus, StepResult, StepStatus, Value,
};

use std::fmt;

// =============================================================================
// Assertion Types (evaluable form)
// =============================================================================

/// An evaluable assertion against pipeline execution results
#[derive(Debug, Clone)]
pub enum Assertion {
    // Pipeline-level assertions
    PipelineSucceeded,
    PipelineFailed,

    // Step status assertions
    StepSucceeded {
        step: String,
    },
    StepFailed {
        step: String,
    },
    StepSkipped {
        step: String,
    },

    // Job status assertions
    JobSucceeded {
        job: String,
    },
    JobFailed {
        job: String,
    },
    JobSkipped {
        job: String,
    },

    // Stage status assertions
    StageSucceeded {
        stage: String,
    },
    StageFailed {
        stage: String,
    },
    StageSkipped {
        stage: String,
    },

    // Output assertions
    StepOutputEquals {
        step: String,
        output: String,
        expected: Value,
    },
    StepOutputContains {
        step: String,
        pattern: String,
        /// If Some, check a specific output variable; if None, check stdout
        output: Option<String>,
    },

    // Order assertions
    StepRanBefore {
        step: String,
        before: String,
    },
    StepsRanInParallel {
        steps: Vec<String>,
    },

    // Variable assertions
    VariableEquals {
        name: String,
        expected: Value,
    },
    VariableContains {
        name: String,
        pattern: String,
    },
}

/// Result of evaluating a single assertion
#[derive(Debug, Clone)]
pub struct AssertionResult {
    /// The assertion that was evaluated
    pub assertion: String,
    /// Whether the assertion passed
    pub passed: bool,
    /// Human-readable description of what was checked
    pub message: String,
    /// Details about the failure (if any)
    pub failure_detail: Option<String>,
}

impl AssertionResult {
    fn pass(assertion: &str, message: impl Into<String>) -> Self {
        Self {
            assertion: assertion.to_string(),
            passed: true,
            message: message.into(),
            failure_detail: None,
        }
    }

    fn fail(assertion: &str, message: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            assertion: assertion.to_string(),
            passed: false,
            message: message.into(),
            failure_detail: Some(detail.into()),
        }
    }
}

impl fmt::Display for Assertion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Assertion::PipelineSucceeded => write!(f, "pipeline_succeeded"),
            Assertion::PipelineFailed => write!(f, "pipeline_failed"),
            Assertion::StepSucceeded { step } => write!(f, "step_succeeded({})", step),
            Assertion::StepFailed { step } => write!(f, "step_failed({})", step),
            Assertion::StepSkipped { step } => write!(f, "step_skipped({})", step),
            Assertion::JobSucceeded { job } => write!(f, "job_succeeded({})", job),
            Assertion::JobFailed { job } => write!(f, "job_failed({})", job),
            Assertion::JobSkipped { job } => write!(f, "job_skipped({})", job),
            Assertion::StageSucceeded { stage } => write!(f, "stage_succeeded({})", stage),
            Assertion::StageFailed { stage } => write!(f, "stage_failed({})", stage),
            Assertion::StageSkipped { stage } => write!(f, "stage_skipped({})", stage),
            Assertion::StepOutputEquals {
                step,
                output,
                expected,
            } => write!(
                f,
                "step_output_equals({}.{} == {})",
                step,
                output,
                expected.as_string()
            ),
            Assertion::StepOutputContains { step, pattern, .. } => {
                write!(f, "step_output_contains({}, \"{}\")", step, pattern)
            }
            Assertion::StepRanBefore { step, before } => {
                write!(f, "step_ran_before({}, {})", step, before)
            }
            Assertion::StepsRanInParallel { steps } => {
                write!(f, "steps_ran_in_parallel({})", steps.join(", "))
            }
            Assertion::VariableEquals { name, expected } => {
                write!(f, "variable_equals({} == {})", name, expected.as_string())
            }
            Assertion::VariableContains { name, pattern } => {
                write!(f, "variable_contains({}, \"{}\")", name, pattern)
            }
        }
    }
}

// =============================================================================
// Assertion Evaluator
// =============================================================================

/// Evaluates assertions against execution results
pub struct AssertionEvaluator<'a> {
    result: &'a ExecutionResult,
    /// Flattened step results with their indices for ordering
    step_index: Vec<StepInfo>,
}

/// Flattened step info for lookup
#[derive(Debug)]
struct StepInfo {
    name: Option<String>,
    display_name: Option<String>,
    #[allow(dead_code)]
    stage_name: String,
    job_name: String,
    index: usize, // Global execution order
    result: StepResult,
}

impl<'a> AssertionEvaluator<'a> {
    /// Create a new evaluator from execution results
    pub fn new(result: &'a ExecutionResult) -> Self {
        let step_index = Self::build_step_index(result);
        Self { result, step_index }
    }

    /// Evaluate a single assertion
    pub fn evaluate(&self, assertion: &Assertion) -> AssertionResult {
        match assertion {
            Assertion::PipelineSucceeded => self.eval_pipeline_succeeded(),
            Assertion::PipelineFailed => self.eval_pipeline_failed(),
            Assertion::StepSucceeded { step } => self.eval_step_status(step, StepStatus::Succeeded),
            Assertion::StepFailed { step } => self.eval_step_status(step, StepStatus::Failed),
            Assertion::StepSkipped { step } => self.eval_step_status(step, StepStatus::Skipped),
            Assertion::JobSucceeded { job } => self.eval_job_status(job, JobStatus::Succeeded),
            Assertion::JobFailed { job } => self.eval_job_status(job, JobStatus::Failed),
            Assertion::JobSkipped { job } => self.eval_job_status(job, JobStatus::Skipped),
            Assertion::StageSucceeded { stage } => {
                self.eval_stage_status(stage, StageStatus::Succeeded)
            }
            Assertion::StageFailed { stage } => self.eval_stage_status(stage, StageStatus::Failed),
            Assertion::StageSkipped { stage } => {
                self.eval_stage_status(stage, StageStatus::Skipped)
            }
            Assertion::StepOutputEquals {
                step,
                output,
                expected,
            } => self.eval_step_output_equals(step, output, expected),
            Assertion::StepOutputContains {
                step,
                pattern,
                output,
            } => self.eval_step_output_contains(step, pattern, output.as_deref()),
            Assertion::StepRanBefore { step, before } => self.eval_step_ran_before(step, before),
            Assertion::StepsRanInParallel { steps } => self.eval_steps_ran_in_parallel(steps),
            Assertion::VariableEquals { name, expected } => {
                self.eval_variable_equals(name, expected)
            }
            Assertion::VariableContains { name, pattern } => {
                self.eval_variable_contains(name, pattern)
            }
        }
    }

    /// Evaluate all assertions and return results
    pub fn evaluate_all(&self, assertions: &[Assertion]) -> Vec<AssertionResult> {
        assertions.iter().map(|a| self.evaluate(a)).collect()
    }

    // =========================================================================
    // Pipeline assertions
    // =========================================================================

    fn eval_pipeline_succeeded(&self) -> AssertionResult {
        let desc = "pipeline_succeeded";
        if self.result.success {
            AssertionResult::pass(desc, "Pipeline completed successfully")
        } else {
            let failed_stages: Vec<&str> = self
                .result
                .stages
                .iter()
                .filter(|s| s.status == StageStatus::Failed)
                .map(|s| s.stage_name.as_str())
                .collect();
            AssertionResult::fail(
                desc,
                "Pipeline did not succeed",
                format!(
                    "Pipeline failed. Failed stages: [{}]",
                    failed_stages.join(", ")
                ),
            )
        }
    }

    fn eval_pipeline_failed(&self) -> AssertionResult {
        let desc = "pipeline_failed";
        if !self.result.success {
            AssertionResult::pass(desc, "Pipeline failed as expected")
        } else {
            AssertionResult::fail(
                desc,
                "Pipeline was expected to fail but succeeded",
                "All stages completed successfully",
            )
        }
    }

    // =========================================================================
    // Step status assertions
    // =========================================================================

    fn eval_step_status(&self, step_name: &str, expected: StepStatus) -> AssertionResult {
        let desc = format!("step_{:?}({})", expected, step_name).to_lowercase();

        match self.find_step(step_name) {
            Some(info) => {
                if info.result.status == expected {
                    AssertionResult::pass(
                        &desc,
                        format!("Step '{}' has status {:?}", step_name, expected),
                    )
                } else {
                    AssertionResult::fail(
                        &desc,
                        format!(
                            "Step '{}' expected {:?} but was {:?}",
                            step_name, expected, info.result.status
                        ),
                        format!(
                            "Actual status: {:?}{}",
                            info.result.status,
                            info.result
                                .error
                                .as_ref()
                                .map(|e| format!(", error: {}", e))
                                .unwrap_or_default()
                        ),
                    )
                }
            }
            None => AssertionResult::fail(
                &desc,
                format!("Step '{}' not found in execution results", step_name),
                format!(
                    "Available steps: [{}]",
                    self.step_index
                        .iter()
                        .filter_map(|s| s.name.as_deref())
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            ),
        }
    }

    // =========================================================================
    // Job status assertions
    // =========================================================================

    fn eval_job_status(&self, job_name: &str, expected: JobStatus) -> AssertionResult {
        let desc = format!("job_{:?}({})", expected, job_name).to_lowercase();

        match self.find_job(job_name) {
            Some(job) => {
                if job.status == expected {
                    AssertionResult::pass(
                        &desc,
                        format!("Job '{}' has status {:?}", job_name, expected),
                    )
                } else {
                    AssertionResult::fail(
                        &desc,
                        format!(
                            "Job '{}' expected {:?} but was {:?}",
                            job_name, expected, job.status
                        ),
                        format!("Actual status: {:?}", job.status),
                    )
                }
            }
            None => {
                let available: Vec<&str> = self
                    .result
                    .stages
                    .iter()
                    .flat_map(|s| s.jobs.iter())
                    .map(|j| j.job_name.as_str())
                    .collect();
                AssertionResult::fail(
                    &desc,
                    format!("Job '{}' not found in execution results", job_name),
                    format!("Available jobs: [{}]", available.join(", ")),
                )
            }
        }
    }

    // =========================================================================
    // Stage status assertions
    // =========================================================================

    fn eval_stage_status(&self, stage_name: &str, expected: StageStatus) -> AssertionResult {
        let desc = format!("stage_{:?}({})", expected, stage_name).to_lowercase();

        match self.find_stage(stage_name) {
            Some(stage) => {
                if stage.status == expected {
                    AssertionResult::pass(
                        &desc,
                        format!("Stage '{}' has status {:?}", stage_name, expected),
                    )
                } else {
                    AssertionResult::fail(
                        &desc,
                        format!(
                            "Stage '{}' expected {:?} but was {:?}",
                            stage_name, expected, stage.status
                        ),
                        format!("Actual status: {:?}", stage.status),
                    )
                }
            }
            None => {
                let available: Vec<&str> = self
                    .result
                    .stages
                    .iter()
                    .map(|s| s.stage_name.as_str())
                    .collect();
                AssertionResult::fail(
                    &desc,
                    format!("Stage '{}' not found in execution results", stage_name),
                    format!("Available stages: [{}]", available.join(", ")),
                )
            }
        }
    }

    // =========================================================================
    // Output assertions
    // =========================================================================

    fn eval_step_output_equals(
        &self,
        step_name: &str,
        output_name: &str,
        expected: &Value,
    ) -> AssertionResult {
        let desc = format!("step_output_equals({}.{})", step_name, output_name);

        match self.find_step(step_name) {
            Some(info) => {
                if let Some(actual) = info.result.outputs.get(output_name) {
                    let expected_str = expected.as_string();
                    if *actual == expected_str {
                        AssertionResult::pass(
                            &desc,
                            format!(
                                "Step '{}' output '{}' equals '{}'",
                                step_name, output_name, expected_str
                            ),
                        )
                    } else {
                        AssertionResult::fail(
                            &desc,
                            format!(
                                "Step '{}' output '{}' does not match",
                                step_name, output_name
                            ),
                            format!("Expected: '{}', Actual: '{}'", expected_str, actual),
                        )
                    }
                } else {
                    let available: Vec<&str> =
                        info.result.outputs.keys().map(|k| k.as_str()).collect();
                    AssertionResult::fail(
                        &desc,
                        format!("Step '{}' has no output named '{}'", step_name, output_name),
                        format!("Available outputs: [{}]", available.join(", ")),
                    )
                }
            }
            None => AssertionResult::fail(
                &desc,
                format!("Step '{}' not found", step_name),
                self.available_steps_hint(),
            ),
        }
    }

    fn eval_step_output_contains(
        &self,
        step_name: &str,
        pattern: &str,
        output_name: Option<&str>,
    ) -> AssertionResult {
        let desc = format!("step_output_contains({}, \"{}\")", step_name, pattern);

        match self.find_step(step_name) {
            Some(info) => {
                let text = if let Some(output_key) = output_name {
                    info.result
                        .outputs
                        .get(output_key)
                        .cloned()
                        .unwrap_or_default()
                } else {
                    // Check stdout (the main output field)
                    info.result.output.clone()
                };

                if text.contains(pattern) {
                    AssertionResult::pass(
                        &desc,
                        format!("Step '{}' output contains '{}'", step_name, pattern),
                    )
                } else {
                    let preview = if text.len() > 200 {
                        format!("{}...", &text[..200])
                    } else {
                        text
                    };
                    AssertionResult::fail(
                        &desc,
                        format!("Step '{}' output does not contain '{}'", step_name, pattern),
                        format!("Actual output: '{}'", preview),
                    )
                }
            }
            None => AssertionResult::fail(
                &desc,
                format!("Step '{}' not found", step_name),
                self.available_steps_hint(),
            ),
        }
    }

    // =========================================================================
    // Order assertions
    // =========================================================================

    fn eval_step_ran_before(&self, step_name: &str, before_name: &str) -> AssertionResult {
        let desc = format!("step_ran_before({}, {})", step_name, before_name);

        let first = self.find_step(step_name);
        let second = self.find_step(before_name);

        match (first, second) {
            (Some(first_info), Some(second_info)) => {
                if first_info.index < second_info.index {
                    AssertionResult::pass(
                        &desc,
                        format!(
                            "Step '{}' (index {}) ran before '{}' (index {})",
                            step_name, first_info.index, before_name, second_info.index
                        ),
                    )
                } else {
                    AssertionResult::fail(
                        &desc,
                        format!("Step '{}' did not run before '{}'", step_name, before_name),
                        format!(
                            "'{}' ran at index {}, '{}' ran at index {}",
                            step_name, first_info.index, before_name, second_info.index
                        ),
                    )
                }
            }
            (None, _) => AssertionResult::fail(
                &desc,
                format!("Step '{}' not found", step_name),
                self.available_steps_hint(),
            ),
            (_, None) => AssertionResult::fail(
                &desc,
                format!("Step '{}' not found", before_name),
                self.available_steps_hint(),
            ),
        }
    }

    fn eval_steps_ran_in_parallel(&self, step_names: &[String]) -> AssertionResult {
        let desc = format!("steps_ran_in_parallel({})", step_names.join(", "));

        // For parallel assertion, we check that steps belong to the same stage
        // but different jobs (or same job with parallel strategy)
        let mut found_steps: Vec<&StepInfo> = Vec::new();
        let mut missing: Vec<&str> = Vec::new();

        for name in step_names {
            if let Some(info) = self.find_step(name) {
                found_steps.push(info);
            } else {
                missing.push(name);
            }
        }

        if !missing.is_empty() {
            return AssertionResult::fail(
                &desc,
                format!("Steps not found: [{}]", missing.join(", ")),
                self.available_steps_hint(),
            );
        }

        // Check if steps are in different jobs (which could run in parallel)
        let job_names: std::collections::HashSet<&str> =
            found_steps.iter().map(|s| s.job_name.as_str()).collect();

        if job_names.len() > 1 {
            // Steps are in different jobs - they could run in parallel
            AssertionResult::pass(
                &desc,
                format!(
                    "Steps are in different jobs [{}] and could run in parallel",
                    job_names.into_iter().collect::<Vec<_>>().join(", ")
                ),
            )
        } else {
            AssertionResult::fail(
                &desc,
                "Steps are all in the same job and run sequentially",
                format!(
                    "All steps are in job '{}'. Steps within a single job always run sequentially.",
                    found_steps[0].job_name
                ),
            )
        }
    }

    // =========================================================================
    // Variable assertions
    // =========================================================================

    fn eval_variable_equals(&self, name: &str, expected: &Value) -> AssertionResult {
        let desc = format!("variable_equals({})", name);
        let expected_str = expected.as_string();

        match self.result.variables.get(name) {
            Some(actual) => {
                if *actual == expected_str {
                    AssertionResult::pass(
                        &desc,
                        format!("Variable '{}' equals '{}'", name, expected_str),
                    )
                } else {
                    AssertionResult::fail(
                        &desc,
                        format!("Variable '{}' does not match", name),
                        format!("Expected: '{}', Actual: '{}'", expected_str, actual),
                    )
                }
            }
            None => {
                let available: Vec<&str> =
                    self.result.variables.keys().map(|k| k.as_str()).collect();
                AssertionResult::fail(
                    &desc,
                    format!("Variable '{}' not found", name),
                    format!("Available variables: [{}]", available.join(", ")),
                )
            }
        }
    }

    fn eval_variable_contains(&self, name: &str, pattern: &str) -> AssertionResult {
        let desc = format!("variable_contains({}, \"{}\")", name, pattern);

        match self.result.variables.get(name) {
            Some(actual) => {
                if actual.contains(pattern) {
                    AssertionResult::pass(
                        &desc,
                        format!("Variable '{}' contains '{}'", name, pattern),
                    )
                } else {
                    AssertionResult::fail(
                        &desc,
                        format!("Variable '{}' does not contain '{}'", name, pattern),
                        format!("Actual value: '{}'", actual),
                    )
                }
            }
            None => AssertionResult::fail(
                &desc,
                format!("Variable '{}' not found", name),
                format!(
                    "Available variables: [{}]",
                    self.result
                        .variables
                        .keys()
                        .map(|k| k.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            ),
        }
    }

    // =========================================================================
    // Lookup helpers
    // =========================================================================

    fn find_step(&self, name: &str) -> Option<&StepInfo> {
        self.step_index
            .iter()
            .find(|s| s.name.as_deref() == Some(name) || s.display_name.as_deref() == Some(name))
    }

    fn find_job(&self, name: &str) -> Option<&JobResult> {
        self.result
            .stages
            .iter()
            .flat_map(|s| s.jobs.iter())
            .find(|j| j.job_name == name || j.display_name.as_deref() == Some(name))
    }

    fn find_stage(&self, name: &str) -> Option<&StageResult> {
        self.result
            .stages
            .iter()
            .find(|s| s.stage_name == name || s.display_name.as_deref() == Some(name))
    }

    fn available_steps_hint(&self) -> String {
        let names: Vec<&str> = self
            .step_index
            .iter()
            .filter_map(|s| s.name.as_deref())
            .collect();
        format!("Available steps: [{}]", names.join(", "))
    }

    fn build_step_index(result: &ExecutionResult) -> Vec<StepInfo> {
        let mut steps = Vec::new();
        let mut global_index = 0;

        for stage in &result.stages {
            for job in &stage.jobs {
                for step in &job.steps {
                    steps.push(StepInfo {
                        name: step.step_name.clone(),
                        display_name: step.display_name.clone(),
                        stage_name: stage.stage_name.clone(),
                        job_name: job.job_name.clone(),
                        index: global_index,
                        result: step.clone(),
                    });
                    global_index += 1;
                }
            }
        }

        steps
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::time::Duration;

    fn make_step(name: &str, status: StepStatus) -> StepResult {
        let exit_code = if status == StepStatus::Succeeded {
            0
        } else {
            1
        };
        StepResult {
            step_name: Some(name.to_string()),
            display_name: None,
            status,
            output: String::new(),
            error: None,
            duration: Duration::from_millis(100),
            exit_code: Some(exit_code),
            outputs: HashMap::new(),
        }
    }

    fn make_job(name: &str, status: JobStatus, steps: Vec<StepResult>) -> JobResult {
        JobResult {
            job_name: name.to_string(),
            display_name: None,
            status,
            steps,
            duration: Duration::from_millis(500),
            outputs: HashMap::new(),
        }
    }

    fn make_stage(name: &str, status: StageStatus, jobs: Vec<JobResult>) -> StageResult {
        StageResult {
            stage_name: name.to_string(),
            display_name: None,
            status,
            jobs,
            duration: Duration::from_secs(1),
        }
    }

    fn make_result(stages: Vec<StageResult>, success: bool) -> ExecutionResult {
        ExecutionResult {
            stages,
            duration: Duration::from_secs(5),
            success,
            variables: HashMap::new(),
        }
    }

    #[test]
    fn test_pipeline_succeeded() {
        let result = make_result(vec![], true);
        let evaluator = AssertionEvaluator::new(&result);
        let r = evaluator.evaluate(&Assertion::PipelineSucceeded);
        assert!(r.passed);
    }

    #[test]
    fn test_pipeline_succeeded_fails() {
        let result = make_result(
            vec![make_stage("Build", StageStatus::Failed, vec![])],
            false,
        );
        let evaluator = AssertionEvaluator::new(&result);
        let r = evaluator.evaluate(&Assertion::PipelineSucceeded);
        assert!(!r.passed);
        assert!(r.failure_detail.unwrap().contains("Build"));
    }

    #[test]
    fn test_pipeline_failed() {
        let result = make_result(vec![], false);
        let evaluator = AssertionEvaluator::new(&result);
        let r = evaluator.evaluate(&Assertion::PipelineFailed);
        assert!(r.passed);
    }

    #[test]
    fn test_step_succeeded() {
        let result = make_result(
            vec![make_stage(
                "Build",
                StageStatus::Succeeded,
                vec![make_job(
                    "compile",
                    JobStatus::Succeeded,
                    vec![make_step("Build", StepStatus::Succeeded)],
                )],
            )],
            true,
        );
        let evaluator = AssertionEvaluator::new(&result);
        let r = evaluator.evaluate(&Assertion::StepSucceeded {
            step: "Build".to_string(),
        });
        assert!(r.passed);
    }

    #[test]
    fn test_step_not_found() {
        let result = make_result(vec![], true);
        let evaluator = AssertionEvaluator::new(&result);
        let r = evaluator.evaluate(&Assertion::StepSucceeded {
            step: "Missing".to_string(),
        });
        assert!(!r.passed);
        assert!(r.message.contains("not found"));
    }

    #[test]
    fn test_step_wrong_status() {
        let result = make_result(
            vec![make_stage(
                "Build",
                StageStatus::Failed,
                vec![make_job(
                    "compile",
                    JobStatus::Failed,
                    vec![make_step("Build", StepStatus::Failed)],
                )],
            )],
            false,
        );
        let evaluator = AssertionEvaluator::new(&result);
        let r = evaluator.evaluate(&Assertion::StepSucceeded {
            step: "Build".to_string(),
        });
        assert!(!r.passed);
        assert!(r.failure_detail.unwrap().contains("Failed"));
    }

    #[test]
    fn test_step_output_contains() {
        let mut step = make_step("Build", StepStatus::Succeeded);
        step.output = "Build succeeded with 0 warnings".to_string();

        let result = make_result(
            vec![make_stage(
                "Build",
                StageStatus::Succeeded,
                vec![make_job("compile", JobStatus::Succeeded, vec![step])],
            )],
            true,
        );

        let evaluator = AssertionEvaluator::new(&result);
        let r = evaluator.evaluate(&Assertion::StepOutputContains {
            step: "Build".to_string(),
            pattern: "Build succeeded".to_string(),
            output: None,
        });
        assert!(r.passed);
    }

    #[test]
    fn test_step_output_not_contains() {
        let mut step = make_step("Build", StepStatus::Succeeded);
        step.output = "Build finished".to_string();

        let result = make_result(
            vec![make_stage(
                "Build",
                StageStatus::Succeeded,
                vec![make_job("compile", JobStatus::Succeeded, vec![step])],
            )],
            true,
        );

        let evaluator = AssertionEvaluator::new(&result);
        let r = evaluator.evaluate(&Assertion::StepOutputContains {
            step: "Build".to_string(),
            pattern: "succeeded".to_string(),
            output: None,
        });
        assert!(!r.passed);
    }

    #[test]
    fn test_step_output_equals() {
        let mut step = make_step("Build", StepStatus::Succeeded);
        step.outputs
            .insert("version".to_string(), "1.2.3".to_string());

        let result = make_result(
            vec![make_stage(
                "Build",
                StageStatus::Succeeded,
                vec![make_job("compile", JobStatus::Succeeded, vec![step])],
            )],
            true,
        );

        let evaluator = AssertionEvaluator::new(&result);
        let r = evaluator.evaluate(&Assertion::StepOutputEquals {
            step: "Build".to_string(),
            output: "version".to_string(),
            expected: Value::String("1.2.3".to_string()),
        });
        assert!(r.passed);
    }

    #[test]
    fn test_step_ran_before() {
        let result = make_result(
            vec![make_stage(
                "Build",
                StageStatus::Succeeded,
                vec![make_job(
                    "compile",
                    JobStatus::Succeeded,
                    vec![
                        make_step("Build", StepStatus::Succeeded),
                        make_step("Test", StepStatus::Succeeded),
                        make_step("Deploy", StepStatus::Succeeded),
                    ],
                )],
            )],
            true,
        );

        let evaluator = AssertionEvaluator::new(&result);

        // Build ran before Deploy
        let r = evaluator.evaluate(&Assertion::StepRanBefore {
            step: "Build".to_string(),
            before: "Deploy".to_string(),
        });
        assert!(r.passed);

        // Deploy did NOT run before Build
        let r = evaluator.evaluate(&Assertion::StepRanBefore {
            step: "Deploy".to_string(),
            before: "Build".to_string(),
        });
        assert!(!r.passed);
    }

    #[test]
    fn test_variable_equals() {
        let mut result = make_result(vec![], true);
        result
            .variables
            .insert("BUILD_CONFIG".to_string(), "Release".to_string());

        let evaluator = AssertionEvaluator::new(&result);
        let r = evaluator.evaluate(&Assertion::VariableEquals {
            name: "BUILD_CONFIG".to_string(),
            expected: Value::String("Release".to_string()),
        });
        assert!(r.passed);
    }

    #[test]
    fn test_variable_contains() {
        let mut result = make_result(vec![], true);
        result
            .variables
            .insert("OUTPUT".to_string(), "hello world".to_string());

        let evaluator = AssertionEvaluator::new(&result);
        let r = evaluator.evaluate(&Assertion::VariableContains {
            name: "OUTPUT".to_string(),
            pattern: "world".to_string(),
        });
        assert!(r.passed);
    }

    #[test]
    fn test_stage_succeeded() {
        let result = make_result(
            vec![make_stage("Build", StageStatus::Succeeded, vec![])],
            true,
        );

        let evaluator = AssertionEvaluator::new(&result);
        let r = evaluator.evaluate(&Assertion::StageSucceeded {
            stage: "Build".to_string(),
        });
        assert!(r.passed);
    }

    #[test]
    fn test_job_skipped() {
        let result = make_result(
            vec![make_stage(
                "Deploy",
                StageStatus::Succeeded,
                vec![make_job("deploy_prod", JobStatus::Skipped, vec![])],
            )],
            true,
        );

        let evaluator = AssertionEvaluator::new(&result);
        let r = evaluator.evaluate(&Assertion::JobSkipped {
            job: "deploy_prod".to_string(),
        });
        assert!(r.passed);
    }

    #[test]
    fn test_steps_ran_in_parallel_different_jobs() {
        let result = make_result(
            vec![make_stage(
                "Build",
                StageStatus::Succeeded,
                vec![
                    make_job(
                        "linux",
                        JobStatus::Succeeded,
                        vec![make_step("BuildLinux", StepStatus::Succeeded)],
                    ),
                    make_job(
                        "windows",
                        JobStatus::Succeeded,
                        vec![make_step("BuildWindows", StepStatus::Succeeded)],
                    ),
                ],
            )],
            true,
        );

        let evaluator = AssertionEvaluator::new(&result);
        let r = evaluator.evaluate(&Assertion::StepsRanInParallel {
            steps: vec!["BuildLinux".to_string(), "BuildWindows".to_string()],
        });
        assert!(r.passed);
    }

    #[test]
    fn test_steps_ran_in_parallel_same_job_fails() {
        let result = make_result(
            vec![make_stage(
                "Build",
                StageStatus::Succeeded,
                vec![make_job(
                    "compile",
                    JobStatus::Succeeded,
                    vec![
                        make_step("Step1", StepStatus::Succeeded),
                        make_step("Step2", StepStatus::Succeeded),
                    ],
                )],
            )],
            true,
        );

        let evaluator = AssertionEvaluator::new(&result);
        let r = evaluator.evaluate(&Assertion::StepsRanInParallel {
            steps: vec!["Step1".to_string(), "Step2".to_string()],
        });
        assert!(!r.passed);
    }

    #[test]
    fn test_assertion_display() {
        let a = Assertion::StepSucceeded {
            step: "Build".to_string(),
        };
        assert_eq!(format!("{}", a), "step_succeeded(Build)");

        let a = Assertion::VariableEquals {
            name: "FOO".to_string(),
            expected: Value::String("bar".to_string()),
        };
        assert_eq!(format!("{}", a), "variable_equals(FOO == bar)");
    }

    #[test]
    fn test_evaluate_all() {
        let result = make_result(
            vec![make_stage(
                "Build",
                StageStatus::Succeeded,
                vec![make_job(
                    "compile",
                    JobStatus::Succeeded,
                    vec![make_step("Build", StepStatus::Succeeded)],
                )],
            )],
            true,
        );

        let evaluator = AssertionEvaluator::new(&result);
        let results = evaluator.evaluate_all(&[
            Assertion::PipelineSucceeded,
            Assertion::StepSucceeded {
                step: "Build".to_string(),
            },
            Assertion::StageSucceeded {
                stage: "Build".to_string(),
            },
        ]);
        assert_eq!(results.len(), 3);
        assert!(results.iter().all(|r| r.passed));
    }
}
