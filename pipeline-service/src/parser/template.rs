// Template Resolution Engine for Azure DevOps Pipelines
// Resolves template references, expands parameters, handles extends,
// and supports ${{ each }} and ${{ if }} template expressions.

use crate::expression::{ExpressionContext, ExpressionEngine};
use crate::parser::azure::AzureParser;
use crate::parser::error::{ParseError, ParseErrorKind, ParseResult};
use crate::parser::models::*;

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Maximum template inclusion depth to prevent infinite recursion
const MAX_TEMPLATE_DEPTH: usize = 50;

/// Error specific to template resolution
#[derive(Debug, Clone)]
pub struct TemplateError {
    pub message: String,
    pub template_path: Option<String>,
    pub kind: TemplateErrorKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TemplateErrorKind {
    /// Template file not found
    NotFound,
    /// Circular template inclusion
    CircularReference,
    /// Maximum depth exceeded
    MaxDepthExceeded,
    /// Invalid parameter
    InvalidParameter,
    /// Parameter type mismatch
    TypeMismatch,
    /// Required parameter missing
    MissingParameter,
    /// Parse error in template file
    ParseError,
    /// Expression evaluation error in template
    ExpressionError,
}

impl std::fmt::Display for TemplateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(path) = &self.template_path {
            write!(f, "template error in '{}': {}", path, self.message)
        } else {
            write!(f, "template error: {}", self.message)
        }
    }
}

impl std::error::Error for TemplateError {}

impl TemplateError {
    pub fn new(message: impl Into<String>, kind: TemplateErrorKind) -> Self {
        Self {
            message: message.into(),
            template_path: None,
            kind,
        }
    }

    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.template_path = Some(path.into());
        self
    }

    pub fn to_parse_error(&self) -> ParseError {
        ParseError::new(self.to_string(), 0, 0).with_kind(ParseErrorKind::TemplateError)
    }
}

/// Content types that a template file can contain
#[derive(Debug, Clone)]
pub enum TemplateContent {
    /// Template containing steps
    Steps(Vec<Step>),
    /// Template containing jobs
    Jobs(Vec<Job>),
    /// Template containing stages
    Stages(Vec<Stage>),
    /// Template containing variables
    Variables(Vec<Variable>),
    /// Full pipeline template (for extends)
    Pipeline(Box<Pipeline>),
}

/// Raw template content before deserialization (preserves ${{ if }}, ${{ each }} directives)
#[derive(Debug, Clone)]
enum RawTemplateContent {
    Steps(serde_yaml::Value),
    Jobs(serde_yaml::Value),
    Stages(serde_yaml::Value),
    Variables(serde_yaml::Value),
    Pipeline(String),
}

/// A parsed template file with its declared parameters
#[derive(Debug, Clone)]
pub struct TemplateFile {
    /// Template parameters declaration
    pub parameters: Vec<Parameter>,
    /// The template content
    pub content: TemplateContent,
}

/// A raw template file with its declared parameters (pre-expression processing)
#[derive(Debug, Clone)]
struct RawTemplateFile {
    /// Template parameters declaration
    parameters: Vec<Parameter>,
    /// The raw template content (before ${{ if }}/${{ each }} processing)
    content: RawTemplateContent,
}

/// Parsed template directive from a YAML key
#[derive(Debug, Clone)]
enum TemplateDirective {
    /// `${{ if <condition> }}`
    If(String),
    /// `${{ elseif <condition> }}` or `${{ else if <condition> }}`
    ElseIf(String),
    /// `${{ else }}`
    Else,
    /// `${{ each <var> in <collection> }}`
    Each(String, String),
}

/// Template resolution engine
///
/// Resolves template references in Azure DevOps pipelines by:
/// 1. Loading template files relative to the repository root
/// 2. Validating and substituting parameters
/// 3. Expanding template expressions (${{ each }}, ${{ if }})
/// 4. Handling extends (pipeline inheritance)
/// 5. Detecting circular references
pub struct TemplateEngine {
    /// Repository root directory for resolving template paths
    repo_root: PathBuf,
    /// Resource repository paths for cross-repo template references
    resource_repos: HashMap<String, PathBuf>,
    /// Track included templates for cycle detection
    include_stack: Vec<String>,
}

impl TemplateEngine {
    /// Create a new template engine
    pub fn new(repo_root: PathBuf) -> Self {
        Self {
            repo_root,
            resource_repos: HashMap::new(),
            include_stack: Vec::new(),
        }
    }

    /// Add a resource repository path for cross-repo template references
    pub fn with_resource_repo(mut self, name: String, path: PathBuf) -> Self {
        self.resource_repos.insert(name, path);
        self
    }

    /// Resolve all templates in a pipeline, returning a fully expanded pipeline
    /// with no template references remaining.
    pub fn resolve_pipeline(&mut self, pipeline: Pipeline) -> ParseResult<Pipeline> {
        let mut resolved = pipeline;

        // 1. Handle extends template (pipeline inheritance)
        if let Some(extends) = resolved.extends.take() {
            resolved = self.resolve_extends(&extends, resolved)?;
        }

        // 2. Resolve variable templates
        resolved.variables = self.resolve_variable_templates(&resolved.variables)?;

        // 3. Resolve stage templates
        resolved.stages = self.resolve_stage_templates(&resolved.stages)?;

        // 4. Resolve job templates (for pipeline-level jobs)
        resolved.jobs = self.resolve_job_templates(&resolved.jobs)?;

        // 5. Resolve step templates (for pipeline-level steps)
        resolved.steps = self.resolve_step_templates(&resolved.steps)?;

        Ok(resolved)
    }

    // =========================================================================
    // Extends Resolution
    // =========================================================================

    /// Resolve an extends template (pipeline inheritance)
    fn resolve_extends(&mut self, extends: &Extends, child: Pipeline) -> ParseResult<Pipeline> {
        let template_path = self.resolve_template_path(&extends.template)?;
        let canonical = self.canonical_path(&template_path);

        self.push_template(&canonical)?;

        let template_content = fs::read_to_string(&template_path).map_err(|e| {
            TemplateError::new(
                format!("failed to read extends template: {}", e),
                TemplateErrorKind::NotFound,
            )
            .with_path(&extends.template)
            .to_parse_error()
        })?;

        let mut parent = AzureParser::parse(&template_content).map_err(|e| {
            ParseError::new(
                format!(
                    "error in extends template '{}': {}",
                    extends.template, e.message
                ),
                e.line,
                e.column,
            )
            .with_kind(ParseErrorKind::TemplateError)
        })?;

        // Validate parameters
        let params =
            self.resolve_parameters(&parent.parameters, &extends.parameters, &extends.template)?;

        // Substitute parameters in parent template
        parent = self.substitute_template_parameters(parent, &params)?;

        // Merge child into parent (child values override parent where applicable)
        let merged = self.merge_extends(parent, child);

        self.pop_template();

        // Recursively resolve templates in the merged pipeline
        self.resolve_pipeline(merged)
    }

    /// Merge a child pipeline into a parent (extends) pipeline
    fn merge_extends(&self, mut parent: Pipeline, child: Pipeline) -> Pipeline {
        // Child's trigger overrides parent's
        if child.trigger.is_some() {
            parent.trigger = child.trigger;
        }
        if child.pr.is_some() {
            parent.pr = child.pr;
        }
        if child.schedules.is_some() {
            parent.schedules = child.schedules;
        }

        // Merge resources
        if child.resources.is_some() {
            parent.resources = child.resources;
        }

        // Child variables are added (overriding parent on conflict)
        let mut merged_vars = parent.variables;
        for var in child.variables {
            if let Variable::KeyValue { ref name, .. } = var {
                merged_vars.retain(|v| {
                    if let Variable::KeyValue { name: existing, .. } = v {
                        existing != name
                    } else {
                        true
                    }
                });
            }
            merged_vars.push(var);
        }
        parent.variables = merged_vars;

        // Child pool overrides
        if child.pool.is_some() {
            parent.pool = child.pool;
        }

        // Child name overrides
        if child.name.is_some() {
            parent.name = child.name;
        }

        parent
    }

    // =========================================================================
    // Variable Template Resolution
    // =========================================================================

    /// Resolve variable template references
    fn resolve_variable_templates(&mut self, variables: &[Variable]) -> ParseResult<Vec<Variable>> {
        let mut resolved = Vec::new();

        for var in variables {
            match var {
                Variable::Template {
                    template,
                    parameters,
                } => {
                    let expanded = self.expand_variable_template(template, parameters)?;
                    resolved.extend(expanded);
                }
                other => resolved.push(other.clone()),
            }
        }

        Ok(resolved)
    }

    /// Expand a single variable template reference
    fn expand_variable_template(
        &mut self,
        template_ref: &str,
        call_params: &HashMap<String, serde_yaml::Value>,
    ) -> ParseResult<Vec<Variable>> {
        let raw_template_file = self.load_template_file(template_ref)?;

        // Validate and resolve parameters
        let params =
            self.resolve_parameters(&raw_template_file.parameters, call_params, template_ref)?;

        // Build engine and process ${{ if }}, ${{ each }}, and parameter substitution
        let engine = self.build_parameter_engine(&params);
        let template_file = self.resolve_raw_template(&raw_template_file, &engine, template_ref)?;

        let result = match template_file.content {
            TemplateContent::Variables(vars) => {
                // Substitute remaining parameters in variable values
                let mut resolved = Vec::new();
                for var in vars {
                    resolved.push(self.substitute_variable_params(&var, &engine)?);
                }
                Ok(resolved)
            }
            _ => Err(TemplateError::new(
                format!(
                    "template '{}' does not contain variables (found {:?} content instead)",
                    template_ref,
                    content_type_name(&template_file.content)
                ),
                TemplateErrorKind::TypeMismatch,
            )
            .with_path(template_ref)
            .to_parse_error()),
        };

        self.pop_template();
        result
    }

    // =========================================================================
    // Stage Template Resolution
    // =========================================================================

    /// Resolve stage template references
    fn resolve_stage_templates(&mut self, stages: &[Stage]) -> ParseResult<Vec<Stage>> {
        let mut resolved = Vec::new();

        for stage in stages {
            if let Some(template_ref) = &stage.template {
                let expanded = self.expand_stage_template(template_ref, &stage.parameters)?;
                resolved.extend(expanded);
            } else {
                let mut stage = stage.clone();
                // Resolve variable templates within the stage
                stage.variables = self.resolve_variable_templates(&stage.variables)?;
                // Resolve job templates within the stage
                stage.jobs = self.resolve_job_templates(&stage.jobs)?;
                resolved.push(stage);
            }
        }

        Ok(resolved)
    }

    /// Expand a single stage template reference
    fn expand_stage_template(
        &mut self,
        template_ref: &str,
        call_params: &HashMap<String, serde_yaml::Value>,
    ) -> ParseResult<Vec<Stage>> {
        let raw_template_file = self.load_template_file(template_ref)?;

        let params =
            self.resolve_parameters(&raw_template_file.parameters, call_params, template_ref)?;

        // Build engine and process ${{ if }}, ${{ each }}, and parameter substitution
        let engine = self.build_parameter_engine(&params);
        let template_file = self.resolve_raw_template(&raw_template_file, &engine, template_ref)?;

        let result = match template_file.content {
            TemplateContent::Stages(stages) => {
                let mut resolved = Vec::new();
                for stage in stages {
                    let expanded = self.substitute_stage_params(&stage, &engine)?;
                    // Recursively resolve templates within the expanded stage
                    let mut expanded_stage = expanded;
                    expanded_stage.variables =
                        self.resolve_variable_templates(&expanded_stage.variables)?;
                    expanded_stage.jobs = self.resolve_job_templates(&expanded_stage.jobs)?;
                    resolved.push(expanded_stage);
                }
                Ok(resolved)
            }
            _ => Err(TemplateError::new(
                format!("template '{}' does not contain stages", template_ref),
                TemplateErrorKind::TypeMismatch,
            )
            .with_path(template_ref)
            .to_parse_error()),
        };

        self.pop_template();
        result
    }

    // =========================================================================
    // Job Template Resolution
    // =========================================================================

