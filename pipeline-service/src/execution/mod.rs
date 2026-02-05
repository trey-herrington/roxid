// Execution Engine Module
// Handles DAG construction, execution orchestration, and matrix expansion

pub mod context;
pub mod events;
pub mod executor;
pub mod graph;
pub mod matrix;

// Re-export key types
pub use context::RuntimeContext;
pub use events::{ExecutionEvent, ProgressSender};
pub use executor::{ExecutionResult, PipelineExecutor};
pub use graph::{ExecutionGraph, GraphError, JobNode, StageNode};
pub use matrix::{MatrixExpander, MatrixInstance};
