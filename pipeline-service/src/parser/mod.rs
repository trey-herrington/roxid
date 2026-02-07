// Parser module for Azure DevOps pipelines
// Provides YAML parsing, validation, and template resolution

pub mod azure;
pub mod error;
pub mod models;
pub mod template;

pub use azure::{normalize_pipeline, AzureParser, PipelineValidator};
pub use error::{ParseError, ParseErrorKind, ParseResult, ValidationError};
pub use models::*;
pub use template::{TemplateEngine, TemplateError, TemplateErrorKind};