    /// Resolve job template references
    fn resolve_job_templates(&mut self, jobs: &[Job]) -> ParseResult<Vec<Job>> {
        let mut resolved = Vec::new();

        for job in jobs {
            if let Some(template_ref) = &job.template {
                let expanded = self.expand_job_template(template_ref, &job.parameters)?;
                resolved.extend(expanded);
            } else {
                let mut job = job.clone();
                // Resolve variable templates within the job
                job.variables = self.resolve_variable_templates(&job.variables)?;
                // Resolve step templates within the job
                job.steps = self.resolve_step_templates(&job.steps)?;
                resolved.push(job);
            }
        }

        Ok(resolved)
    }

    /// Expand a single job template reference
    fn expand_job_template(
        &mut self,
        template_ref: &str,
        call_params: &HashMap<String, serde_yaml::Value>,
    ) -> ParseResult<Vec<Job>> {
        let raw_template_file = self.load_template_file(template_ref)?;

        let params =
            self.resolve_parameters(&raw_template_file.parameters, call_params, template_ref)?;

        // Build engine and process ${{ if }}, ${{ each }}, and parameter substitution
        let engine = self.build_parameter_engine(&params);
        let template_file = self.resolve_raw_template(&raw_template_file, &engine, template_ref)?;

        let result = match template_file.content {
            TemplateContent::Jobs(jobs) => {
                let mut resolved = Vec::new();
                for job in jobs {
                    let expanded = self.substitute_job_params(&job, &engine)?;
                    // Recursively resolve templates within the expanded job
                    let mut expanded_job = expanded;
                    expanded_job.variables =
                        self.resolve_variable_templates(&expanded_job.variables)?;
                    expanded_job.steps = self.resolve_step_templates(&expanded_job.steps)?;
                    resolved.push(expanded_job);
                }
                Ok(resolved)
            }
            _ => Err(TemplateError::new(
                format!("template '{}' does not contain jobs", template_ref),
                TemplateErrorKind::TypeMismatch,
            )
            .with_path(template_ref)
            .to_parse_error()),
        };

        self.pop_template();
        result
    }

    // =========================================================================
    // Step Template Resolution
    // =========================================================================

    /// Resolve step template references
    fn resolve_step_templates(&mut self, steps: &[Step]) -> ParseResult<Vec<Step>> {
        let mut resolved = Vec::new();

        for step in steps {
            if let StepAction::Template(template_step) = &step.action {
                let expanded =
                    self.expand_step_template(&template_step.template, &template_step.parameters)?;
                resolved.extend(expanded);
            } else {
                resolved.push(step.clone());
            }
        }

        Ok(resolved)
    }

    /// Expand a single step template reference
    fn expand_step_template(
        &mut self,
        template_ref: &str,
        call_params: &HashMap<String, serde_yaml::Value>,
    ) -> ParseResult<Vec<Step>> {
        let raw_template_file = self.load_template_file(template_ref)?;

        let params =
            self.resolve_parameters(&raw_template_file.parameters, call_params, template_ref)?;

        // Build engine and process ${{ if }}, ${{ each }}, and parameter substitution
        let engine = self.build_parameter_engine(&params);
        let template_file = self.resolve_raw_template(&raw_template_file, &engine, template_ref)?;

        let result = match template_file.content {
            TemplateContent::Steps(steps) => {
                let mut resolved = Vec::new();
                for step in steps {
                    let expanded = self.substitute_step_params(&step, &engine)?;
                    // Recursively resolve any nested template steps
                    if let StepAction::Template(nested) = &expanded.action {
                        let nested_steps =
                            self.expand_step_template(&nested.template, &nested.parameters)?;
                        resolved.extend(nested_steps);
                    } else {
                        resolved.push(expanded);
                    }
                }
                Ok(resolved)
            }
            _ => Err(TemplateError::new(
                format!("template '{}' does not contain steps", template_ref),
                TemplateErrorKind::TypeMismatch,
            )
            .with_path(template_ref)
            .to_parse_error()),
        };

        self.pop_template();
        result
    }

    // =========================================================================
    // Template File Loading
    // =========================================================================

    /// Load and parse a template file.
    /// NOTE: This pushes the template onto the include stack for cycle detection.
    /// Callers must call `pop_template()` after they are done expanding the template
    /// (including any recursive resolution of nested templates).
    fn load_template_file(&mut self, template_ref: &str) -> ParseResult<RawTemplateFile> {
        let template_path = self.resolve_template_path(template_ref)?;
        let canonical = self.canonical_path(&template_path);

        self.push_template(&canonical)?;

        let content = fs::read_to_string(&template_path).map_err(|e| {
            self.pop_template();
            TemplateError::new(
                format!("failed to read template '{}': {}", template_ref, e),
                TemplateErrorKind::NotFound,
            )
            .with_path(template_ref)
            .to_parse_error()
        })?;

        let result = self.parse_raw_template_content(template_ref, &content);

        if result.is_err() {
            self.pop_template();
        }
        // On success, caller is responsible for calling pop_template()

        result
    }

    /// Parse template file content into raw form (before expression processing)
    fn parse_raw_template_content(
        &self,
        template_ref: &str,
        content: &str,
    ) -> ParseResult<RawTemplateFile> {
        // Parse as generic YAML first
        let yaml: serde_yaml::Value =
            serde_yaml::from_str(content).map_err(|e| ParseError::from_yaml_error(&e, content))?;

        let mapping = yaml.as_mapping().ok_or_else(|| {
            TemplateError::new(
                format!("template '{}' must be a YAML mapping", template_ref),
                TemplateErrorKind::ParseError,
            )
            .with_path(template_ref)
            .to_parse_error()
        })?;

        // Extract parameters
        let parameters = if let Some(params_val) = mapping.get("parameters") {
            self.parse_template_parameters(params_val)?
        } else {
            Vec::new()
        };

        // Determine content type based on which key is present, but keep raw YAML
        if let Some(steps_val) = mapping.get("steps") {
            Ok(RawTemplateFile {
                parameters,
                content: RawTemplateContent::Steps(steps_val.clone()),
            })
        } else if let Some(jobs_val) = mapping.get("jobs") {
            Ok(RawTemplateFile {
                parameters,
                content: RawTemplateContent::Jobs(jobs_val.clone()),
            })
        } else if let Some(stages_val) = mapping.get("stages") {
            Ok(RawTemplateFile {
                parameters,
                content: RawTemplateContent::Stages(stages_val.clone()),
            })
        } else if let Some(variables_val) = mapping.get("variables") {
            Ok(RawTemplateFile {
                parameters,
                content: RawTemplateContent::Variables(variables_val.clone()),
            })
        } else {
            // Try to parse as a full pipeline (for extends) - store raw content string
            Ok(RawTemplateFile {
                parameters,
                content: RawTemplateContent::Pipeline(content.to_string()),
            })
        }
    }

    /// Process template expressions (${{ if }}, ${{ each }}) in raw YAML content
    /// and deserialize to a TemplateFile.
    fn resolve_raw_template(
        &self,
        raw: &RawTemplateFile,
        engine: &ExpressionEngine,
        template_ref: &str,
    ) -> ParseResult<TemplateFile> {
        let content = match &raw.content {
            RawTemplateContent::Steps(yaml_val) => {
                let processed = self.process_template_expressions(yaml_val, engine)?;
                let steps: Vec<Step> = serde_yaml::from_value(processed).map_err(|e| {
                    ParseError::new(
                        format!("error parsing steps in template '{}': {}", template_ref, e),
                        0,
                        0,
                    )
                    .with_kind(ParseErrorKind::TemplateError)
                })?;
                TemplateContent::Steps(steps)
            }
            RawTemplateContent::Jobs(yaml_val) => {
                let processed = self.process_template_expressions(yaml_val, engine)?;
                let jobs: Vec<Job> = serde_yaml::from_value(processed).map_err(|e| {
                    ParseError::new(
                        format!("error parsing jobs in template '{}': {}", template_ref, e),
                        0,
                        0,
                    )
                    .with_kind(ParseErrorKind::TemplateError)
                })?;
                TemplateContent::Jobs(jobs)
            }
            RawTemplateContent::Stages(yaml_val) => {
                let processed = self.process_template_expressions(yaml_val, engine)?;
                let stages: Vec<Stage> = serde_yaml::from_value(processed).map_err(|e| {
                    ParseError::new(
                        format!("error parsing stages in template '{}': {}", template_ref, e),
                        0,
                        0,
                    )
                    .with_kind(ParseErrorKind::TemplateError)
                })?;
                TemplateContent::Stages(stages)
            }
            RawTemplateContent::Variables(yaml_val) => {
                let processed = self.process_template_expressions(yaml_val, engine)?;
                let variables: Vec<Variable> = serde_yaml::from_value(processed).map_err(|e| {
                    ParseError::new(
                        format!(
                            "error parsing variables in template '{}': {}",
                            template_ref, e
                        ),
                        0,
                        0,
                    )
                    .with_kind(ParseErrorKind::TemplateError)
                })?;
                TemplateContent::Variables(variables)
            }
            RawTemplateContent::Pipeline(content_str) => {
                let pipeline = AzureParser::parse(content_str).map_err(|e| {
                    ParseError::new(
                        format!(
                            "template '{}' is not a valid template file: {}",
                            template_ref, e.message
                        ),
                        0,
                        0,
                    )
                    .with_kind(ParseErrorKind::TemplateError)
                })?;
                TemplateContent::Pipeline(Box::new(pipeline))
            }
        };

        Ok(TemplateFile {
            parameters: raw.parameters.clone(),
            content,
        })
    }

    /// Parse template parameter declarations
    fn parse_template_parameters(
        &self,
        params_val: &serde_yaml::Value,
    ) -> ParseResult<Vec<Parameter>> {
        match params_val {
            serde_yaml::Value::Sequence(seq) => {
                let mut params = Vec::new();
                for item in seq {
                    let param: Parameter = serde_yaml::from_value(item.clone()).map_err(|e| {
                        ParseError::new(format!("invalid parameter definition: {}", e), 0, 0)
                            .with_kind(ParseErrorKind::TemplateError)
                    })?;
                    params.push(param);
                }
                Ok(params)
            }
            serde_yaml::Value::Mapping(map) => {
                // Simple key-value parameter format (name: default_value)
                let mut params = Vec::new();
                for (key, value) in map {
                    if let Some(name) = key.as_str() {
                        params.push(Parameter {
                            name: name.to_string(),
                            display_name: None,
                            param_type: ParameterType::String,
                            default: Some(value.clone()),
                            values: None,
                        });
                    }
                }
                Ok(params)
            }
            _ => Err(
                ParseError::new("parameters must be a list or mapping", 0, 0)
                    .with_kind(ParseErrorKind::TemplateError),
            ),
        }
    }

    // =========================================================================
    // Parameter Resolution
    // =========================================================================

    /// Validate and resolve parameters passed to a template
    fn resolve_parameters(
        &self,
        declared: &[Parameter],
        provided: &HashMap<String, serde_yaml::Value>,
        template_ref: &str,
    ) -> ParseResult<HashMap<String, Value>> {
        let mut resolved = HashMap::new();

        for param in declared {
            if let Some(provided_val) = provided.get(&param.name) {
                // Validate type if possible
                self.validate_parameter_type(
                    &param.name,
                    provided_val,
                    &param.param_type,
                    template_ref,
                )?;

                // Validate allowed values
                if let Some(allowed) = &param.values {
                    if !allowed.iter().any(|v| v == provided_val) {
                        return Err(TemplateError::new(
                            format!("parameter '{}' value not in allowed values", param.name),
                            TemplateErrorKind::InvalidParameter,
                        )
                        .with_path(template_ref)
                        .to_parse_error());
                    }
                }

                resolved.insert(param.name.clone(), yaml_to_value(provided_val));
            } else if let Some(default) = &param.default {
                // Use default value
                resolved.insert(param.name.clone(), yaml_to_value(default));
            } else {
                // Required parameter missing
                return Err(TemplateError::new(
                    format!(
                        "required parameter '{}' not provided for template '{}'",
                        param.name, template_ref
                    ),
                    TemplateErrorKind::MissingParameter,
                )
                .with_path(template_ref)
                .to_parse_error());
            }
        }

        // Also pass through any extra parameters not declared (Azure DevOps allows this)
        for (name, value) in provided {
            if !resolved.contains_key(name) {
                resolved.insert(name.clone(), yaml_to_value(value));
            }
        }

        Ok(resolved)
    }

