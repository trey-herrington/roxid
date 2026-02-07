// Expression Engine Module
// Full Azure DevOps expression support: ${{ }}, $[ ], and $(var)

pub mod evaluator;
pub mod functions;
pub mod lexer;
pub mod parser;

pub use evaluator::{
    AgentContext, DependenciesContext, EvalError, Evaluator, ExpressionContext, ExpressionEngine,
    JobContext, JobDependency, JobStatusContext, PipelineContext, PipelineResourceContext,
    RepositoryResourceContext, ResourcesContext, StageContext, StageDependency, StepContext,
    StepStatusContext,
};
pub use functions::BuiltinFunctions;
pub use lexer::{extract_expressions, ExpressionType, LexError, Lexer, Token};
pub use parser::{BinaryOp, Expr, ExprParser, ParseExprError, Reference, ReferencePart, UnaryOp};
