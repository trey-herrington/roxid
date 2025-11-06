use crate::pipeline::dependency::build_job_graph;
use crate::pipeline::models::{
    ExecutionContext, Job, JobResult, JobStatus, Pipeline, Stage, StageResult, StageStatus, Step,
    StepResult, StepStatus,
};
use crate::pipeline::runners::shell::ShellRunner;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc;

pub type ProgressSender = mpsc::UnboundedSender<ExecutionEvent>;
pub type ProgressReceiver = mpsc::UnboundedReceiver<ExecutionEvent>;

#[derive(Debug, Clone)]
pub enum ExecutionEvent {
    PipelineStarted {
        name: String,
    },
    StageStarted {
        stage_name: String,
        stage_index: usize,
    },
    StageCompleted {
        result: StageResult,
        stage_index: usize,
    },
    JobStarted {
        job_name: String,
        job_index: usize,
    },
    JobCompleted {
        result: JobResult,
        job_index: usize,
    },
    StepStarted {
        step_name: String,
        step_index: usize,
    },
    StepOutput {
        step_name: String,
        output: String,
    },
    StepCompleted {
        result: StepResult,
        step_index: usize,
    },
    PipelineCompleted {
        success: bool,
        total_steps: usize,
        failed_steps: usize,
    },
}

pub struct PipelineExecutor {
    context: ExecutionContext,
}

impl PipelineExecutor {
    pub fn new(context: ExecutionContext) -> Self {
        Self { context }
    }

    pub async fn execute(
        &self,
        pipeline: Pipeline,
        progress_tx: Option<ProgressSender>,
    ) -> Vec<StepResult> {
        // Convert legacy format to stages if needed
        let pipeline = pipeline.to_stages_format();

        let mut context = self.context.clone();
        context.env.extend(pipeline.env.clone());

        if let Some(tx) = &progress_tx {
            let _ = tx.send(ExecutionEvent::PipelineStarted {
                name: pipeline.name.clone(),
            });
        }

        let mut all_results = Vec::new();

        // Execute stages
        for (stage_index, stage) in pipeline.stages.iter().enumerate() {
            if let Some(tx) = &progress_tx {
                let _ = tx.send(ExecutionEvent::StageStarted {
                    stage_name: stage.stage.clone(),
                    stage_index,
                });
            }

            let stage_result = self
                .execute_stage(stage, &context, progress_tx.as_ref())
                .await;

            if let Some(tx) = &progress_tx {
                let _ = tx.send(ExecutionEvent::StageCompleted {
                    result: stage_result.clone(),
                    stage_index,
                });
            }

            // Collect all step results
            for job_result in &stage_result.jobs {
                all_results.extend(job_result.steps.clone());
            }

            // Stop if stage failed
            if stage_result.status == StageStatus::Failed {
                break;
            }
        }

        if let Some(tx) = &progress_tx {
            let failed_count = all_results
                .iter()
                .filter(|r| r.status == StepStatus::Failed)
                .count();
            let _ = tx.send(ExecutionEvent::PipelineCompleted {
                success: failed_count == 0,
                total_steps: all_results.len(),
                failed_steps: failed_count,
            });
        }

        all_results
    }