    /// Validate that a parameter value matches the declared type
    fn validate_parameter_type(
        &self,
        name: &str,
        value: &serde_yaml::Value,
        param_type: &ParameterType,
        template_ref: &str,
    ) -> ParseResult<()> {
        let valid = match param_type {
            ParameterType::String => value.is_string() || value.is_number() || value.is_bool(),
            ParameterType::Number => {
                value.is_number()
                    || value
                        .as_str()
                        .map(|s| s.parse::<f64>().is_ok())
                        .unwrap_or(false)
            }
            ParameterType::Boolean => {
                value.is_bool()
                    || value
                        .as_str()
                        .map(|s| s == "true" || s == "false")
                        .unwrap_or(false)
            }
            ParameterType::Object => value.is_mapping() || value.is_sequence(),
            ParameterType::Step => value.is_mapping(),
            ParameterType::StepList => value.is_sequence(),
            ParameterType::Job => value.is_mapping(),
            ParameterType::JobList => value.is_sequence(),
            ParameterType::Stage => value.is_mapping(),
            ParameterType::StageList => value.is_sequence(),
        };

        if !valid {
            Err(TemplateError::new(
                format!(
                    "parameter '{}' expected type {:?} but got {:?}",
                    name, param_type, value
                ),
                TemplateErrorKind::TypeMismatch,
            )
            .with_path(template_ref)
            .to_parse_error())
        } else {
            Ok(())
        }
    }

    // =========================================================================
    // Parameter Substitution
    // =========================================================================

    /// Build an ExpressionEngine with parameters set as context
    fn build_parameter_engine(&self, params: &HashMap<String, Value>) -> ExpressionEngine {
        let ctx = ExpressionContext {
            parameters: params.clone(),
            ..Default::default()
        };
        ExpressionEngine::new(ctx)
    }

    /// Substitute ${{ }} compile-time expressions in a string,
    /// preserving $(macro) and $[ runtime ] expressions for later evaluation.
    fn substitute_compile_time(
        &self,
        text: &str,
        engine: &ExpressionEngine,
    ) -> ParseResult<String> {
        use crate::expression::lexer::{extract_expressions, ExpressionType};

        let expressions = extract_expressions(text);
        let mut result = String::new();

        for expr in expressions {
            match expr {
                ExpressionType::Text(s) => result.push_str(&s),
                ExpressionType::CompileTime(expr_str) => {
                    let value = engine.evaluate_compile_time(&expr_str).map_err(|e| {
                        TemplateError::new(
                            format!(
                                "expression error in '${{{{ {} }}}}': {}",
                                expr_str, e.message
                            ),
                            TemplateErrorKind::ExpressionError,
                        )
                        .to_parse_error()
                    })?;
                    result.push_str(&value.as_string());
                }
                ExpressionType::Macro(var_name) => {
                    // Preserve macros - they are runtime, not template-time
                    result.push_str(&format!("$({})", var_name));
                }
                ExpressionType::Runtime(expr_str) => {
                    // Preserve runtime expressions - they are evaluated at runtime
                    result.push_str(&format!("$[ {} ]", expr_str));
                }
            }
        }

        Ok(result)
    }

    /// Substitute parameters in a pipeline (for extends)
    fn substitute_template_parameters(
        &self,
        mut pipeline: Pipeline,
        params: &HashMap<String, Value>,
    ) -> ParseResult<Pipeline> {
        let engine = self.build_parameter_engine(params);

        // Substitute in variables
        pipeline.variables = pipeline
            .variables
            .iter()
            .map(|v| self.substitute_variable_params(v, &engine))
            .collect::<ParseResult<Vec<_>>>()?;

        // Substitute in stages
        pipeline.stages = pipeline
            .stages
            .iter()
            .map(|s| self.substitute_stage_params(s, &engine))
            .collect::<ParseResult<Vec<_>>>()?;

        // Substitute in jobs
        pipeline.jobs = pipeline
            .jobs
            .iter()
            .map(|j| self.substitute_job_params(j, &engine))
            .collect::<ParseResult<Vec<_>>>()?;

        // Substitute in steps
        pipeline.steps = pipeline
            .steps
            .iter()
            .map(|s| self.substitute_step_params(s, &engine))
            .collect::<ParseResult<Vec<_>>>()?;

        Ok(pipeline)
    }

    /// Substitute parameters in a variable definition
    fn substitute_variable_params(
        &self,
        var: &Variable,
        engine: &ExpressionEngine,
    ) -> ParseResult<Variable> {
        match var {
            Variable::KeyValue {
                name,
                value,
                readonly,
            } => {
                let new_name = self.substitute_compile_time(name, engine)?;
                let new_value = self.substitute_compile_time(value, engine)?;
                Ok(Variable::KeyValue {
                    name: new_name,
                    value: new_value,
                    readonly: *readonly,
                })
            }
            other => Ok(other.clone()),
        }
    }

    /// Substitute parameters in a stage
    fn substitute_stage_params(
        &self,
        stage: &Stage,
        engine: &ExpressionEngine,
    ) -> ParseResult<Stage> {
        let mut new_stage = stage.clone();

        if let Some(stage_name) = &stage.stage {
            new_stage.stage = Some(self.substitute_compile_time(stage_name, engine)?);
        }

        if let Some(display_name) = &stage.display_name {
            new_stage.display_name = Some(self.substitute_compile_time(display_name, engine)?);
        }

        if let Some(condition) = &stage.condition {
            new_stage.condition = Some(self.substitute_compile_time(condition, engine)?);
        }

        // Substitute in variables
        new_stage.variables = stage
            .variables
            .iter()
            .map(|v| self.substitute_variable_params(v, engine))
            .collect::<ParseResult<Vec<_>>>()?;

        // Substitute in jobs
        new_stage.jobs = stage
            .jobs
            .iter()
            .map(|j| self.substitute_job_params(j, engine))
            .collect::<ParseResult<Vec<_>>>()?;

        Ok(new_stage)
    }

    /// Substitute parameters in a job
    fn substitute_job_params(&self, job: &Job, engine: &ExpressionEngine) -> ParseResult<Job> {
        let mut new_job = job.clone();

        if let Some(name) = &job.job {
            new_job.job = Some(self.substitute_compile_time(name, engine)?);
        }

        if let Some(display_name) = &job.display_name {
            new_job.display_name = Some(self.substitute_compile_time(display_name, engine)?);
        }

        if let Some(condition) = &job.condition {
            new_job.condition = Some(self.substitute_compile_time(condition, engine)?);
        }

        // Substitute in variables
        new_job.variables = job
            .variables
            .iter()
            .map(|v| self.substitute_variable_params(v, engine))
            .collect::<ParseResult<Vec<_>>>()?;

        // Substitute in steps
        new_job.steps = job
            .steps
            .iter()
            .map(|s| self.substitute_step_params(s, engine))
            .collect::<ParseResult<Vec<_>>>()?;

        Ok(new_job)
    }

    /// Substitute parameters in a step
    fn substitute_step_params(&self, step: &Step, engine: &ExpressionEngine) -> ParseResult<Step> {
        let mut new_step = step.clone();

        if let Some(display_name) = &step.display_name {
            new_step.display_name = Some(self.substitute_compile_time(display_name, engine)?);
        }

        if let Some(condition) = &step.condition {
            new_step.condition = Some(self.substitute_compile_time(condition, engine)?);
        }

        // Substitute in the action
        new_step.action = self.substitute_step_action_params(&step.action, engine)?;

        // Substitute in env
        let mut new_env = HashMap::new();
        for (key, value) in &step.env {
            let new_key = self.substitute_compile_time(key, engine)?;
            let new_value = self.substitute_compile_time(value, engine)?;
            new_env.insert(new_key, new_value);
        }
        new_step.env = new_env;

        Ok(new_step)
    }

    /// Substitute parameters in a step action
    fn substitute_step_action_params(
        &self,
        action: &StepAction,
        engine: &ExpressionEngine,
    ) -> ParseResult<StepAction> {
        match action {
            StepAction::Script(script_step) => {
                let new_script = self.substitute_compile_time(&script_step.script, engine)?;
                let new_wd = script_step
                    .working_directory
                    .as_ref()
                    .map(|wd| self.substitute_compile_time(wd, engine))
                    .transpose()?;
                Ok(StepAction::Script(ScriptStep {
                    script: new_script,
                    working_directory: new_wd,
                    fail_on_stderr: script_step.fail_on_stderr,
                }))
            }
            StepAction::Bash(bash_step) => {
                let new_script = self.substitute_compile_time(&bash_step.bash, engine)?;
                let new_wd = bash_step
                    .working_directory
                    .as_ref()
                    .map(|wd| self.substitute_compile_time(wd, engine))
                    .transpose()?;
                Ok(StepAction::Bash(BashStep {
                    bash: new_script,
                    working_directory: new_wd,
                    fail_on_stderr: bash_step.fail_on_stderr,
                }))
            }
            StepAction::Pwsh(pwsh_step) => {
                let new_script = self.substitute_compile_time(&pwsh_step.pwsh, engine)?;
                let new_wd = pwsh_step
                    .working_directory
                    .as_ref()
                    .map(|wd| self.substitute_compile_time(wd, engine))
                    .transpose()?;
                Ok(StepAction::Pwsh(PwshStep {
                    pwsh: new_script,
                    working_directory: new_wd,
                    fail_on_stderr: pwsh_step.fail_on_stderr,
                    error_action_preference: pwsh_step.error_action_preference.clone(),
                }))
            }
            StepAction::PowerShell(ps_step) => {
                let new_script = self.substitute_compile_time(&ps_step.powershell, engine)?;
                let new_wd = ps_step
                    .working_directory
                    .as_ref()
                    .map(|wd| self.substitute_compile_time(wd, engine))
                    .transpose()?;
                Ok(StepAction::PowerShell(PowerShellStep {
                    powershell: new_script,
                    working_directory: new_wd,
                    fail_on_stderr: ps_step.fail_on_stderr,
                    error_action_preference: ps_step.error_action_preference.clone(),
                }))
            }
            StepAction::Task(task_step) => {
                let new_task = self.substitute_compile_time(&task_step.task, engine)?;
                let mut new_inputs = HashMap::new();
                for (key, value) in &task_step.inputs {
                    let new_key = self.substitute_compile_time(key, engine)?;
                    let new_value = self.substitute_compile_time(value, engine)?;
                    new_inputs.insert(new_key, new_value);
                }
                Ok(StepAction::Task(TaskStep {
                    task: new_task,
                    inputs: new_inputs,
                }))
            }
            // Template steps: substitute ${{ }} expressions in parameter values
            StepAction::Template(template_step) => {
                let new_template = self.substitute_compile_time(&template_step.template, engine)?;
                let mut new_params = HashMap::new();
                for (key, value) in &template_step.parameters {
                    // Substitute in string parameter values
                    if let serde_yaml::Value::String(s) = value {
                        let new_val = self.substitute_compile_time(s, engine)?;
                        new_params.insert(key.clone(), serde_yaml::Value::String(new_val));
                    } else {
                        new_params.insert(key.clone(), value.clone());
                    }
                }
                Ok(StepAction::Template(TemplateStep {
                    template: new_template,
                    parameters: new_params,
                }))
            }
            // Other actions pass through unchanged
            other => Ok(other.clone()),
        }
    }

    // =========================================================================
    // Template Expression Processing (${{ if }} and ${{ each }})
    // =========================================================================

