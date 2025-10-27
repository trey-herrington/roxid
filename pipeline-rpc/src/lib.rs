pub mod api;
pub mod error;
pub mod handlers;

pub use api::RpcServer;
pub use error::{RpcError, RpcResult};
pub use handlers::{PipelineHandler, UserHandler};

// Re-export types needed by clients
pub use pipeline_service::pipeline::{ExecutionEvent, Pipeline, StepStatus};
pub use pipeline_service;

