// Pipeline Service Library
// Core service for Azure DevOps pipeline parsing and execution

pub mod error;
pub mod execution;
pub mod expression;
pub mod parser;
pub mod runners;
pub mod tasks;
pub mod testing;
pub mod workflow;

// Re-export commonly used types
pub use error::{ServiceError, ServiceResult};

// Re-export parser types
pub use parser::{
    normalize_pipeline, AzureParser, ParseError, ParseErrorKind, ParseResult, Pipeline,
    PipelineValidator, TemplateEngine, TemplateError, TemplateErrorKind, ValidationError,
};

// Re-export expression types
pub use expression::{EvalError, ExpressionContext, ExpressionEngine, ExpressionType};

// Re-export execution types
pub use execution::{
    ExecutionEvent, ExecutionGraph, ExecutionResult, GraphError, JobNode, MatrixExpander,
    MatrixInstance, PipelineExecutor, ProgressSender, RuntimeContext, StageNode,
};

// Re-export runner types
pub use runners::{ContainerRunner, RunnerRegistry, ShellRunner, TaskRunner};

// Re-export task types
pub use tasks::{TaskCache, TaskCacheConfig, TaskManifest};

// Re-export testing types
pub use testing::{
    Assertion, AssertionResult, ReportFormat, TestFileParser, TestReporter, TestResult, TestRunner,
    TestSuiteResult,
};
