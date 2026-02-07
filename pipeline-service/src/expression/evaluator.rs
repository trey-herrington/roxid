// Expression Engine Evaluator
// Evaluates AST expressions with context (variables, parameters, etc.)

use crate::expression::functions::BuiltinFunctions;
use crate::expression::parser::{BinaryOp, Expr, Reference, ReferencePart, UnaryOp};
use crate::parser::models::Value;

use std::collections::HashMap;
use std::fmt;

/// Evaluation error
#[derive(Debug, Clone)]
pub struct EvalError {
    pub message: String,
}

impl fmt::Display for EvalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "evaluation error: {}", self.message)
    }
}

impl std::error::Error for EvalError {}

impl EvalError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

/// Context for expression evaluation
#[derive(Debug, Clone, Default)]
pub struct ExpressionContext {
    /// Pipeline variables
    pub variables: HashMap<String, Value>,

    /// Pipeline parameters
    pub parameters: HashMap<String, Value>,

    /// Pipeline context (pipeline.*, Build.*, etc.)
    pub pipeline: PipelineContext,

    /// Current stage context
    pub stage: Option<StageContext>,

    /// Current job context
    pub job: Option<JobContext>,

    /// Step outputs by step name
    pub steps: HashMap<String, StepContext>,

    /// Stage/job dependencies output
    pub dependencies: DependenciesContext,

    /// Environment variables
    pub env: HashMap<String, Value>,

    /// Resources context
    pub resources: ResourcesContext,
}

