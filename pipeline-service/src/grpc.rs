use crate::pipeline::models::{
    Pipeline, Step, StepAction, StepResult, StepStatus,
};
use crate::pipeline::executor::ExecutionEvent;

pub mod proto {
    tonic::include_proto!("pipeline");
}

// Convert from proto to domain models
impl From<proto::Pipeline> for Pipeline {
    fn from(p: proto::Pipeline) -> Self {
        Pipeline {
            name: p.name,
            description: p.description,
            env: p.env,
            steps: p.steps.into_iter().map(Step::from).collect(),
        }
    }
}

impl From<proto::Step> for Step {
    fn from(s: proto::Step) -> Self {
        Step {
            name: s.name,
            action: s.action.map(StepAction::from).unwrap_or(StepAction::Command(String::new())),
            env: s.env,
            continue_on_error: s.continue_on_error,
        }
    }
}

impl From<proto::StepAction> for StepAction {
    fn from(a: proto::StepAction) -> Self {
        match a.action {
            Some(proto::step_action::Action::Command(cmd)) => StepAction::Command(cmd),
            Some(proto::step_action::Action::Shell(shell)) => StepAction::Shell {
                shell: shell.shell,
                script: shell.script,
            },
            None => StepAction::Command(String::new()),
        }
    }
}

// Convert from domain models to proto
impl From<Pipeline> for proto::Pipeline {
    fn from(p: Pipeline) -> Self {
        proto::Pipeline {
            name: p.name,
            description: p.description,
            env: p.env,
            steps: p.steps.into_iter().map(proto::Step::from).collect(),
        }
    }
}

impl From<Step> for proto::Step {
    fn from(s: Step) -> Self {
        proto::Step {
            name: s.name,
            action: Some(proto::StepAction::from(s.action)),
            env: s.env,
            continue_on_error: s.continue_on_error,
        }
    }
}

impl From<StepAction> for proto::StepAction {
    fn from(a: StepAction) -> Self {
        let action = match a {
            StepAction::Command(cmd) => proto::step_action::Action::Command(cmd),
            StepAction::Shell { shell, script } => {
                proto::step_action::Action::Shell(proto::ShellScript { shell, script })
            }
        };
        proto::StepAction {
            action: Some(action),
        }
    }
}

impl From<StepStatus> for proto::StepStatus {
    fn from(s: StepStatus) -> Self {
        match s {
            StepStatus::Pending => proto::StepStatus::Pending,
            StepStatus::Running => proto::StepStatus::Running,
            StepStatus::Success => proto::StepStatus::Success,
            StepStatus::Failed => proto::StepStatus::Failed,
            StepStatus::Skipped => proto::StepStatus::Skipped,
        }
    }
}

impl From<proto::StepStatus> for StepStatus {
    fn from(s: proto::StepStatus) -> Self {
        match s {
            proto::StepStatus::Pending => StepStatus::Pending,
            proto::StepStatus::Running => StepStatus::Running,
            proto::StepStatus::Success => StepStatus::Success,
            proto::StepStatus::Failed => StepStatus::Failed,
            proto::StepStatus::Skipped => StepStatus::Skipped,
        }
    }
}

impl From<StepResult> for proto::StepResult {
    fn from(r: StepResult) -> Self {
        proto::StepResult {
            step_name: r.step_name,
            status: i32::from(proto::StepStatus::from(r.status)),
            output: r.output,
            error: r.error,
            duration_ms: r.duration.as_millis() as u64,
            exit_code: r.exit_code,
        }
    }
}

impl From<ExecutionEvent> for proto::ExecutionEvent {
    fn from(e: ExecutionEvent) -> Self {
        let event = match e {
            ExecutionEvent::PipelineStarted { name } => {
                proto::execution_event::Event::PipelineStarted(proto::PipelineStarted { name })
            }
            ExecutionEvent::StepStarted {
                step_name,
                step_index,
            } => proto::execution_event::Event::StepStarted(proto::StepStarted {
                step_name,
                step_index: step_index as u32,
            }),
            ExecutionEvent::StepOutput { step_name, output } => {
                proto::execution_event::Event::StepOutput(proto::StepOutput { step_name, output })
            }
            ExecutionEvent::StepCompleted {
                result,
                step_index,
            } => proto::execution_event::Event::StepCompleted(proto::StepCompleted {
                result: Some(proto::StepResult::from(result)),
                step_index: step_index as u32,
            }),
            ExecutionEvent::PipelineCompleted {
                success,
                total_steps,
                failed_steps,
            } => proto::execution_event::Event::PipelineCompleted(proto::PipelineCompleted {
                success,
                total_steps: total_steps as u32,
                failed_steps: failed_steps as u32,
            }),
        };
        proto::ExecutionEvent { event: Some(event) }
    }
}
