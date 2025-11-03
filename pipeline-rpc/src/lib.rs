pub mod error;
pub mod handlers;

pub use error::{RpcError, RpcResult};
pub use handlers::PipelineHandler;

// Re-export types needed by clients
pub use pipeline_service;
pub use pipeline_service::pipeline::{ExecutionEvent, Pipeline, StepStatus};