#[derive(Debug, Clone, Default)]
pub struct PipelineContext {
    /// Pipeline name
    pub name: Option<String>,
    /// Pipeline workspace
    pub workspace: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct StageContext {
    /// Stage name
    pub name: String,
    /// Stage display name
    pub display_name: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct JobContext {
    /// Job name
    pub name: String,
    /// Job display name
    pub display_name: Option<String>,
    /// Agent context
    pub agent: AgentContext,
    /// Job status
    pub status: JobStatusContext,
}

#[derive(Debug, Clone, Default)]
pub struct AgentContext {
    pub name: Option<String>,
    pub os: Option<String>,
    pub os_architecture: Option<String>,
    pub temp_directory: Option<String>,
    pub tools_directory: Option<String>,
    pub work_folder: Option<String>,
    pub build_directory: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct JobStatusContext {
    pub succeeded: bool,
    pub failed: bool,
    pub canceled: bool,
}

#[derive(Debug, Clone, Default)]
pub struct StepContext {
    /// Step outputs
    pub outputs: HashMap<String, Value>,
    /// Step status
    pub status: StepStatusContext,
}

#[derive(Debug, Clone, Default)]
pub struct StepStatusContext {
    pub succeeded: bool,
    pub failed: bool,
    pub skipped: bool,
}

#[derive(Debug, Clone, Default)]
pub struct DependenciesContext {
    /// Stage dependencies: dependencies.stageName.outputs.jobName.varName
    pub stages: HashMap<String, StageDependency>,
    /// Job dependencies: dependencies.jobName.outputs.varName
    pub jobs: HashMap<String, JobDependency>,
}

#[derive(Debug, Clone, Default)]
pub struct StageDependency {
    pub outputs: HashMap<String, HashMap<String, Value>>,
    pub result: String,
}

#[derive(Debug, Clone, Default)]
pub struct JobDependency {
    pub outputs: HashMap<String, Value>,
    pub result: String,
}

#[derive(Debug, Clone, Default)]
pub struct ResourcesContext {
    pub pipelines: HashMap<String, PipelineResourceContext>,
    pub repositories: HashMap<String, RepositoryResourceContext>,
}

#[derive(Debug, Clone, Default)]
pub struct PipelineResourceContext {
    pub pipeline_id: Option<String>,
    pub run_name: Option<String>,
    pub run_id: Option<String>,
    pub run_uri: Option<String>,
    pub source_branch: Option<String>,
    pub source_commit: Option<String>,
    pub source_provider: Option<String>,
    pub requested_for: Option<String>,
    pub requested_for_id: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct RepositoryResourceContext {
    pub name: Option<String>,
    pub repo_type: Option<String>,
    pub ref_name: Option<String>,
    pub version: Option<String>,
}

/// Expression evaluator
pub struct Evaluator<'a> {
    context: &'a ExpressionContext,
    functions: BuiltinFunctions,
}

impl<'a> Evaluator<'a> {
    pub fn new(context: &'a ExpressionContext) -> Self {
        Self {
            context,
            functions: BuiltinFunctions::new(),
        }
    }

    /// Evaluate an expression
    pub fn eval(&self, expr: &Expr) -> Result<Value, EvalError> {
        match expr {
            Expr::Null => Ok(Value::Null),
            Expr::Bool(b) => Ok(Value::Bool(*b)),
            Expr::Number(n) => Ok(Value::Number(*n)),
            Expr::String(s) => Ok(Value::String(s.clone())),

            Expr::Reference(reference) => self.eval_reference(reference),

            Expr::FunctionCall { name, args } => self.eval_function(name, args),

            Expr::Index { object, index } => {
                let obj = self.eval(object)?;
                let idx = self.eval(index)?;
                self.eval_index(&obj, &idx)
            }

            Expr::Member { object, property } => {
                let obj = self.eval(object)?;
                self.eval_member(&obj, property)
            }

            Expr::Unary { op, expr } => {
                let val = self.eval(expr)?;
                self.eval_unary(*op, &val)
            }

            Expr::Binary { op, left, right } => {
                // Short-circuit evaluation for && and ||
                match op {
                    BinaryOp::And => {
                        let left_val = self.eval(left)?;
                        if !left_val.is_truthy() {
                            return Ok(Value::Bool(false));
                        }
                        let right_val = self.eval(right)?;
                        Ok(Value::Bool(right_val.is_truthy()))
                    }
                    BinaryOp::Or => {
                        let left_val = self.eval(left)?;
                        if left_val.is_truthy() {
                            return Ok(Value::Bool(true));
                        }
                        let right_val = self.eval(right)?;
                        Ok(Value::Bool(right_val.is_truthy()))
                    }
                    _ => {
                        let left_val = self.eval(left)?;
                        let right_val = self.eval(right)?;
                        self.eval_binary(*op, &left_val, &right_val)
                    }
                }
            }

            Expr::Ternary {
                condition,
                then_expr,
                else_expr,
            } => {
                let cond = self.eval(condition)?;
                if cond.is_truthy() {
                    self.eval(then_expr)
                } else {
                    self.eval(else_expr)
                }
            }

            Expr::Array(items) => {
                let values: Result<Vec<Value>, EvalError> =
                    items.iter().map(|e| self.eval(e)).collect();
                Ok(Value::Array(values?))
            }

            Expr::Object(pairs) => {
                let mut map = HashMap::new();
                for (key, value_expr) in pairs {
                    map.insert(key.clone(), self.eval(value_expr)?);
                }
                Ok(Value::Object(map))
            }
        }
    }

    fn eval_reference(&self, reference: &Reference) -> Result<Value, EvalError> {
        let mut current: Option<Value> = None;

        for (i, part) in reference.parts.iter().enumerate() {
            match part {
                ReferencePart::Property(name) => {
                    if i == 0 {
                        // Top-level context lookup
                        current = Some(self.lookup_context(name)?);
                    } else {
                        let obj = current.ok_or_else(|| EvalError::new("invalid reference"))?;
                        current = Some(self.eval_member(&obj, name)?);
                    }
                }
                ReferencePart::Index(index_expr) => {
                    let obj = current.ok_or_else(|| EvalError::new("invalid index access"))?;
                    let index = self.eval(index_expr)?;
                    current = Some(self.eval_index(&obj, &index)?);
                }
            }
        }

        current.ok_or_else(|| EvalError::new("empty reference"))
    }

    fn lookup_context(&self, name: &str) -> Result<Value, EvalError> {
        // Check for direct parameter match first (iteration variables from ${{ each }}
        // shadow built-in context names like 'env')
        if let Some(value) = self.context.parameters.get(name) {
            // Only shadow built-in contexts for non-context names, OR when the
            // parameter name matches a built-in context name (iteration variable).
            // The full context objects "variables" and "parameters" should still
            // be accessible via their full paths, so only shadow them if the
            // parameter name is NOT one of the primary context prefixes that
            // users access with dot-notation (variables.x, parameters.x).
            let is_primary_context =
                matches!(name.to_lowercase().as_str(), "variables" | "parameters");
            if !is_primary_context {
                return Ok(value.clone());
            }
        }

        match name.to_lowercase().as_str() {
            "variables" => Ok(Value::Object(
                self.context
                    .variables
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect(),
            )),
            "parameters" => Ok(Value::Object(
                self.context
                    .parameters
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect(),
            )),
            "pipeline" => self.pipeline_to_value(),
            "stage" => self.stage_to_value(),
            "job" => self.job_to_value(),
            "steps" => Ok(Value::Object(
                self.context
                    .steps
                    .iter()
                    .map(|(k, v)| (k.clone(), self.step_context_to_value(v)))
                    .collect(),
            )),
            "dependencies" => self.dependencies_to_value(),
            "env" => Ok(Value::Object(
                self.context
                    .env
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect(),
            )),
            "resources" => self.resources_to_value(),

            // Direct variable lookup (for $(varName) compatibility)
            _ => {
                // First try variables
                if let Some(value) = self.context.variables.get(name) {
                    return Ok(value.clone());
                }
                // Then try parameters
                if let Some(value) = self.context.parameters.get(name) {
                    return Ok(value.clone());
                }
                // Return empty string for undefined (Azure DevOps behavior)
                Ok(Value::String(String::new()))
            }
        }
    }

    fn pipeline_to_value(&self) -> Result<Value, EvalError> {
        let mut map = HashMap::new();
        if let Some(name) = &self.context.pipeline.name {
            map.insert("name".to_string(), Value::String(name.clone()));
        }
        if let Some(workspace) = &self.context.pipeline.workspace {
            map.insert("workspace".to_string(), Value::String(workspace.clone()));
        }
        Ok(Value::Object(map))
    }

    fn stage_to_value(&self) -> Result<Value, EvalError> {
        let Some(stage) = &self.context.stage else {
            return Ok(Value::Null);
        };

        let mut map = HashMap::new();
        map.insert("name".to_string(), Value::String(stage.name.clone()));
        if let Some(display_name) = &stage.display_name {
            map.insert(
                "displayName".to_string(),
                Value::String(display_name.clone()),
            );
        }
        Ok(Value::Object(map))
    }

    fn job_to_value(&self) -> Result<Value, EvalError> {
        let Some(job) = &self.context.job else {
            return Ok(Value::Null);
        };

        let mut map = HashMap::new();
        map.insert("name".to_string(), Value::String(job.name.clone()));
        if let Some(display_name) = &job.display_name {
            map.insert(
                "displayName".to_string(),
                Value::String(display_name.clone()),
            );
        }

        // Agent sub-object
        let mut agent = HashMap::new();
        if let Some(name) = &job.agent.name {
            agent.insert("name".to_string(), Value::String(name.clone()));
        }
        if let Some(os) = &job.agent.os {
            agent.insert("os".to_string(), Value::String(os.clone()));
        }
        map.insert("agent".to_string(), Value::Object(agent));

        Ok(Value::Object(map))
    }

    fn step_context_to_value(&self, step: &StepContext) -> Value {
        let mut map = HashMap::new();

        // Outputs
        let outputs: HashMap<String, Value> = step
            .outputs
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        map.insert("outputs".to_string(), Value::Object(outputs));

        Value::Object(map)
    }

    fn dependencies_to_value(&self) -> Result<Value, EvalError> {
        let mut map = HashMap::new();

        // Stage dependencies
        for (name, dep) in &self.context.dependencies.stages {
            let mut stage_map = HashMap::new();
            stage_map.insert("result".to_string(), Value::String(dep.result.clone()));

            let mut outputs = HashMap::new();
            for (job_name, job_outputs) in &dep.outputs {
                outputs.insert(
                    job_name.clone(),
                    Value::Object(
                        job_outputs
                            .iter()
                            .map(|(k, v)| (k.clone(), v.clone()))
                            .collect(),
                    ),
                );
            }
            stage_map.insert("outputs".to_string(), Value::Object(outputs));

            map.insert(name.clone(), Value::Object(stage_map));
        }

        // Job dependencies
        for (name, dep) in &self.context.dependencies.jobs {
            let mut job_map = HashMap::new();
            job_map.insert("result".to_string(), Value::String(dep.result.clone()));
            job_map.insert(
                "outputs".to_string(),
                Value::Object(
                    dep.outputs
                        .iter()
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect(),
                ),
            );
            map.insert(name.clone(), Value::Object(job_map));
        }

        Ok(Value::Object(map))
    }

    fn resources_to_value(&self) -> Result<Value, EvalError> {
        let mut map = HashMap::new();

        // Pipelines
        let mut pipelines = HashMap::new();
        for (name, resource) in &self.context.resources.pipelines {
            let mut resource_map = HashMap::new();
            if let Some(id) = &resource.pipeline_id {
                resource_map.insert("pipelineID".to_string(), Value::String(id.clone()));
            }
            if let Some(name) = &resource.run_name {
                resource_map.insert("runName".to_string(), Value::String(name.clone()));
            }
            pipelines.insert(name.clone(), Value::Object(resource_map));
        }
        map.insert("pipelines".to_string(), Value::Object(pipelines));

        // Repositories
        let mut repos = HashMap::new();
        for (name, resource) in &self.context.resources.repositories {
            let mut resource_map = HashMap::new();
            if let Some(n) = &resource.name {
                resource_map.insert("name".to_string(), Value::String(n.clone()));
            }
            if let Some(t) = &resource.repo_type {
                resource_map.insert("type".to_string(), Value::String(t.clone()));
            }
            repos.insert(name.clone(), Value::Object(resource_map));
        }
        map.insert("repositories".to_string(), Value::Object(repos));

        Ok(Value::Object(map))
    }

    fn eval_function(&self, name: &str, args: &[Expr]) -> Result<Value, EvalError> {
        let evaluated_args: Result<Vec<Value>, EvalError> =
            args.iter().map(|a| self.eval(a)).collect();
        self.functions.call(name, evaluated_args?, self.context)
    }

    fn eval_index(&self, object: &Value, index: &Value) -> Result<Value, EvalError> {
        match (object, index) {
            (Value::Array(arr), Value::Number(n)) => {
                let i = *n as usize;
                arr.get(i)
                    .cloned()
                    .ok_or_else(|| EvalError::new(format!("array index {} out of bounds", i)))
            }
            (Value::Object(map), Value::String(key)) => {
                Ok(map.get(key).cloned().unwrap_or(Value::Null))
            }
            (Value::Object(map), Value::Number(n)) => {
                let key = n.to_string();
                Ok(map.get(&key).cloned().unwrap_or(Value::Null))
            }
            (Value::String(s), Value::Number(n)) => {
                let i = *n as usize;
                s.chars()
                    .nth(i)
                    .map(|c| Value::String(c.to_string()))
                    .ok_or_else(|| EvalError::new(format!("string index {} out of bounds", i)))
            }
            _ => Err(EvalError::new(format!(
                "cannot index {:?} with {:?}",
                object, index
            ))),
        }
    }

    fn eval_member(&self, object: &Value, property: &str) -> Result<Value, EvalError> {
        match object {
            Value::Object(map) => Ok(map.get(property).cloned().unwrap_or(Value::Null)),
            Value::Array(arr) if property == "length" => Ok(Value::Number(arr.len() as f64)),
            Value::String(s) if property == "length" => Ok(Value::Number(s.len() as f64)),
            _ => Err(EvalError::new(format!(
                "cannot access property '{}' on {:?}",
                property, object
            ))),
        }
    }

    fn eval_unary(&self, op: UnaryOp, value: &Value) -> Result<Value, EvalError> {
        match op {
            UnaryOp::Not => Ok(Value::Bool(!value.is_truthy())),
            UnaryOp::Neg => match value {
                Value::Number(n) => Ok(Value::Number(-n)),
                _ => Err(EvalError::new("cannot negate non-number")),
            },
        }
    }

    fn eval_binary(&self, op: BinaryOp, left: &Value, right: &Value) -> Result<Value, EvalError> {
        match op {
            // Arithmetic
            BinaryOp::Add => self.eval_add(left, right),
            BinaryOp::Sub => self.eval_numeric_op(left, right, |a, b| a - b),
            BinaryOp::Mul => self.eval_numeric_op(left, right, |a, b| a * b),
            BinaryOp::Div => self.eval_numeric_op(left, right, |a, b| a / b),
            BinaryOp::Mod => self.eval_numeric_op(left, right, |a, b| a % b),

            // Comparison
            BinaryOp::Eq => Ok(Value::Bool(self.values_equal(left, right))),
            BinaryOp::Ne => Ok(Value::Bool(!self.values_equal(left, right))),
            BinaryOp::Lt => self.eval_comparison(left, right, |a, b| a < b),
            BinaryOp::Le => self.eval_comparison(left, right, |a, b| a <= b),
            BinaryOp::Gt => self.eval_comparison(left, right, |a, b| a > b),
            BinaryOp::Ge => self.eval_comparison(left, right, |a, b| a >= b),

            // Logical (handled in eval() for short-circuit)
            BinaryOp::And | BinaryOp::Or => unreachable!("handled in eval()"),
        }
    }

    fn eval_add(&self, left: &Value, right: &Value) -> Result<Value, EvalError> {
        match (left, right) {
            (Value::Number(a), Value::Number(b)) => Ok(Value::Number(a + b)),
            (Value::String(a), Value::String(b)) => Ok(Value::String(format!("{}{}", a, b))),
            (Value::String(a), b) => Ok(Value::String(format!("{}{}", a, b.as_string()))),
            (a, Value::String(b)) => Ok(Value::String(format!("{}{}", a.as_string(), b))),
            _ => Err(EvalError::new("cannot add these types")),
        }
    }

    fn eval_numeric_op<F>(&self, left: &Value, right: &Value, op: F) -> Result<Value, EvalError>
    where
        F: FnOnce(f64, f64) -> f64,
    {
        let a = left
            .as_number()
            .ok_or_else(|| EvalError::new("left operand is not a number"))?;
        let b = right
            .as_number()
            .ok_or_else(|| EvalError::new("right operand is not a number"))?;
        Ok(Value::Number(op(a, b)))
    }

    fn eval_comparison<F>(&self, left: &Value, right: &Value, op: F) -> Result<Value, EvalError>
    where
        F: FnOnce(f64, f64) -> bool,
    {
        let a = left
            .as_number()
            .ok_or_else(|| EvalError::new("left operand is not comparable"))?;
        let b = right
            .as_number()
            .ok_or_else(|| EvalError::new("right operand is not comparable"))?;
        Ok(Value::Bool(op(a, b)))
    }

    fn values_equal(&self, left: &Value, right: &Value) -> bool {
        match (left, right) {
            (Value::Null, Value::Null) => true,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Number(a), Value::Number(b)) => (a - b).abs() < f64::EPSILON,
            (Value::String(a), Value::String(b)) => a.to_lowercase() == b.to_lowercase(),
            // Coerce for comparison
            (Value::Number(a), Value::String(b)) | (Value::String(b), Value::Number(a)) => b
                .parse::<f64>()
                .map(|n| (a - n).abs() < f64::EPSILON)
                .unwrap_or(false),
            (Value::Bool(a), Value::String(b)) | (Value::String(b), Value::Bool(a)) => {
                let b_lower = b.to_lowercase();
                (*a && b_lower == "true") || (!*a && b_lower == "false")
            }
            _ => false,
        }
    }
}

/// High-level expression engine
pub struct ExpressionEngine {
    context: ExpressionContext,
}

impl ExpressionEngine {
    pub fn new(context: ExpressionContext) -> Self {
        Self { context }
    }

    /// Evaluate a compile-time expression: ${{ expression }}
    pub fn evaluate_compile_time(&self, expr: &str) -> Result<Value, EvalError> {
        use crate::expression::parser::ExprParser;

        let ast = ExprParser::parse_str(expr)
            .map_err(|e| EvalError::new(format!("parse error: {}", e)))?;

        let evaluator = Evaluator::new(&self.context);
        evaluator.eval(&ast)
    }

    /// Evaluate a runtime expression: $[ expression ]
    pub fn evaluate_runtime(&self, expr: &str) -> Result<Value, EvalError> {
        // Runtime expressions have the same syntax as compile-time
        self.evaluate_compile_time(expr)
    }

    /// Substitute macro variables: $(variableName)
    pub fn substitute_macros(&self, text: &str) -> Result<String, EvalError> {
        use crate::expression::lexer::{extract_expressions, ExpressionType};

        let expressions = extract_expressions(text);
        let mut result = String::new();

        for expr in expressions {
            match expr {
                ExpressionType::Text(s) => result.push_str(&s),
                ExpressionType::Macro(var_path) => {
                    let value = self.resolve_variable_path(&var_path)?;
                    result.push_str(&value.as_string());
                }
                ExpressionType::CompileTime(expr) => {
                    let value = self.evaluate_compile_time(&expr)?;
                    result.push_str(&value.as_string());
                }
                ExpressionType::Runtime(expr) => {
                    let value = self.evaluate_runtime(&expr)?;
                    result.push_str(&value.as_string());
                }
            }
        }

        Ok(result)
    }

    fn resolve_variable_path(&self, path: &str) -> Result<Value, EvalError> {
        // Handle dotted paths like Build.SourceBranch or simple names like foo
        let parts: Vec<&str> = path.split('.').collect();

        if parts.len() == 1 {
            // Simple variable lookup
            if let Some(value) = self.context.variables.get(parts[0]) {
                return Ok(value.clone());
            }
            if let Some(value) = self.context.parameters.get(parts[0]) {
                return Ok(value.clone());
            }
            // Return empty string for undefined variables
            return Ok(Value::String(String::new()));
        }

        // Handle prefixed lookups like variables.foo
        let prefix = parts[0].to_lowercase();
        let rest = &parts[1..];

        match prefix.as_str() {
            "variables" => {
                let var_name = rest.join(".");
                Ok(self
                    .context
                    .variables
                    .get(&var_name)
                    .cloned()
                    .unwrap_or(Value::String(String::new())))
            }
            "parameters" => {
                let param_name = rest.join(".");
                Ok(self
                    .context
                    .parameters
                    .get(&param_name)
                    .cloned()
                    .unwrap_or(Value::Null))
            }
            "env" => {
                let env_name = rest.join(".");
                Ok(self
                    .context
                    .env
                    .get(&env_name)
                    .cloned()
                    .unwrap_or(Value::String(String::new())))
            }
            _ => {
                // Try as a full dotted variable name (e.g., Build.SourceBranch)
                if let Some(value) = self.context.variables.get(path) {
                    return Ok(value.clone());
                }
                Ok(Value::String(String::new()))
            }
        }
    }

    /// Get the context for modification
    pub fn context_mut(&mut self) -> &mut ExpressionContext {
        &mut self.context
    }

    /// Get the context
    pub fn context(&self) -> &ExpressionContext {
        &self.context
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_context() -> ExpressionContext {
        let mut ctx = ExpressionContext::default();
        ctx.variables
            .insert("foo".to_string(), Value::String("bar".to_string()));
        ctx.variables.insert("num".to_string(), Value::Number(42.0));
        ctx.variables.insert(
            "Build.SourceBranch".to_string(),
            Value::String("refs/heads/main".to_string()),
        );
        ctx.parameters
            .insert("config".to_string(), Value::String("Release".to_string()));
        ctx
    }

    #[test]
    fn test_eval_literals() {
        let engine = ExpressionEngine::new(ExpressionContext::default());

        assert_eq!(engine.evaluate_compile_time("null").unwrap(), Value::Null);
        assert_eq!(
            engine.evaluate_compile_time("true").unwrap(),
            Value::Bool(true)
        );
        assert_eq!(
            engine.evaluate_compile_time("42").unwrap(),
            Value::Number(42.0)
        );
        assert_eq!(
            engine.evaluate_compile_time("'hello'").unwrap(),
            Value::String("hello".to_string())
        );
    }

    #[test]
    fn test_eval_variable_reference() {
        let engine = ExpressionEngine::new(make_context());

        assert_eq!(
            engine.evaluate_compile_time("variables.foo").unwrap(),
            Value::String("bar".to_string())
        );
        assert_eq!(
            engine.evaluate_compile_time("variables['foo']").unwrap(),
            Value::String("bar".to_string())
        );
    }

    #[test]
    fn test_eval_parameter_reference() {
        let engine = ExpressionEngine::new(make_context());

        assert_eq!(
            engine.evaluate_compile_time("parameters.config").unwrap(),
            Value::String("Release".to_string())
        );
    }

    #[test]
    fn test_eval_comparison() {
        let engine = ExpressionEngine::new(make_context());

        assert_eq!(
            engine
                .evaluate_compile_time("variables.foo == 'bar'")
                .unwrap(),
            Value::Bool(true)
        );
        assert_eq!(
            engine.evaluate_compile_time("variables.num > 40").unwrap(),
            Value::Bool(true)
        );
    }

    #[test]
    fn test_eval_logical() {
        let engine = ExpressionEngine::new(make_context());

        assert_eq!(
            engine.evaluate_compile_time("true && true").unwrap(),
            Value::Bool(true)
        );
        assert_eq!(
            engine.evaluate_compile_time("true && false").unwrap(),
            Value::Bool(false)
        );
        assert_eq!(
            engine.evaluate_compile_time("false || true").unwrap(),
            Value::Bool(true)
        );
        assert_eq!(
            engine.evaluate_compile_time("!false").unwrap(),
            Value::Bool(true)
        );
    }

    #[test]
    fn test_eval_ternary() {
        let engine = ExpressionEngine::new(make_context());

        assert_eq!(
            engine.evaluate_compile_time("true ? 'yes' : 'no'").unwrap(),
            Value::String("yes".to_string())
        );
        assert_eq!(
            engine
                .evaluate_compile_time("false ? 'yes' : 'no'")
                .unwrap(),
            Value::String("no".to_string())
        );
    }

    #[test]
    fn test_substitute_macros() {
        let engine = ExpressionEngine::new(make_context());

        assert_eq!(
            engine.substitute_macros("Value: $(foo)").unwrap(),
            "Value: bar"
        );
        assert_eq!(
            engine
                .substitute_macros("Branch: $(Build.SourceBranch)")
                .unwrap(),
            "Branch: refs/heads/main"
        );
    }

    #[test]
    fn test_substitute_mixed() {
        let engine = ExpressionEngine::new(make_context());

        assert_eq!(
            engine
                .substitute_macros("Config: ${{ parameters.config }} on $(Build.SourceBranch)")
                .unwrap(),
            "Config: Release on refs/heads/main"
        );
    }

    #[test]
    fn test_undefined_variable() {
        let engine = ExpressionEngine::new(make_context());

        // Undefined variables return empty string
        assert_eq!(engine.substitute_macros("$(undefined)").unwrap(), "");
    }
}