    /// Process `${{ if }}` and `${{ each }}` template expressions in a serde_yaml::Value tree.
    /// These are compile-time structural directives that conditionally include or
    /// repeat YAML nodes before deserialization to typed structs.
    ///
    /// In Azure DevOps YAML, these appear as mapping entries within sequences:
    /// ```yaml
    /// steps:
    ///   - ${{ if eq(parameters.runTests, true) }}:
    ///     - script: cargo test
    ///   - ${{ each env in parameters.environments }}:
    ///     - script: echo deploying to ${{ env }}
    /// ```
    fn process_template_expressions(
        &self,
        value: &serde_yaml::Value,
        engine: &ExpressionEngine,
    ) -> ParseResult<serde_yaml::Value> {
        match value {
            serde_yaml::Value::Sequence(seq) => {
                let mut result = Vec::new();
                // Track if/elseif/else chaining: when we encounter an ${{ if }},
                // we record whether any branch in the chain was taken. Subsequent
                // ${{ elseif }} and ${{ else }} directives check this state.
                let mut chain_active = false; // Are we inside an if/elseif/else chain?
                let mut chain_taken = false; // Was any branch in the current chain already taken?

                for item in seq {
                    // Determine if this item is a directive
                    let directive = self.extract_directive(item);

                    match &directive {
                        Some((TemplateDirective::If(_), _)) => {
                            // Start a new if-chain
                            chain_active = true;
                            chain_taken = false;
                        }
                        Some((TemplateDirective::ElseIf(_), _))
                        | Some((TemplateDirective::Else, _)) => {
                            // Continue existing chain - if no chain is active, treat as standalone
                            if !chain_active {
                                chain_active = true;
                                chain_taken = false;
                            }
                        }
                        _ => {
                            // Non-directive item breaks the chain
                            chain_active = false;
                            chain_taken = false;
                        }
                    }

                    match directive {
                        Some((TemplateDirective::If(condition), val)) => {
                            let cond_result =
                                engine.evaluate_compile_time(&condition).map_err(|e| {
                                    TemplateError::new(
                                        format!(
                                            "error evaluating if condition '{}': {}",
                                            condition, e.message
                                        ),
                                        TemplateErrorKind::ExpressionError,
                                    )
                                    .to_parse_error()
                                })?;

                            if cond_result.is_truthy() {
                                let expanded = self.expand_directive_body(val, engine)?;
                                result.extend(expanded);
                                chain_taken = true;
                            }
                        }
                        Some((TemplateDirective::ElseIf(condition), val)) => {
                            if !chain_taken {
                                let cond_result =
                                    engine.evaluate_compile_time(&condition).map_err(|e| {
                                        TemplateError::new(
                                            format!(
                                                "error evaluating elseif condition '{}': {}",
                                                condition, e.message
                                            ),
                                            TemplateErrorKind::ExpressionError,
                                        )
                                        .to_parse_error()
                                    })?;

                                if cond_result.is_truthy() {
                                    let expanded = self.expand_directive_body(val, engine)?;
                                    result.extend(expanded);
                                    chain_taken = true;
                                }
                            }
                        }
                        Some((TemplateDirective::Else, val)) => {
                            if !chain_taken {
                                let expanded = self.expand_directive_body(val, engine)?;
                                result.extend(expanded);
                                chain_taken = true;
                            }
                        }
                        Some((TemplateDirective::Each(var_name, collection_expr), val)) => {
                            let collection = engine
                                .evaluate_compile_time(&collection_expr)
                                .map_err(|e| {
                                    TemplateError::new(
                                        format!(
                                            "error evaluating each collection '{}': {}",
                                            collection_expr, e.message
                                        ),
                                        TemplateErrorKind::ExpressionError,
                                    )
                                    .to_parse_error()
                                })?;

                            let items = self.value_to_iterable(&collection)?;

                            for (iter_key, iter_value) in &items {
                                let iter_engine = self.build_iteration_engine(
                                    engine, &var_name, iter_key, iter_value,
                                );
                                let expanded = self.expand_directive_body(val, &iter_engine)?;
                                result.extend(expanded);
                            }
                        }
                        None => {
                            // Not a directive - process recursively and include
                            let processed = self.process_template_expressions(item, engine)?;
                            result.push(processed);
                        }
                    }
                }
                Ok(serde_yaml::Value::Sequence(result))
            }
            serde_yaml::Value::Mapping(map) => {
                let mut result = serde_yaml::Mapping::new();
                for (key, val) in map {
                    // Check if the key itself is a template expression
                    if let Some(key_str) = key.as_str() {
                        if let Some(directive) = Self::parse_directive(key_str) {
                            // Process the directive at the mapping level
                            match directive {
                                TemplateDirective::If(condition) => {
                                    let cond_result =
                                        engine.evaluate_compile_time(&condition).map_err(|e| {
                                            TemplateError::new(
                                                format!(
                                                    "error evaluating if condition '{}': {}",
                                                    condition, e.message
                                                ),
                                                TemplateErrorKind::ExpressionError,
                                            )
                                            .to_parse_error()
                                        })?;

                                    if cond_result.is_truthy() {
                                        // Include the value's entries into this mapping
                                        if let Some(inner_map) = val.as_mapping() {
                                            for (ik, iv) in inner_map {
                                                let processed =
                                                    self.process_template_expressions(iv, engine)?;
                                                result.insert(ik.clone(), processed);
                                            }
                                        }
                                    }
                                    continue;
                                }
                                TemplateDirective::ElseIf(_) | TemplateDirective::Else => {
                                    // elseif/else at mapping level - skip for now
                                    // (handled in sequence context with preceding if)
                                    continue;
                                }
                                TemplateDirective::Each(var_name, collection_expr) => {
                                    let collection = engine
                                        .evaluate_compile_time(&collection_expr)
                                        .map_err(|e| {
                                            TemplateError::new(
                                                format!(
                                                    "error evaluating each collection '{}': {}",
                                                    collection_expr, e.message
                                                ),
                                                TemplateErrorKind::ExpressionError,
                                            )
                                            .to_parse_error()
                                        })?;

                                    if let Some(inner_map) = val.as_mapping() {
                                        let items = self.value_to_iterable(&collection)?;
                                        for (iter_key, iter_value) in &items {
                                            let iter_engine = self.build_iteration_engine(
                                                engine, &var_name, iter_key, iter_value,
                                            );
                                            for (ik, iv) in inner_map {
                                                let resolved_key =
                                                    self.substitute_yaml_value(ik, &iter_engine)?;
                                                let resolved_val = self
                                                    .process_template_expressions(
                                                        iv,
                                                        &iter_engine,
                                                    )?;
                                                result.insert(resolved_key, resolved_val);
                                            }
                                        }
                                    }
                                    continue;
                                }
                            }
                        }
                    }

                    // Regular key-value pair: recurse into value
                    let processed = self.process_template_expressions(val, engine)?;
                    result.insert(key.clone(), processed);
                }
                Ok(serde_yaml::Value::Mapping(result))
            }
            serde_yaml::Value::String(s) => {
                // Substitute compile-time expressions in strings
                let substituted = self.substitute_compile_time(s, engine)?;
                Ok(serde_yaml::Value::String(substituted))
            }
            // Scalars pass through unchanged
            other => Ok(other.clone()),
        }
    }

