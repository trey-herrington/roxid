use crate::pipeline::models::{ExecutionContext, Pipeline, Step, StepResult, StepStatus};
use crate::pipeline::runners::shell::ShellRunner;
use std::time::Instant;
use tokio::sync::mpsc;

pub type ProgressSender = mpsc::UnboundedSender<ExecutionEvent>;
pub type ProgressReceiver = mpsc::UnboundedReceiver<ExecutionEvent>;

#[derive(Debug, Clone)]
pub enum ExecutionEvent {
    PipelineStarted { name: String },
    StepStarted { step_name: String, step_index: usize },
    StepOutput { step_name: String, output: String },
    StepCompleted { result: StepResult, step_index: usize },
    PipelineCompleted { success: bool, total_steps: usize, failed_steps: usize },
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
        let mut results = Vec::new();
        let mut context = self.context.clone();
        
        context.env.extend(pipeline.env.clone());

        if let Some(tx) = &progress_tx {
            let _ = tx.send(ExecutionEvent::PipelineStarted {
                name: pipeline.name.clone(),
            });
        }

        for (index, step) in pipeline.steps.iter().enumerate() {
            if let Some(tx) = &progress_tx {
                let _ = tx.send(ExecutionEvent::StepStarted {
                    step_name: step.name.clone(),
                    step_index: index,
                });
            }

            let result = self.execute_step(step, &context, progress_tx.as_ref()).await;

            if let Some(tx) = &progress_tx {
                let _ = tx.send(ExecutionEvent::StepCompleted {
                    result: result.clone(),
                    step_index: index,
                });
            }

            let should_continue = result.status == StepStatus::Success || step.continue_on_error;
            results.push(result);

            if !should_continue {
                break;
            }
        }

        if let Some(tx) = &progress_tx {
            let failed_count = results.iter().filter(|r| r.status == StepStatus::Failed).count();
            let _ = tx.send(ExecutionEvent::PipelineCompleted {
                success: failed_count == 0,
                total_steps: results.len(),
                failed_steps: failed_count,
            });
        }

        results
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