    async fn execute_stage(
        &self,
        stage: &Stage,
        context: &ExecutionContext,
        progress_tx: Option<&ProgressSender>,
    ) -> StageResult {
        let start = Instant::now();
        let stage_context = context.clone().with_stage(stage.stage.clone());

        // Build dependency graph for jobs
        let job_graph = match build_job_graph(&stage.jobs) {
            Ok(graph) => graph,
            Err(e) => {
                eprintln!("Error building job dependency graph: {}", e);
                return StageResult {
                    stage_name: stage.stage.clone(),
                    status: StageStatus::Failed,
                    jobs: vec![],
                    duration: start.elapsed(),
                };
            }
        };

        // Get execution levels (jobs that can run in parallel)
        let levels = match job_graph.get_execution_levels() {
            Ok(levels) => levels,
            Err(e) => {
                eprintln!("Error resolving job dependencies: {}", e);
                return StageResult {
                    stage_name: stage.stage.clone(),
                    status: StageStatus::Failed,
                    jobs: vec![],
                    duration: start.elapsed(),
                };
            }
        };

        let mut job_results = Vec::new();
        let mut stage_failed = false;

        // Execute jobs level by level (parallel within level, sequential between levels)
        for level in levels {
            let mut handles = Vec::new();

            for &job_index in &level {
                let job = job_graph.get_node(job_index).unwrap().clone();
                let job_context = stage_context.clone().with_job(job.job.clone());
                let tx = progress_tx.cloned();
                let executor = Arc::new(self.clone());

                // Spawn each job in the level
                let handle = tokio::spawn(async move {
                    executor.execute_job(&job, &job_context, tx.as_ref()).await
                });

                handles.push((job_index, handle));
            }

            // Wait for all jobs in this level to complete
            for (job_index, handle) in handles {
                match handle.await {
                    Ok(result) => {
                        if let Some(tx) = progress_tx {
                            let _ = tx.send(ExecutionEvent::JobCompleted {
                                result: result.clone(),
                                job_index,
                            });
                        }

                        if result.status == JobStatus::Failed {
                            stage_failed = true;
                        }
                        job_results.push(result);
                    }
                    Err(e) => {
                        eprintln!("Job execution panicked: {}", e);
                        stage_failed = true;
                    }
                }
            }

            // Stop if any job in the level failed
            if stage_failed {
                break;
            }
        }

        let status = if stage_failed {
            StageStatus::Failed
        } else {
            StageStatus::Success
        };

        StageResult {
            stage_name: stage.stage.clone(),
            status,
            jobs: job_results,
            duration: start.elapsed(),
        }
    }

    async fn execute_job(
        &self,
        job: &Job,
        context: &ExecutionContext,
        progress_tx: Option<&ProgressSender>,
    ) -> JobResult {
        let start = Instant::now();
        let mut job_context = context.clone();
        job_context.env.extend(job.env.clone());

        if let Some(tx) = progress_tx {
            let _ = tx.send(ExecutionEvent::JobStarted {
                job_name: job.job.clone(),
                job_index: 0, // Will be set by caller
            });
        }

        let mut step_results = Vec::new();
        let mut job_failed = false;

        for (index, step) in job.steps.iter().enumerate() {
            if let Some(tx) = progress_tx {
                let _ = tx.send(ExecutionEvent::StepStarted {
                    step_name: step.name.clone(),
                    step_index: index,
                });
            }

            let result = self.execute_step(step, &job_context, progress_tx).await;

            if let Some(tx) = progress_tx {
                let _ = tx.send(ExecutionEvent::StepCompleted {
                    result: result.clone(),
                    step_index: index,
                });
            }

            let should_continue = result.status == StepStatus::Success || step.continue_on_error;
            step_results.push(result);

            if !should_continue {
                job_failed = true;
                break;
            }
        }

        let status = if job_failed {
            JobStatus::Failed
        } else {
            JobStatus::Success
        };

        JobResult {
            job_name: job.job.clone(),
            status,
            steps: step_results,
            duration: start.elapsed(),
        }
    }

    async fn execute_step(
        &self,
        step: &Step,
        context: &ExecutionContext,
        progress_tx: Option<&ProgressSender>,
    ) -> StepResult {
        let start = Instant::now();
        let mut step_context = context.clone();
        step_context.env.extend(step.env.clone());

        let runner = ShellRunner::new(step_context);
        let result = runner.run(step, progress_tx).await;

        StepResult {
            step_name: step.name.clone(),
            status: result.status,
            output: result.output,
            error: result.error,
            duration: start.elapsed(),
            exit_code: result.exit_code,
        }
    }
}

// Make PipelineExecutor cloneable for parallel execution
impl Clone for PipelineExecutor {
    fn clone(&self) -> Self {
        Self {
            context: self.context.clone(),
        }
    }
}