    /// Extract a template directive and its value from a YAML sequence item.
    /// Returns `None` if the item is not a directive.
    fn extract_directive<'a>(
        &self,
        item: &'a serde_yaml::Value,
    ) -> Option<(TemplateDirective, &'a serde_yaml::Value)> {
        let map = item.as_mapping()?;
        if map.len() != 1 {
            return None;
        }
        let (key, val) = map.iter().next()?;
        let key_str = key.as_str()?;
        let directive = Self::parse_directive(key_str)?;
        Some((directive, val))
    }

    /// Expand the body of a directive (the value portion), which should be a sequence.
    fn expand_directive_body(
        &self,
        value: &serde_yaml::Value,
        engine: &ExpressionEngine,
    ) -> ParseResult<Vec<serde_yaml::Value>> {
        match value {
            serde_yaml::Value::Sequence(_) => {
                // Delegate to process_template_expressions which handles
                // if/elseif/else chaining within sequences
                let processed = self.process_template_expressions(value, engine)?;
                if let serde_yaml::Value::Sequence(items) = processed {
                    Ok(items)
                } else {
                    Ok(vec![processed])
                }
            }
            // If the body is a single mapping, wrap it in a vec
            serde_yaml::Value::Mapping(_) => {
                let processed = self.process_template_expressions(value, engine)?;
                Ok(vec![processed])
            }
            // If the body is a scalar value (e.g., ${{ item }} in an each),
            // process it as an expression
            serde_yaml::Value::String(s) => {
                let substituted = self.substitute_compile_time(s, engine)?;
                Ok(vec![serde_yaml::Value::String(substituted)])
            }
            other => Ok(vec![other.clone()]),
        }
    }

    /// Parse a YAML key string to determine if it's a template directive.
    /// Handles: `${{ if condition }}`, `${{ elseif condition }}`, `${{ else }}`,
    /// and `${{ each var in collection }}`.
    fn parse_directive(key: &str) -> Option<TemplateDirective> {
        let trimmed = key.trim();

        // Check for ${{ ... }} pattern
        if !trimmed.starts_with("${{") || !trimmed.ends_with("}}") {
            return None;
        }

        // Extract the inner content
        let inner = trimmed[3..trimmed.len() - 2].trim();

        if let Some(rest) = inner.strip_prefix("if ") {
            let condition = rest.trim().to_string();
            Some(TemplateDirective::If(condition))
        } else if let Some(rest) = inner.strip_prefix("elseif ") {
            let condition = rest.trim().to_string();
            Some(TemplateDirective::ElseIf(condition))
        } else if let Some(rest) = inner.strip_prefix("else if ") {
            let condition = rest.trim().to_string();
            Some(TemplateDirective::ElseIf(condition))
        } else if inner == "else" {
            Some(TemplateDirective::Else)
        } else if let Some(rest) = inner.strip_prefix("each ") {
            // Parse: each <var> in <collection>
            let rest = rest.trim();
            if let Some(in_pos) = rest.find(" in ") {
                let var_name = rest[..in_pos].trim().to_string();
                let collection_expr = rest[in_pos + 4..].trim().to_string();
                if !var_name.is_empty() && !collection_expr.is_empty() {
                    Some(TemplateDirective::Each(var_name, collection_expr))
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Convert a Value into an iterable list of (key, value) pairs.
    /// Arrays yield (index_as_string, item) pairs.
    /// Objects yield (key, value) pairs.
    fn value_to_iterable(&self, value: &Value) -> ParseResult<Vec<(Value, Value)>> {
        match value {
            Value::Array(arr) => Ok(arr
                .iter()
                .enumerate()
                .map(|(i, v)| (Value::Number(i as f64), v.clone()))
                .collect()),
            Value::Object(map) => Ok(map
                .iter()
                .map(|(k, v)| (Value::String(k.clone()), v.clone()))
                .collect()),
            other => Err(TemplateError::new(
                format!(
                    "each directive requires an array or object, got: {}",
                    other.as_string()
                ),
                TemplateErrorKind::ExpressionError,
            )
            .to_parse_error()),
        }
    }

    /// Build an ExpressionEngine for a single iteration of `${{ each }}`.
    /// Adds the iteration variable to the parameters context.
    fn build_iteration_engine(
        &self,
        parent_engine: &ExpressionEngine,
        var_name: &str,
        _iter_key: &Value,
        iter_value: &Value,
    ) -> ExpressionEngine {
        let mut ctx = parent_engine.context().clone();
        ctx.parameters
            .insert(var_name.to_string(), iter_value.clone());
        ExpressionEngine::new(ctx)
    }

    /// Substitute compile-time expressions within a serde_yaml::Value key
    fn substitute_yaml_value(
        &self,
        value: &serde_yaml::Value,
        engine: &ExpressionEngine,
    ) -> ParseResult<serde_yaml::Value> {
        match value {
            serde_yaml::Value::String(s) => {
                let substituted = self.substitute_compile_time(s, engine)?;
                Ok(serde_yaml::Value::String(substituted))
            }
            other => Ok(other.clone()),
        }
    }

    // =========================================================================
    // Path Resolution
    // =========================================================================

    /// Resolve a template reference to an absolute file path
    fn resolve_template_path(&self, template_ref: &str) -> ParseResult<PathBuf> {
        // Check for cross-repository template reference: repo@template
        if let Some((repo_name, template_path)) = template_ref.split_once('@') {
            // Format: template@repo_name  (Azure DevOps uses template path first)
            // Actually Azure DevOps uses: template: steps/build.yml@templates_repo
            if let Some(repo_path) = self.resource_repos.get(repo_name) {
                let full_path = repo_path.join(template_path);
                if full_path.exists() {
                    return Ok(full_path);
                }
                return Err(TemplateError::new(
                    format!(
                        "template '{}' not found in repository '{}' (looked in {})",
                        template_path,
                        repo_name,
                        full_path.display()
                    ),
                    TemplateErrorKind::NotFound,
                )
                .with_path(template_ref)
                .to_parse_error());
            }
            // Also try: file_path@repo_name (the path part is before @)
            if let Some(repo_path) = self.resource_repos.get(template_path) {
                let full_path = repo_path.join(repo_name);
                if full_path.exists() {
                    return Ok(full_path);
                }
            }
        }

        // Local template reference (relative to repo root)
        let full_path = self.repo_root.join(template_ref);
        if full_path.exists() {
            return Ok(full_path);
        }

        Err(TemplateError::new(
            format!(
                "template '{}' not found (looked in {})",
                template_ref,
                full_path.display()
            ),
            TemplateErrorKind::NotFound,
        )
        .with_path(template_ref)
        .to_parse_error())
    }

    // =========================================================================
    // Cycle Detection
    // =========================================================================

    /// Push a template onto the include stack, checking for cycles
    fn push_template(&mut self, canonical_path: &str) -> ParseResult<()> {
        // Check depth limit
        if self.include_stack.len() >= MAX_TEMPLATE_DEPTH {
            return Err(TemplateError::new(
                format!(
                    "maximum template inclusion depth ({}) exceeded. Include stack:\n  {}",
                    MAX_TEMPLATE_DEPTH,
                    self.include_stack.join("\n  -> ")
                ),
                TemplateErrorKind::MaxDepthExceeded,
            )
            .to_parse_error());
        }

        // Check for cycles
        if self.include_stack.contains(&canonical_path.to_string()) {
            let mut cycle = self.include_stack.clone();
            cycle.push(canonical_path.to_string());
            return Err(TemplateError::new(
                format!(
                    "circular template reference detected:\n  {}",
                    cycle.join("\n  -> ")
                ),
                TemplateErrorKind::CircularReference,
            )
            .to_parse_error());
        }

        self.include_stack.push(canonical_path.to_string());
        Ok(())
    }

    /// Pop the current template from the include stack
    fn pop_template(&mut self) {
        self.include_stack.pop();
    }

    /// Get a canonical path string for comparison
    fn canonical_path(&self, path: &Path) -> String {
        path.canonicalize()
            .unwrap_or_else(|_| path.to_path_buf())
            .to_string_lossy()
            .to_string()
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Convert serde_yaml::Value to our Value type
pub fn yaml_to_value(yaml: &serde_yaml::Value) -> Value {
    match yaml {
        serde_yaml::Value::Null => Value::Null,
        serde_yaml::Value::Bool(b) => Value::Bool(*b),
        serde_yaml::Value::Number(n) => {
            Value::Number(n.as_f64().unwrap_or(n.as_i64().unwrap_or(0) as f64))
        }
        serde_yaml::Value::String(s) => Value::String(s.clone()),
        serde_yaml::Value::Sequence(seq) => Value::Array(seq.iter().map(yaml_to_value).collect()),
        serde_yaml::Value::Mapping(map) => Value::Object(
            map.iter()
                .filter_map(|(k, v)| k.as_str().map(|key| (key.to_string(), yaml_to_value(v))))
                .collect(),
        ),
        serde_yaml::Value::Tagged(_) => Value::Null,
    }
}

/// Convert our Value type back to serde_yaml::Value
pub fn value_to_yaml(value: &Value) -> serde_yaml::Value {
    match value {
        Value::Null => serde_yaml::Value::Null,
        Value::Bool(b) => serde_yaml::Value::Bool(*b),
        Value::Number(n) => {
            if n.fract() == 0.0 {
                serde_yaml::Value::Number(serde_yaml::Number::from(*n as i64))
            } else {
                serde_yaml::Value::Number(serde_yaml::Number::from(*n))
            }
        }
        Value::String(s) => serde_yaml::Value::String(s.clone()),
        Value::Array(arr) => serde_yaml::Value::Sequence(arr.iter().map(value_to_yaml).collect()),
        Value::Object(map) => {
            let mut mapping = serde_yaml::Mapping::new();
            for (k, v) in map {
                mapping.insert(serde_yaml::Value::String(k.clone()), value_to_yaml(v));
            }
            serde_yaml::Value::Mapping(mapping)
        }
    }
}

/// Get a human-readable name for template content type
fn content_type_name(content: &TemplateContent) -> &'static str {
    match content {
        TemplateContent::Steps(_) => "steps",
        TemplateContent::Jobs(_) => "jobs",
        TemplateContent::Stages(_) => "stages",
        TemplateContent::Variables(_) => "variables",
        TemplateContent::Pipeline(_) => "pipeline",
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    /// Helper to create a temp directory with template files
    fn setup_templates(files: &[(&str, &str)]) -> TempDir {
        let dir = TempDir::new().unwrap();
        for (name, content) in files {
            let path = dir.path().join(name);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            let mut file = fs::File::create(&path).unwrap();
            file.write_all(content.as_bytes()).unwrap();
        }
        dir
    }

    #[test]
    fn test_resolve_step_template() {
        let dir = setup_templates(&[(
            "steps/build.yml",
            r#"
parameters:
  - name: buildConfig
    type: string
    default: Debug

steps:
  - script: echo Building ${{ parameters.buildConfig }}
    displayName: Build ${{ parameters.buildConfig }}
"#,
        )]);

        let mut engine = TemplateEngine::new(dir.path().to_path_buf());

        let pipeline = Pipeline {
            steps: vec![Step {
                name: None,
                display_name: None,
                condition: None,
                continue_on_error: BoolOrExpression::default(),
                enabled: true,
                timeout_in_minutes: None,
                retry_count_on_task_failure: None,
                env: HashMap::new(),
                action: StepAction::Template(TemplateStep {
                    template: "steps/build.yml".to_string(),
                    parameters: {
                        let mut params = HashMap::new();
                        params.insert(
                            "buildConfig".to_string(),
                            serde_yaml::Value::String("Release".to_string()),
                        );
                        params
                    },
                }),
            }],
            ..Default::default()
        };

        let resolved = engine.resolve_pipeline(pipeline).unwrap();
        assert_eq!(resolved.steps.len(), 1);
        if let StepAction::Script(script) = &resolved.steps[0].action {
            assert_eq!(script.script, "echo Building Release");
        } else {
            panic!("expected script step");
        }
        assert_eq!(
            resolved.steps[0].display_name.as_deref(),
            Some("Build Release")
        );
    }

    #[test]
    fn test_resolve_step_template_default_params() {
        let dir = setup_templates(&[(
            "steps/build.yml",
            r#"
parameters:
  - name: buildConfig
    type: string
    default: Debug

steps:
  - script: echo Building ${{ parameters.buildConfig }}
"#,
        )]);

        let mut engine = TemplateEngine::new(dir.path().to_path_buf());

        let pipeline = Pipeline {
            steps: vec![Step {
                name: None,
                display_name: None,
                condition: None,
                continue_on_error: BoolOrExpression::default(),
                enabled: true,
                timeout_in_minutes: None,
                retry_count_on_task_failure: None,
                env: HashMap::new(),
                action: StepAction::Template(TemplateStep {
                    template: "steps/build.yml".to_string(),
                    parameters: HashMap::new(), // No params - use defaults
                }),
            }],
            ..Default::default()
        };

        let resolved = engine.resolve_pipeline(pipeline).unwrap();
        assert_eq!(resolved.steps.len(), 1);
        if let StepAction::Script(script) = &resolved.steps[0].action {
            assert_eq!(script.script, "echo Building Debug");
        } else {
            panic!("expected script step");
        }
    }

    #[test]
    fn test_resolve_job_template() {
        let dir = setup_templates(&[(
            "jobs/build.yml",
            r#"
parameters:
  - name: vmImage
    type: string
    default: ubuntu-latest

jobs:
  - job: Build
    pool:
      vmImage: ${{ parameters.vmImage }}
    steps:
      - script: cargo build
"#,
        )]);

        let mut engine = TemplateEngine::new(dir.path().to_path_buf());

        let pipeline = Pipeline {
            jobs: vec![Job {
                template: Some("jobs/build.yml".to_string()),
                parameters: {
                    let mut params = HashMap::new();
                    params.insert(
                        "vmImage".to_string(),
                        serde_yaml::Value::String("windows-latest".to_string()),
                    );
                    params
                },
                ..Default::default()
            }],
            ..Default::default()
        };

        let resolved = engine.resolve_pipeline(pipeline).unwrap();
        assert_eq!(resolved.jobs.len(), 1);
        assert_eq!(resolved.jobs[0].job, Some("Build".to_string()));
    }

    #[test]
    fn test_resolve_stage_template() {
        let dir = setup_templates(&[(
            "stages/deploy.yml",
            r#"
parameters:
  - name: environment
    type: string

stages:
  - stage: Deploy
    displayName: Deploy to ${{ parameters.environment }}
    jobs:
      - job: DeployJob
        steps:
          - script: echo Deploying to ${{ parameters.environment }}
"#,
        )]);

        let mut engine = TemplateEngine::new(dir.path().to_path_buf());

        let pipeline = Pipeline {
            stages: vec![Stage {
                stage: Some("placeholder".to_string()),
                template: Some("stages/deploy.yml".to_string()),
                parameters: {
                    let mut params = HashMap::new();
                    params.insert(
                        "environment".to_string(),
                        serde_yaml::Value::String("production".to_string()),
                    );
                    params
                },
                ..Default::default()
            }],
            ..Default::default()
        };

        let resolved = engine.resolve_pipeline(pipeline).unwrap();
        assert_eq!(resolved.stages.len(), 1);
        assert_eq!(
            resolved.stages[0].display_name.as_deref(),
            Some("Deploy to production")
        );
    }

    #[test]
    fn test_resolve_variable_template() {
        let dir = setup_templates(&[(
            "variables/common.yml",
            r#"
variables:
  - name: buildConfig
    value: Release
  - name: testFramework
    value: NUnit
"#,
        )]);

        let mut engine = TemplateEngine::new(dir.path().to_path_buf());

        let pipeline = Pipeline {
            variables: vec![
                Variable::Template {
                    template: "variables/common.yml".to_string(),
                    parameters: HashMap::new(),
                },
                Variable::KeyValue {
                    name: "extraVar".to_string(),
                    value: "extraValue".to_string(),
                    readonly: false,
                },
            ],
            steps: vec![Step {
                name: None,
                display_name: None,
                condition: None,
                continue_on_error: BoolOrExpression::default(),
                enabled: true,
                timeout_in_minutes: None,
                retry_count_on_task_failure: None,
                env: HashMap::new(),
                action: StepAction::Script(ScriptStep {
                    script: "echo hello".to_string(),
                    working_directory: None,
                    fail_on_stderr: false,
                }),
            }],
            ..Default::default()
        };

        let resolved = engine.resolve_pipeline(pipeline).unwrap();
        assert_eq!(resolved.variables.len(), 3);
    }

    #[test]
    fn test_resolve_extends_template() {
        let dir = setup_templates(&[(
            "base-pipeline.yml",
            r#"
parameters:
  - name: buildConfig
    type: string
    default: Debug

stages:
  - stage: Build
    jobs:
      - job: BuildJob
        steps:
          - script: echo Building ${{ parameters.buildConfig }}
"#,
        )]);

        let mut engine = TemplateEngine::new(dir.path().to_path_buf());

        let pipeline = Pipeline {
            extends: Some(Extends {
                template: "base-pipeline.yml".to_string(),
                parameters: {
                    let mut params = HashMap::new();
                    params.insert(
                        "buildConfig".to_string(),
                        serde_yaml::Value::String("Release".to_string()),
                    );
                    params
                },
            }),
            ..Default::default()
        };

        let resolved = engine.resolve_pipeline(pipeline).unwrap();
        assert_eq!(resolved.stages.len(), 1);
        assert_eq!(resolved.stages[0].stage, Some("Build".to_string()));
        let build_step = &resolved.stages[0].jobs[0].steps[0];
        if let StepAction::Script(script) = &build_step.action {
            assert_eq!(script.script, "echo Building Release");
        } else {
            panic!("expected script step");
        }
    }

    #[test]
    fn test_circular_reference_detection() {
        let dir = setup_templates(&[
            (
                "a.yml",
                r#"
steps:
  - template: b.yml
"#,
            ),
            (
                "b.yml",
                r#"
steps:
  - template: a.yml
"#,
            ),
        ]);

        let mut engine = TemplateEngine::new(dir.path().to_path_buf());

        let pipeline = Pipeline {
            steps: vec![Step {
                name: None,
                display_name: None,
                condition: None,
                continue_on_error: BoolOrExpression::default(),
                enabled: true,
                timeout_in_minutes: None,
                retry_count_on_task_failure: None,
                env: HashMap::new(),
                action: StepAction::Template(TemplateStep {
                    template: "a.yml".to_string(),
                    parameters: HashMap::new(),
                }),
            }],
            ..Default::default()
        };

        let result = engine.resolve_pipeline(pipeline);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.message.contains("circular") || err.kind == ParseErrorKind::TemplateError,
            "Expected circular reference error, got: {}",
            err.message
        );
    }

    #[test]
    fn test_missing_required_parameter() {
        let dir = setup_templates(&[(
            "steps/build.yml",
            r#"
parameters:
  - name: buildConfig
    type: string

steps:
  - script: echo ${{ parameters.buildConfig }}
"#,
        )]);

        let mut engine = TemplateEngine::new(dir.path().to_path_buf());

        let pipeline = Pipeline {
            steps: vec![Step {
                name: None,
                display_name: None,
                condition: None,
                continue_on_error: BoolOrExpression::default(),
                enabled: true,
                timeout_in_minutes: None,
                retry_count_on_task_failure: None,
                env: HashMap::new(),
                action: StepAction::Template(TemplateStep {
                    template: "steps/build.yml".to_string(),
                    parameters: HashMap::new(), // Missing required param
                }),
            }],
            ..Default::default()
        };

        let result = engine.resolve_pipeline(pipeline);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.message.contains("required parameter"),
            "Expected missing parameter error, got: {}",
            err.message
        );
    }

    #[test]
    fn test_template_not_found() {
        let dir = TempDir::new().unwrap();
        let mut engine = TemplateEngine::new(dir.path().to_path_buf());

        let pipeline = Pipeline {
            steps: vec![Step {
                name: None,
                display_name: None,
                condition: None,
                continue_on_error: BoolOrExpression::default(),
                enabled: true,
                timeout_in_minutes: None,
                retry_count_on_task_failure: None,
                env: HashMap::new(),
                action: StepAction::Template(TemplateStep {
                    template: "nonexistent.yml".to_string(),
                    parameters: HashMap::new(),
                }),
            }],
            ..Default::default()
        };

        let result = engine.resolve_pipeline(pipeline);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("not found"));
    }

    #[test]
    fn test_nested_templates() {
        let dir = setup_templates(&[
            (
                "steps/inner.yml",
                r#"
parameters:
  - name: msg
    type: string

steps:
  - script: echo ${{ parameters.msg }}
"#,
            ),
            (
                "steps/outer.yml",
                r#"
parameters:
  - name: prefix
    type: string

steps:
  - script: echo Starting ${{ parameters.prefix }}
  - template: steps/inner.yml
    parameters:
      msg: ${{ parameters.prefix }} inner
  - script: echo Done ${{ parameters.prefix }}
"#,
            ),
        ]);

        let mut engine = TemplateEngine::new(dir.path().to_path_buf());

        let pipeline = Pipeline {
            steps: vec![Step {
                name: None,
                display_name: None,
                condition: None,
                continue_on_error: BoolOrExpression::default(),
                enabled: true,
                timeout_in_minutes: None,
                retry_count_on_task_failure: None,
                env: HashMap::new(),
                action: StepAction::Template(TemplateStep {
                    template: "steps/outer.yml".to_string(),
                    parameters: {
                        let mut params = HashMap::new();
                        params.insert(
                            "prefix".to_string(),
                            serde_yaml::Value::String("Build".to_string()),
                        );
                        params
                    },
                }),
            }],
            ..Default::default()
        };

        let resolved = engine.resolve_pipeline(pipeline).unwrap();
        assert_eq!(resolved.steps.len(), 3);

        if let StepAction::Script(script) = &resolved.steps[0].action {
            assert_eq!(script.script, "echo Starting Build");
        } else {
            panic!("expected script step at index 0");
        }

        if let StepAction::Script(script) = &resolved.steps[1].action {
            assert_eq!(script.script, "echo Build inner");
        } else {
            panic!("expected script step at index 1");
        }

        if let StepAction::Script(script) = &resolved.steps[2].action {
            assert_eq!(script.script, "echo Done Build");
        } else {
            panic!("expected script step at index 2");
        }
    }

    #[test]
    fn test_macro_variables_preserved() {
        let dir = setup_templates(&[(
            "steps/build.yml",
            r#"
parameters:
  - name: config
    type: string

steps:
  - script: echo Building $(Build.SourceBranch) with ${{ parameters.config }}
"#,
        )]);

        let mut engine = TemplateEngine::new(dir.path().to_path_buf());

        let pipeline = Pipeline {
            steps: vec![Step {
                name: None,
                display_name: None,
                condition: None,
                continue_on_error: BoolOrExpression::default(),
                enabled: true,
                timeout_in_minutes: None,
                retry_count_on_task_failure: None,
                env: HashMap::new(),
                action: StepAction::Template(TemplateStep {
                    template: "steps/build.yml".to_string(),
                    parameters: {
                        let mut params = HashMap::new();
                        params.insert(
                            "config".to_string(),
                            serde_yaml::Value::String("Release".to_string()),
                        );
                        params
                    },
                }),
            }],
            ..Default::default()
        };

        let resolved = engine.resolve_pipeline(pipeline).unwrap();
        if let StepAction::Script(script) = &resolved.steps[0].action {
            // ${{ parameters.config }} should be resolved, $(Build.SourceBranch) should be preserved
            assert_eq!(
                script.script,
                "echo Building $(Build.SourceBranch) with Release"
            );
        } else {
            panic!("expected script step");
        }
    }

    #[test]
    fn test_yaml_to_value_conversion() {
        assert_eq!(yaml_to_value(&serde_yaml::Value::Null), Value::Null);
        assert_eq!(
            yaml_to_value(&serde_yaml::Value::Bool(true)),
            Value::Bool(true)
        );
        assert_eq!(
            yaml_to_value(&serde_yaml::Value::String("hello".to_string())),
            Value::String("hello".to_string())
        );

        let seq = serde_yaml::Value::Sequence(vec![
            serde_yaml::Value::String("a".to_string()),
            serde_yaml::Value::String("b".to_string()),
        ]);
        if let Value::Array(arr) = yaml_to_value(&seq) {
            assert_eq!(arr.len(), 2);
        } else {
            panic!("expected array");
        }
    }

    #[test]
    fn test_value_to_yaml_conversion() {
        let val = Value::String("hello".to_string());
        let yaml = value_to_yaml(&val);
        assert_eq!(yaml, serde_yaml::Value::String("hello".to_string()));

        let val = Value::Bool(true);
        let yaml = value_to_yaml(&val);
        assert_eq!(yaml, serde_yaml::Value::Bool(true));

        let val = Value::Array(vec![Value::String("a".to_string())]);
        let yaml = value_to_yaml(&val);
        assert!(yaml.is_sequence());
    }

    #[test]
    fn test_simple_key_value_parameters() {
        let dir = setup_templates(&[(
            "steps/build.yml",
            r#"
parameters:
  buildConfig: Debug
  platform: x64

steps:
  - script: echo ${{ parameters.buildConfig }} ${{ parameters.platform }}
"#,
        )]);

        let mut engine = TemplateEngine::new(dir.path().to_path_buf());

        let pipeline = Pipeline {
            steps: vec![Step {
                name: None,
                display_name: None,
                condition: None,
                continue_on_error: BoolOrExpression::default(),
                enabled: true,
                timeout_in_minutes: None,
                retry_count_on_task_failure: None,
                env: HashMap::new(),
                action: StepAction::Template(TemplateStep {
                    template: "steps/build.yml".to_string(),
                    parameters: HashMap::new(), // Use defaults
                }),
            }],
            ..Default::default()
        };

        let resolved = engine.resolve_pipeline(pipeline).unwrap();
        if let StepAction::Script(script) = &resolved.steps[0].action {
            assert_eq!(script.script, "echo Debug x64");
        } else {
            panic!("expected script step");
        }
    }

    #[test]
    fn test_multiple_step_templates() {
        let dir = setup_templates(&[
            (
                "steps/build.yml",
                r#"
steps:
  - script: cargo build
    displayName: Build
"#,
            ),
            (
                "steps/test.yml",
                r#"
steps:
  - script: cargo test
    displayName: Test
"#,
            ),
        ]);

        let mut engine = TemplateEngine::new(dir.path().to_path_buf());

        let pipeline = Pipeline {
            steps: vec![
                Step {
                    name: None,
                    display_name: None,
                    condition: None,
                    continue_on_error: BoolOrExpression::default(),
                    enabled: true,
                    timeout_in_minutes: None,
                    retry_count_on_task_failure: None,
                    env: HashMap::new(),
                    action: StepAction::Template(TemplateStep {
                        template: "steps/build.yml".to_string(),
                        parameters: HashMap::new(),
                    }),
                },
                Step {
                    name: None,
                    display_name: None,
                    condition: None,
                    continue_on_error: BoolOrExpression::default(),
                    enabled: true,
                    timeout_in_minutes: None,
                    retry_count_on_task_failure: None,
                    env: HashMap::new(),
                    action: StepAction::Template(TemplateStep {
                        template: "steps/test.yml".to_string(),
                        parameters: HashMap::new(),
                    }),
                },
            ],
            ..Default::default()
        };

        let resolved = engine.resolve_pipeline(pipeline).unwrap();
        assert_eq!(resolved.steps.len(), 2);
        assert_eq!(resolved.steps[0].display_name.as_deref(), Some("Build"));
        assert_eq!(resolved.steps[1].display_name.as_deref(), Some("Test"));
    }

    #[test]
    fn test_extends_with_child_overrides() {
        let dir = setup_templates(&[(
            "base.yml",
            r#"
variables:
  - name: baseVar
    value: baseValue
  - name: sharedVar
    value: fromBase

stages:
  - stage: Build
    jobs:
      - job: BuildJob
        steps:
          - script: echo base build
"#,
        )]);

        let mut engine = TemplateEngine::new(dir.path().to_path_buf());

        let pipeline = Pipeline {
            extends: Some(Extends {
                template: "base.yml".to_string(),
                parameters: HashMap::new(),
            }),
            variables: vec![Variable::KeyValue {
                name: "sharedVar".to_string(),
                value: "fromChild".to_string(),
                readonly: false,
            }],
            ..Default::default()
        };

        let resolved = engine.resolve_pipeline(pipeline).unwrap();

        // Should have both baseVar and the overridden sharedVar
        let shared = resolved.variables.iter().find(|v| {
            if let Variable::KeyValue { name, .. } = v {
                name == "sharedVar"
            } else {
                false
            }
        });
        assert!(shared.is_some());
        if let Some(Variable::KeyValue { value, .. }) = shared {
            assert_eq!(value, "fromChild");
        }
    }

    // =========================================================================
    // ${{ if }} directive tests
    // =========================================================================

    #[test]
    fn test_if_directive_true_condition() {
        let dir = setup_templates(&[(
            "steps/conditional.yml",
            r#"
parameters:
  - name: runTests
    type: boolean
    default: true

steps:
  - script: echo always runs
  - ${{ if eq(parameters.runTests, true) }}:
    - script: cargo test
      displayName: Run Tests
"#,
        )]);

        let mut engine = TemplateEngine::new(dir.path().to_path_buf());

        let pipeline = Pipeline {
            steps: vec![Step {
                name: None,
                display_name: None,
                condition: None,
                continue_on_error: BoolOrExpression::default(),
                enabled: true,
                timeout_in_minutes: None,
                retry_count_on_task_failure: None,
                env: HashMap::new(),
                action: StepAction::Template(TemplateStep {
                    template: "steps/conditional.yml".to_string(),
                    parameters: {
                        let mut params = HashMap::new();
                        params.insert("runTests".to_string(), serde_yaml::Value::Bool(true));
                        params
                    },
                }),
            }],
            ..Default::default()
        };

        let resolved = engine.resolve_pipeline(pipeline).unwrap();
        assert_eq!(resolved.steps.len(), 2);
        if let StepAction::Script(script) = &resolved.steps[0].action {
            assert_eq!(script.script, "echo always runs");
        }
        if let StepAction::Script(script) = &resolved.steps[1].action {
            assert_eq!(script.script, "cargo test");
        }
        assert_eq!(resolved.steps[1].display_name.as_deref(), Some("Run Tests"));
    }

    #[test]
    fn test_if_directive_false_condition() {
        let dir = setup_templates(&[(
            "steps/conditional.yml",
            r#"
parameters:
  - name: runTests
    type: boolean
    default: true

steps:
  - script: echo always runs
  - ${{ if eq(parameters.runTests, true) }}:
    - script: cargo test
      displayName: Run Tests
"#,
        )]);

        let mut engine = TemplateEngine::new(dir.path().to_path_buf());

        let pipeline = Pipeline {
            steps: vec![Step {
                name: None,
                display_name: None,
                condition: None,
                continue_on_error: BoolOrExpression::default(),
                enabled: true,
                timeout_in_minutes: None,
                retry_count_on_task_failure: None,
                env: HashMap::new(),
                action: StepAction::Template(TemplateStep {
                    template: "steps/conditional.yml".to_string(),
                    parameters: {
                        let mut params = HashMap::new();
                        params.insert("runTests".to_string(), serde_yaml::Value::Bool(false));
                        params
                    },
                }),
            }],
            ..Default::default()
        };

        let resolved = engine.resolve_pipeline(pipeline).unwrap();
        // Only the unconditional step should be present
        assert_eq!(resolved.steps.len(), 1);
        if let StepAction::Script(script) = &resolved.steps[0].action {
            assert_eq!(script.script, "echo always runs");
        }
    }

    #[test]
    fn test_if_directive_with_string_comparison() {
        let dir = setup_templates(&[(
            "steps/env-steps.yml",
            r#"
parameters:
  - name: environment
    type: string

steps:
  - script: echo deploying
  - ${{ if eq(parameters.environment, 'production') }}:
    - script: echo production safety checks
  - ${{ if ne(parameters.environment, 'production') }}:
    - script: echo skipping safety checks
"#,
        )]);

        let mut engine = TemplateEngine::new(dir.path().to_path_buf());

        // Test with production
        let pipeline = Pipeline {
            steps: vec![Step {
                name: None,
                display_name: None,
                condition: None,
                continue_on_error: BoolOrExpression::default(),
                enabled: true,
                timeout_in_minutes: None,
                retry_count_on_task_failure: None,
                env: HashMap::new(),
                action: StepAction::Template(TemplateStep {
                    template: "steps/env-steps.yml".to_string(),
                    parameters: {
                        let mut params = HashMap::new();
                        params.insert(
                            "environment".to_string(),
                            serde_yaml::Value::String("production".to_string()),
                        );
                        params
                    },
                }),
            }],
            ..Default::default()
        };

        let resolved = engine.resolve_pipeline(pipeline).unwrap();
        assert_eq!(resolved.steps.len(), 2);
        if let StepAction::Script(script) = &resolved.steps[1].action {
            assert_eq!(script.script, "echo production safety checks");
        }
    }

    #[test]
    fn test_if_directive_multiple_items() {
        let dir = setup_templates(&[(
            "steps/multi.yml",
            r#"
parameters:
  - name: includeExtra
    type: boolean
    default: true

steps:
  - script: echo first
  - ${{ if eq(parameters.includeExtra, true) }}:
    - script: echo extra step 1
    - script: echo extra step 2
    - script: echo extra step 3
  - script: echo last
"#,
        )]);

        let mut engine = TemplateEngine::new(dir.path().to_path_buf());

        let pipeline = Pipeline {
            steps: vec![Step {
                name: None,
                display_name: None,
                condition: None,
                continue_on_error: BoolOrExpression::default(),
                enabled: true,
                timeout_in_minutes: None,
                retry_count_on_task_failure: None,
                env: HashMap::new(),
                action: StepAction::Template(TemplateStep {
                    template: "steps/multi.yml".to_string(),
                    parameters: {
                        let mut params = HashMap::new();
                        params.insert("includeExtra".to_string(), serde_yaml::Value::Bool(true));
                        params
                    },
                }),
            }],
            ..Default::default()
        };

        let resolved = engine.resolve_pipeline(pipeline).unwrap();
        // first + 3 extra + last = 5 steps
        assert_eq!(resolved.steps.len(), 5);
        if let StepAction::Script(script) = &resolved.steps[0].action {
            assert_eq!(script.script, "echo first");
        }
        if let StepAction::Script(script) = &resolved.steps[1].action {
            assert_eq!(script.script, "echo extra step 1");
        }
        if let StepAction::Script(script) = &resolved.steps[4].action {
            assert_eq!(script.script, "echo last");
        }
    }

    // =========================================================================
    // ${{ each }} directive tests
    // =========================================================================

    #[test]
    fn test_each_directive_array() {
        let dir = setup_templates(&[(
            "steps/deploy.yml",
            r#"
parameters:
  - name: environments
    type: object

steps:
  - ${{ each env in parameters.environments }}:
    - script: echo deploying to ${{ env }}
"#,
        )]);

        let mut engine = TemplateEngine::new(dir.path().to_path_buf());

        let pipeline = Pipeline {
            steps: vec![Step {
                name: None,
                display_name: None,
                condition: None,
                continue_on_error: BoolOrExpression::default(),
                enabled: true,
                timeout_in_minutes: None,
                retry_count_on_task_failure: None,
                env: HashMap::new(),
                action: StepAction::Template(TemplateStep {
                    template: "steps/deploy.yml".to_string(),
                    parameters: {
                        let mut params = HashMap::new();
                        params.insert(
                            "environments".to_string(),
                            serde_yaml::Value::Sequence(vec![
                                serde_yaml::Value::String("dev".to_string()),
                                serde_yaml::Value::String("staging".to_string()),
                                serde_yaml::Value::String("production".to_string()),
                            ]),
                        );
                        params
                    },
                }),
            }],
            ..Default::default()
        };

        let resolved = engine.resolve_pipeline(pipeline).unwrap();
        assert_eq!(resolved.steps.len(), 3);
        if let StepAction::Script(script) = &resolved.steps[0].action {
            assert_eq!(script.script, "echo deploying to dev");
        }
        if let StepAction::Script(script) = &resolved.steps[1].action {
            assert_eq!(script.script, "echo deploying to staging");
        }
        if let StepAction::Script(script) = &resolved.steps[2].action {
            assert_eq!(script.script, "echo deploying to production");
        }
    }

    #[test]
    fn test_each_directive_with_multiple_steps_per_iteration() {
        let dir = setup_templates(&[(
            "steps/multi-deploy.yml",
            r#"
parameters:
  - name: environments
    type: object

steps:
  - ${{ each env in parameters.environments }}:
    - script: echo starting deploy to ${{ env }}
    - script: echo finished deploy to ${{ env }}
"#,
        )]);

        let mut engine = TemplateEngine::new(dir.path().to_path_buf());

        let pipeline = Pipeline {
            steps: vec![Step {
                name: None,
                display_name: None,
                condition: None,
                continue_on_error: BoolOrExpression::default(),
                enabled: true,
                timeout_in_minutes: None,
                retry_count_on_task_failure: None,
                env: HashMap::new(),
                action: StepAction::Template(TemplateStep {
                    template: "steps/multi-deploy.yml".to_string(),
                    parameters: {
                        let mut params = HashMap::new();
                        params.insert(
                            "environments".to_string(),
                            serde_yaml::Value::Sequence(vec![
                                serde_yaml::Value::String("dev".to_string()),
                                serde_yaml::Value::String("prod".to_string()),
                            ]),
                        );
                        params
                    },
                }),
            }],
            ..Default::default()
        };

        let resolved = engine.resolve_pipeline(pipeline).unwrap();
        // 2 environments * 2 steps each = 4 steps
        assert_eq!(resolved.steps.len(), 4);
        if let StepAction::Script(script) = &resolved.steps[0].action {
            assert_eq!(script.script, "echo starting deploy to dev");
        }
        if let StepAction::Script(script) = &resolved.steps[1].action {
            assert_eq!(script.script, "echo finished deploy to dev");
        }
        if let StepAction::Script(script) = &resolved.steps[2].action {
            assert_eq!(script.script, "echo starting deploy to prod");
        }
        if let StepAction::Script(script) = &resolved.steps[3].action {
            assert_eq!(script.script, "echo finished deploy to prod");
        }
    }

    #[test]
    fn test_each_directive_empty_array() {
        let dir = setup_templates(&[(
            "steps/deploy.yml",
            r#"
parameters:
  - name: environments
    type: object

steps:
  - script: echo before
  - ${{ each env in parameters.environments }}:
    - script: echo deploying to ${{ env }}
  - script: echo after
"#,
        )]);

        let mut engine = TemplateEngine::new(dir.path().to_path_buf());

        let pipeline = Pipeline {
            steps: vec![Step {
                name: None,
                display_name: None,
                condition: None,
                continue_on_error: BoolOrExpression::default(),
                enabled: true,
                timeout_in_minutes: None,
                retry_count_on_task_failure: None,
                env: HashMap::new(),
                action: StepAction::Template(TemplateStep {
                    template: "steps/deploy.yml".to_string(),
                    parameters: {
                        let mut params = HashMap::new();
                        params.insert(
                            "environments".to_string(),
                            serde_yaml::Value::Sequence(vec![]),
                        );
                        params
                    },
                }),
            }],
            ..Default::default()
        };

        let resolved = engine.resolve_pipeline(pipeline).unwrap();
        // Only before + after, no items from each
        assert_eq!(resolved.steps.len(), 2);
        if let StepAction::Script(script) = &resolved.steps[0].action {
            assert_eq!(script.script, "echo before");
        }
        if let StepAction::Script(script) = &resolved.steps[1].action {
            assert_eq!(script.script, "echo after");
        }
    }

    #[test]
    fn test_if_and_each_combined() {
        let dir = setup_templates(&[(
            "steps/combined.yml",
            r#"
parameters:
  - name: runDeploy
    type: boolean
  - name: environments
    type: object

steps:
  - script: echo building
  - ${{ if eq(parameters.runDeploy, true) }}:
    - ${{ each env in parameters.environments }}:
      - script: echo deploying to ${{ env }}
"#,
        )]);

        let mut engine = TemplateEngine::new(dir.path().to_path_buf());

        // Test with deploy enabled
        let pipeline = Pipeline {
            steps: vec![Step {
                name: None,
                display_name: None,
                condition: None,
                continue_on_error: BoolOrExpression::default(),
                enabled: true,
                timeout_in_minutes: None,
                retry_count_on_task_failure: None,
                env: HashMap::new(),
                action: StepAction::Template(TemplateStep {
                    template: "steps/combined.yml".to_string(),
                    parameters: {
                        let mut params = HashMap::new();
                        params.insert("runDeploy".to_string(), serde_yaml::Value::Bool(true));
                        params.insert(
                            "environments".to_string(),
                            serde_yaml::Value::Sequence(vec![
                                serde_yaml::Value::String("dev".to_string()),
                                serde_yaml::Value::String("prod".to_string()),
                            ]),
                        );
                        params
                    },
                }),
            }],
            ..Default::default()
        };

        let resolved = engine.resolve_pipeline(pipeline).unwrap();
        // build + 2 deploy steps = 3
        assert_eq!(resolved.steps.len(), 3);
        if let StepAction::Script(script) = &resolved.steps[1].action {
            assert_eq!(script.script, "echo deploying to dev");
        }
        if let StepAction::Script(script) = &resolved.steps[2].action {
            assert_eq!(script.script, "echo deploying to prod");
        }
    }

    #[test]
    fn test_if_and_each_combined_false() {
        let dir = setup_templates(&[(
            "steps/combined.yml",
            r#"
parameters:
  - name: runDeploy
    type: boolean
  - name: environments
    type: object

steps:
  - script: echo building
  - ${{ if eq(parameters.runDeploy, true) }}:
    - ${{ each env in parameters.environments }}:
      - script: echo deploying to ${{ env }}
"#,
        )]);

        let mut engine = TemplateEngine::new(dir.path().to_path_buf());

        // Test with deploy disabled
        let pipeline = Pipeline {
            steps: vec![Step {
                name: None,
                display_name: None,
                condition: None,
                continue_on_error: BoolOrExpression::default(),
                enabled: true,
                timeout_in_minutes: None,
                retry_count_on_task_failure: None,
                env: HashMap::new(),
                action: StepAction::Template(TemplateStep {
                    template: "steps/combined.yml".to_string(),
                    parameters: {
                        let mut params = HashMap::new();
                        params.insert("runDeploy".to_string(), serde_yaml::Value::Bool(false));
                        params.insert(
                            "environments".to_string(),
                            serde_yaml::Value::Sequence(vec![
                                serde_yaml::Value::String("dev".to_string()),
                                serde_yaml::Value::String("prod".to_string()),
                            ]),
                        );
                        params
                    },
                }),
            }],
            ..Default::default()
        };

        let resolved = engine.resolve_pipeline(pipeline).unwrap();
        // Only the build step
        assert_eq!(resolved.steps.len(), 1);
        if let StepAction::Script(script) = &resolved.steps[0].action {
            assert_eq!(script.script, "echo building");
        }
    }

    // =========================================================================
    // parse_directive unit tests
    // =========================================================================

    #[test]
    fn test_parse_directive_if() {
        let result = TemplateEngine::parse_directive("${{ if eq(parameters.x, true) }}");
        assert!(result.is_some());
        if let Some(TemplateDirective::If(condition)) = result {
            assert_eq!(condition, "eq(parameters.x, true)");
        } else {
            panic!("expected If directive");
        }
    }

    #[test]
    fn test_parse_directive_elseif() {
        let result = TemplateEngine::parse_directive("${{ elseif eq(parameters.x, 'y') }}");
        assert!(result.is_some());
        if let Some(TemplateDirective::ElseIf(condition)) = result {
            assert_eq!(condition, "eq(parameters.x, 'y')");
        } else {
            panic!("expected ElseIf directive");
        }
    }

    #[test]
    fn test_parse_directive_else_if() {
        let result = TemplateEngine::parse_directive("${{ else if ne(parameters.a, 'b') }}");
        assert!(result.is_some());
        if let Some(TemplateDirective::ElseIf(condition)) = result {
            assert_eq!(condition, "ne(parameters.a, 'b')");
        } else {
            panic!("expected ElseIf directive");
        }
    }

    #[test]
    fn test_parse_directive_else() {
        let result = TemplateEngine::parse_directive("${{ else }}");
        assert!(result.is_some());
        assert!(matches!(result, Some(TemplateDirective::Else)));
    }

    #[test]
    fn test_parse_directive_each() {
        let result = TemplateEngine::parse_directive("${{ each env in parameters.environments }}");
        assert!(result.is_some());
        if let Some(TemplateDirective::Each(var, collection)) = result {
            assert_eq!(var, "env");
            assert_eq!(collection, "parameters.environments");
        } else {
            panic!("expected Each directive");
        }
    }

    #[test]
    fn test_parse_directive_not_a_directive() {
        assert!(TemplateEngine::parse_directive("regular string").is_none());
        assert!(TemplateEngine::parse_directive("${{ parameters.x }}").is_none());
        assert!(TemplateEngine::parse_directive("${{ }}").is_none());
    }

    #[test]
    fn test_each_directive_in_jobs() {
        let dir = setup_templates(&[(
            "jobs/deploy.yml",
            r#"
parameters:
  - name: environments
    type: object

jobs:
  - ${{ each env in parameters.environments }}:
    - job: Deploy_${{ env }}
      displayName: Deploy to ${{ env }}
      steps:
        - script: echo deploying to ${{ env }}
"#,
        )]);

        let mut engine = TemplateEngine::new(dir.path().to_path_buf());

        let pipeline = Pipeline {
            jobs: vec![Job {
                template: Some("jobs/deploy.yml".to_string()),
                parameters: {
                    let mut params = HashMap::new();
                    params.insert(
                        "environments".to_string(),
                        serde_yaml::Value::Sequence(vec![
                            serde_yaml::Value::String("dev".to_string()),
                            serde_yaml::Value::String("staging".to_string()),
                        ]),
                    );
                    params
                },
                ..Default::default()
            }],
            ..Default::default()
        };

        let resolved = engine.resolve_pipeline(pipeline).unwrap();
        assert_eq!(resolved.jobs.len(), 2);
        assert_eq!(resolved.jobs[0].job, Some("Deploy_dev".to_string()));
        assert_eq!(
            resolved.jobs[0].display_name.as_deref(),
            Some("Deploy to dev")
        );
        assert_eq!(resolved.jobs[1].job, Some("Deploy_staging".to_string()));
        assert_eq!(
            resolved.jobs[1].display_name.as_deref(),
            Some("Deploy to staging")
        );
    }

    #[test]
    fn test_if_elseif_else_chain_first_branch() {
        // When the if condition is true, elseif and else should be skipped
        let dir = setup_templates(&[(
            "steps/deploy.yml",
            r#"
parameters:
  - name: environment
    type: string

steps:
  - ${{ if eq(parameters.environment, 'production') }}:
    - script: echo deploying to production
  - ${{ elseif eq(parameters.environment, 'staging') }}:
    - script: echo deploying to staging
  - ${{ else }}:
    - script: echo deploying to dev
"#,
        )]);

        let mut engine = TemplateEngine::new(dir.path().to_path_buf());

        let pipeline = Pipeline {
            steps: vec![Step {
                name: None,
                display_name: None,
                condition: None,
                continue_on_error: BoolOrExpression::default(),
                enabled: true,
                timeout_in_minutes: None,
                retry_count_on_task_failure: None,
                env: HashMap::new(),
                action: StepAction::Template(TemplateStep {
                    template: "steps/deploy.yml".to_string(),
                    parameters: {
                        let mut params = HashMap::new();
                        params.insert(
                            "environment".to_string(),
                            serde_yaml::Value::String("production".to_string()),
                        );
                        params
                    },
                }),
            }],
            ..Default::default()
        };

        let resolved = engine.resolve_pipeline(pipeline).unwrap();
        assert_eq!(resolved.steps.len(), 1);
        if let StepAction::Script(script) = &resolved.steps[0].action {
            assert_eq!(script.script, "echo deploying to production");
        } else {
            panic!("Expected script step");
        }
    }

    #[test]
    fn test_if_elseif_else_chain_second_branch() {
        // When if is false but elseif is true, only elseif body should be included
        let dir = setup_templates(&[(
            "steps/deploy.yml",
            r#"
parameters:
  - name: environment
    type: string

steps:
  - ${{ if eq(parameters.environment, 'production') }}:
    - script: echo deploying to production
  - ${{ elseif eq(parameters.environment, 'staging') }}:
    - script: echo deploying to staging
  - ${{ else }}:
    - script: echo deploying to dev
"#,
        )]);

        let mut engine = TemplateEngine::new(dir.path().to_path_buf());

        let pipeline = Pipeline {
            steps: vec![Step {
                name: None,
                display_name: None,
                condition: None,
                continue_on_error: BoolOrExpression::default(),
                enabled: true,
                timeout_in_minutes: None,
                retry_count_on_task_failure: None,
                env: HashMap::new(),
                action: StepAction::Template(TemplateStep {
                    template: "steps/deploy.yml".to_string(),
                    parameters: {
                        let mut params = HashMap::new();
                        params.insert(
                            "environment".to_string(),
                            serde_yaml::Value::String("staging".to_string()),
                        );
                        params
                    },
                }),
            }],
            ..Default::default()
        };

        let resolved = engine.resolve_pipeline(pipeline).unwrap();
        assert_eq!(resolved.steps.len(), 1);
        if let StepAction::Script(script) = &resolved.steps[0].action {
            assert_eq!(script.script, "echo deploying to staging");
        } else {
            panic!("Expected script step");
        }
    }

    #[test]
    fn test_if_elseif_else_chain_else_branch() {
        // When both if and elseif are false, else body should be included
        let dir = setup_templates(&[(
            "steps/deploy.yml",
            r#"
parameters:
  - name: environment
    type: string

steps:
  - ${{ if eq(parameters.environment, 'production') }}:
    - script: echo deploying to production
  - ${{ elseif eq(parameters.environment, 'staging') }}:
    - script: echo deploying to staging
  - ${{ else }}:
    - script: echo deploying to dev
"#,
        )]);

        let mut engine = TemplateEngine::new(dir.path().to_path_buf());

        let pipeline = Pipeline {
            steps: vec![Step {
                name: None,
                display_name: None,
                condition: None,
                continue_on_error: BoolOrExpression::default(),
                enabled: true,
                timeout_in_minutes: None,
                retry_count_on_task_failure: None,
                env: HashMap::new(),
                action: StepAction::Template(TemplateStep {
                    template: "steps/deploy.yml".to_string(),
                    parameters: {
                        let mut params = HashMap::new();
                        params.insert(
                            "environment".to_string(),
                            serde_yaml::Value::String("development".to_string()),
                        );
                        params
                    },
                }),
            }],
            ..Default::default()
        };

        let resolved = engine.resolve_pipeline(pipeline).unwrap();
        assert_eq!(resolved.steps.len(), 1);
        if let StepAction::Script(script) = &resolved.steps[0].action {
            assert_eq!(script.script, "echo deploying to dev");
        } else {
            panic!("Expected script step");
        }
    }

    #[test]
    fn test_if_elseif_chain_multiple_elseif() {
        // Multiple elseif branches: only the first matching one should be taken
        let dir = setup_templates(&[(
            "steps/config.yml",
            r#"
parameters:
  - name: os
    type: string

steps:
  - ${{ if eq(parameters.os, 'linux') }}:
    - script: echo linux setup
  - ${{ elseif eq(parameters.os, 'macos') }}:
    - script: echo macos setup
  - ${{ elseif eq(parameters.os, 'windows') }}:
    - script: echo windows setup
  - ${{ else }}:
    - script: echo unknown os
"#,
        )]);

        let mut engine = TemplateEngine::new(dir.path().to_path_buf());

        // Test that 'windows' matches third branch only
        let pipeline = Pipeline {
            steps: vec![Step {
                name: None,
                display_name: None,
                condition: None,
                continue_on_error: BoolOrExpression::default(),
                enabled: true,
                timeout_in_minutes: None,
                retry_count_on_task_failure: None,
                env: HashMap::new(),
                action: StepAction::Template(TemplateStep {
                    template: "steps/config.yml".to_string(),
                    parameters: {
                        let mut params = HashMap::new();
                        params.insert(
                            "os".to_string(),
                            serde_yaml::Value::String("windows".to_string()),
                        );
                        params
                    },
                }),
            }],
            ..Default::default()
        };

        let resolved = engine.resolve_pipeline(pipeline).unwrap();
        assert_eq!(resolved.steps.len(), 1);
        if let StepAction::Script(script) = &resolved.steps[0].action {
            assert_eq!(script.script, "echo windows setup");
        } else {
            panic!("Expected script step");
        }
    }

    #[test]
    fn test_if_chain_non_directive_breaks_chain() {
        // A non-directive item between if and else should break the chain,
        // so the else is treated as a standalone (always included)
        let dir = setup_templates(&[(
            "steps/broken.yml",
            r#"
parameters:
  - name: flag
    type: boolean

steps:
  - ${{ if eq(parameters.flag, true) }}:
    - script: echo flag is true
  - script: echo always runs
  - ${{ else }}:
    - script: echo flag is false
"#,
        )]);

        let mut engine = TemplateEngine::new(dir.path().to_path_buf());

        let pipeline = Pipeline {
            steps: vec![Step {
                name: None,
                display_name: None,
                condition: None,
                continue_on_error: BoolOrExpression::default(),
                enabled: true,
                timeout_in_minutes: None,
                retry_count_on_task_failure: None,
                env: HashMap::new(),
                action: StepAction::Template(TemplateStep {
                    template: "steps/broken.yml".to_string(),
                    parameters: {
                        let mut params = HashMap::new();
                        params.insert("flag".to_string(), serde_yaml::Value::Bool(true));
                        params
                    },
                }),
            }],
            ..Default::default()
        };

        let resolved = engine.resolve_pipeline(pipeline).unwrap();
        // if(true) + always runs + else (treated as standalone since chain was broken)
        assert_eq!(resolved.steps.len(), 3);
        if let StepAction::Script(script) = &resolved.steps[0].action {
            assert_eq!(script.script, "echo flag is true");
        } else {
            panic!("Expected script step");
        }
        if let StepAction::Script(script) = &resolved.steps[1].action {
            assert_eq!(script.script, "echo always runs");
        } else {
            panic!("Expected script step");
        }
        if let StepAction::Script(script) = &resolved.steps[2].action {
            assert_eq!(script.script, "echo flag is false");
        } else {
            panic!("Expected script step");
        }
    }
}
