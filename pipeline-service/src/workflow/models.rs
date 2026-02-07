use serde::{Deserialize, Serialize};
use serde_json::Value;

use std::collections::HashMap;

/// A GitHub Actions-compatible workflow definition.
///
/// This represents the top-level structure of a workflow YAML file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    /// The name of the workflow (displayed in GitHub Actions UI)
    pub name: Option<String>,

    /// The trigger configuration for the workflow
    #[serde(rename = "on")]
    pub on: Trigger,

    /// Workflow-level environment variables
    #[serde(default)]
    pub env: HashMap<String, String>,

    /// Default settings for all jobs in the workflow
    #[serde(default)]
    pub defaults: Option<Defaults>,

    /// The jobs that make up this workflow
    pub jobs: HashMap<String, Job>,

    /// Permissions for the GITHUB_TOKEN (parsed but not enforced locally)
    #[serde(default)]
    pub permissions: Option<Permissions>,

    /// Concurrency settings (parsed but not enforced locally)
    #[serde(default)]
    pub concurrency: Option<Concurrency>,
}

/// Trigger configuration for when the workflow should run.
///
/// Supports multiple trigger formats:
/// - Simple: `on: push`
/// - List: `on: [push, pull_request]`
/// - Detailed: `on: { push: { branches: [main] } }`
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Trigger {
    /// Single event trigger: `on: push`
    Single(String),

    /// Multiple events: `on: [push, pull_request]`
    Multiple(Vec<String>),

    /// Detailed event configuration
    Detailed(HashMap<String, Option<EventConfig>>),
}

/// Configuration for a specific trigger event.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EventConfig {
    /// Branch filters for push/pull_request events
    #[serde(default)]
    pub branches: Vec<String>,

    /// Branch ignore patterns
    #[serde(default, rename = "branches-ignore")]
    pub branches_ignore: Vec<String>,

    /// Tag filters for push events
    #[serde(default)]
    pub tags: Vec<String>,

    /// Tag ignore patterns
    #[serde(default, rename = "tags-ignore")]
    pub tags_ignore: Vec<String>,

    /// Path filters
    #[serde(default)]
    pub paths: Vec<String>,

    /// Path ignore patterns
    #[serde(default, rename = "paths-ignore")]
    pub paths_ignore: Vec<String>,

    /// Event types for events like issues, pull_request_review, etc.
    #[serde(default)]
    pub types: Vec<String>,

    /// Cron schedules for schedule events
    #[serde(default)]
    pub cron: Option<String>,

    /// Inputs for workflow_dispatch
    #[serde(default)]
    pub inputs: HashMap<String, WorkflowInput>,

    /// Outputs for workflow_call
    #[serde(default)]
    pub outputs: HashMap<String, WorkflowOutput>,

    /// Secrets for workflow_call
    #[serde(default)]
    pub secrets: HashMap<String, WorkflowSecret>,
}

/// Input definition for workflow_dispatch or workflow_call triggers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowInput {
    /// Description of the input
    #[serde(default)]
    pub description: Option<String>,

    /// Whether the input is required
    #[serde(default)]
    pub required: bool,

    /// Default value for the input
    #[serde(default)]
    pub default: Option<Value>,

    /// Input type (string, boolean, choice, environment)
    #[serde(default, rename = "type")]
    pub input_type: Option<String>,

    /// Options for choice type inputs
    #[serde(default)]
    pub options: Vec<String>,
}

/// Output definition for workflow_call triggers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowOutput {
    /// Description of the output
    #[serde(default)]
    pub description: Option<String>,

    /// Value expression for the output
    pub value: String,
}

/// Secret definition for workflow_call triggers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowSecret {
    /// Description of the secret
    #[serde(default)]
    pub description: Option<String>,

    /// Whether the secret is required
    #[serde(default)]
    pub required: bool,
}

/// Default settings for jobs and steps.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Defaults {
    /// Default settings for run steps
    #[serde(default)]
    pub run: Option<RunDefaults>,
}

/// Default settings for run steps.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RunDefaults {
    /// Default shell to use
    #[serde(default)]
    pub shell: Option<String>,

    /// Default working directory
    #[serde(default, rename = "working-directory")]
    pub working_directory: Option<String>,
}

/// Permissions configuration for GITHUB_TOKEN.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Permissions {
    /// Read-all or write-all
    Level(String),

    /// Granular permissions
    Granular(HashMap<String, String>),
}

/// Concurrency settings to limit workflow runs.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Concurrency {
    /// Simple concurrency group name
    Simple(String),

    /// Detailed concurrency configuration
    Detailed {
        group: String,
        #[serde(default, rename = "cancel-in-progress")]
        cancel_in_progress: bool,
    },
}

/// A job within a workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    /// Display name for the job
    #[serde(default)]
    pub name: Option<String>,

    /// Jobs that must complete before this job runs
    #[serde(default)]
    pub needs: JobNeeds,

    /// Runner label (parsed but ignored locally - always runs locally)
    #[serde(default, rename = "runs-on")]
    pub runs_on: Option<RunsOn>,

    /// Conditional expression for job execution
    #[serde(default, rename = "if")]
    pub if_condition: Option<String>,

    /// Job-level environment variables
    #[serde(default)]
    pub env: HashMap<String, String>,

    /// Default settings for steps in this job
    #[serde(default)]
    pub defaults: Option<Defaults>,

    /// Job outputs to pass to dependent jobs
    #[serde(default)]
    pub outputs: HashMap<String, String>,

    /// Matrix strategy for running multiple job instances
    #[serde(default)]
    pub strategy: Option<Strategy>,

    /// The steps that make up this job
    #[serde(default)]
    pub steps: Vec<Step>,

    /// Service containers for the job
    #[serde(default)]
    pub services: HashMap<String, Service>,

    /// Container to run the job in
    #[serde(default)]
    pub container: Option<Container>,

    /// Job timeout in minutes
    #[serde(default, rename = "timeout-minutes")]
    pub timeout_minutes: Option<u32>,

    /// Whether to continue workflow if this job fails
    #[serde(default, rename = "continue-on-error")]
    pub continue_on_error: ContinueOnError,

    /// Permissions for this job's GITHUB_TOKEN
    #[serde(default)]
    pub permissions: Option<Permissions>,

    /// Concurrency settings for this job
    #[serde(default)]
    pub concurrency: Option<Concurrency>,

    /// Environment deployment target
    #[serde(default)]
    pub environment: Option<Environment>,
}

/// Job dependencies - can be a single string or a list.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(untagged)]
pub enum JobNeeds {
    #[default]
    None,
    Single(String),
    Multiple(Vec<String>),
}

impl JobNeeds {
    /// Convert to a vector of job IDs.
    pub fn to_vec(&self) -> Vec<String> {
        match self {
            JobNeeds::None => vec![],
            JobNeeds::Single(s) => vec![s.clone()],
            JobNeeds::Multiple(v) => v.clone(),
        }
    }

    /// Check if there are any dependencies.
    pub fn is_empty(&self) -> bool {
        match self {
            JobNeeds::None => true,
            JobNeeds::Single(_) => false,
            JobNeeds::Multiple(v) => v.is_empty(),
        }
    }
}

/// Runner specification - can be a string or a list of labels.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RunsOn {
    /// Single runner label: `runs-on: ubuntu-latest`
    Label(String),

    /// Multiple labels: `runs-on: [self-hosted, linux]`
    Labels(Vec<String>),

    /// Expression: `runs-on: ${{ matrix.os }}`
    Expression(String),
}

/// Continue-on-error setting - can be a boolean or an expression.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ContinueOnError {
    Bool(bool),
    Expression(String),
}

impl Default for ContinueOnError {
    fn default() -> Self {
        ContinueOnError::Bool(false)
    }
}

/// Strategy configuration for matrix builds.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Strategy {
    /// Matrix configuration
    #[serde(default)]
    pub matrix: Option<Matrix>,

    /// Whether to cancel all jobs if one fails
    #[serde(default = "default_fail_fast", rename = "fail-fast")]
    pub fail_fast: bool,

    /// Maximum number of jobs to run in parallel
    #[serde(default, rename = "max-parallel")]
    pub max_parallel: Option<u32>,
}

fn default_fail_fast() -> bool {
    true
}

/// Matrix configuration for parallel job execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Matrix {
    /// Matrix dimensions (dynamic keys)
    #[serde(flatten)]
    pub dimensions: HashMap<String, Vec<Value>>,

    /// Additional matrix combinations to include
    #[serde(default)]
    pub include: Vec<HashMap<String, Value>>,

    /// Matrix combinations to exclude
    #[serde(default)]
    pub exclude: Vec<HashMap<String, Value>>,
}

/// A step within a job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step {
    /// Unique identifier for the step (used in outputs)
    #[serde(default)]
    pub id: Option<String>,

    /// Display name for the step
    #[serde(default)]
    pub name: Option<String>,

    /// Conditional expression for step execution
    #[serde(default, rename = "if")]
    pub if_condition: Option<String>,

    /// Shell command to run
    #[serde(default)]
    pub run: Option<String>,

    /// Shell to use for the run command
    #[serde(default)]
    pub shell: Option<String>,

    /// Working directory for the step
    #[serde(default, rename = "working-directory")]
    pub working_directory: Option<String>,

    /// Action to use (e.g., "actions/checkout@v4")
    #[serde(default)]
    pub uses: Option<String>,

    /// Inputs to pass to the action
    #[serde(default)]
    pub with: HashMap<String, Value>,

    /// Step-level environment variables
    #[serde(default)]
    pub env: HashMap<String, String>,

    /// Whether to continue job if this step fails
    #[serde(default, rename = "continue-on-error")]
    pub continue_on_error: bool,

    /// Step timeout in minutes
    #[serde(default, rename = "timeout-minutes")]
    pub timeout_minutes: Option<u32>,
}

impl Step {
    /// Get a display name for the step.
    pub fn display_name(&self) -> String {
        if let Some(name) = &self.name {
            name.clone()
        } else if let Some(uses) = &self.uses {
            format!("Run {}", uses)
        } else if let Some(run) = &self.run {
            // Truncate long commands
            let first_line = run.lines().next().unwrap_or(run);
            if first_line.len() > 50 {
                format!("{}...", &first_line[..47])
            } else {
                format!("Run {}", first_line)
            }
        } else {
            "Unnamed step".to_string()
        }
    }

    /// Check if this is a run step.
    pub fn is_run(&self) -> bool {
        self.run.is_some()
    }

    /// Check if this is a uses step.
    pub fn is_uses(&self) -> bool {
        self.uses.is_some()
    }
}

/// Service container definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Service {
    /// Docker image to use
    pub image: String,

    /// Environment variables for the container
    #[serde(default)]
    pub env: HashMap<String, String>,

    /// Port mappings (host:container)
    #[serde(default)]
    pub ports: Vec<String>,

    /// Volume mounts
    #[serde(default)]
    pub volumes: Vec<String>,

    /// Additional docker options
    #[serde(default)]
    pub options: Option<String>,

    /// Credentials for private registries
    #[serde(default)]
    pub credentials: Option<ContainerCredentials>,
}

/// Container configuration for running a job.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Container {
    /// Simple image name
    Image(String),

    /// Detailed container configuration
    Detailed(ContainerConfig),
}

/// Detailed container configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerConfig {
    /// Docker image to use
    pub image: String,

    /// Environment variables for the container
    #[serde(default)]
    pub env: HashMap<String, String>,

    /// Port mappings
    #[serde(default)]
    pub ports: Vec<String>,

    /// Volume mounts
    #[serde(default)]
    pub volumes: Vec<String>,

    /// Additional docker options
    #[serde(default)]
    pub options: Option<String>,

    /// Credentials for private registries
    #[serde(default)]
    pub credentials: Option<ContainerCredentials>,
}

/// Credentials for container registries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerCredentials {
    /// Username for the registry
    pub username: String,

    /// Password for the registry
    pub password: String,
}

/// Environment deployment target.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Environment {
    /// Simple environment name
    Name(String),

    /// Detailed environment configuration
    Detailed {
        name: String,
        #[serde(default)]
        url: Option<String>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_workflow() {
        let yaml = r#"
name: CI
on: push
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - run: echo "Hello, World!"
"#;
        let workflow: Workflow = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(workflow.name, Some("CI".to_string()));
        assert!(matches!(workflow.on, Trigger::Single(ref s) if s == "push"));
        assert!(workflow.jobs.contains_key("build"));
    }

    #[test]
    fn test_parse_workflow_with_multiple_triggers() {
        let yaml = r#"
name: CI
on: [push, pull_request]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - run: cargo test
"#;
        let workflow: Workflow = serde_yaml::from_str(yaml).unwrap();
        assert!(matches!(workflow.on, Trigger::Multiple(ref v) if v.len() == 2));
    }

    #[test]
    fn test_parse_workflow_with_detailed_triggers() {
        let yaml = r#"
name: CI
on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main]
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - run: echo "Building"
"#;
        let workflow: Workflow = serde_yaml::from_str(yaml).unwrap();
        if let Trigger::Detailed(events) = &workflow.on {
            assert!(events.contains_key("push"));
            assert!(events.contains_key("pull_request"));
        } else {
            panic!("Expected detailed trigger");
        }
    }

    #[test]
    fn test_parse_job_with_needs() {
        let yaml = r#"
name: CI
on: push
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - run: cargo build
  test:
    needs: build
    runs-on: ubuntu-latest
    steps:
      - run: cargo test
  deploy:
    needs: [build, test]
    runs-on: ubuntu-latest
    steps:
      - run: echo "Deploying"
"#;
        let workflow: Workflow = serde_yaml::from_str(yaml).unwrap();

        let build = workflow.jobs.get("build").unwrap();
        assert!(build.needs.is_empty());

        let test = workflow.jobs.get("test").unwrap();
        assert_eq!(test.needs.to_vec(), vec!["build"]);

        let deploy = workflow.jobs.get("deploy").unwrap();
        assert_eq!(deploy.needs.to_vec(), vec!["build", "test"]);
    }

    #[test]
    fn test_parse_matrix_strategy() {
        let yaml = r#"
name: Matrix CI
on: push
jobs:
  test:
    runs-on: ubuntu-latest
    strategy:
      matrix:
        node: [16, 18, 20]
        os: [ubuntu-latest, macos-latest]
    steps:
      - run: echo "Testing"
"#;
        let workflow: Workflow = serde_yaml::from_str(yaml).unwrap();
        let job = workflow.jobs.get("test").unwrap();
        let strategy = job.strategy.as_ref().unwrap();
        let matrix = strategy.matrix.as_ref().unwrap();

        assert!(matrix.dimensions.contains_key("node"));
        assert!(matrix.dimensions.contains_key("os"));
    }

    #[test]
    fn test_parse_step_with_uses() {
        let yaml = r#"
name: CI
on: push
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: '20'
      - run: npm test
"#;
        let workflow: Workflow = serde_yaml::from_str(yaml).unwrap();
        let job = workflow.jobs.get("build").unwrap();

        assert!(job.steps[0].is_uses());
        assert_eq!(job.steps[0].uses, Some("actions/checkout@v4".to_string()));

        assert!(job.steps[1].is_uses());
        assert!(job.steps[1].with.contains_key("node-version"));

        assert!(job.steps[2].is_run());
    }

    #[test]
    fn test_parse_step_with_if_condition() {
        let yaml = r#"
name: CI
on: push
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - run: echo "Always runs"
      - run: echo "Only on main"
        if: github.ref == 'refs/heads/main'
      - run: echo "On failure"
        if: failure()
"#;
        let workflow: Workflow = serde_yaml::from_str(yaml).unwrap();
        let job = workflow.jobs.get("build").unwrap();

        assert!(job.steps[0].if_condition.is_none());
        assert_eq!(
            job.steps[1].if_condition,
            Some("github.ref == 'refs/heads/main'".to_string())
        );
        assert_eq!(job.steps[2].if_condition, Some("failure()".to_string()));
    }

    #[test]
    fn test_parse_job_outputs() {
        let yaml = r#"
name: CI
on: push
jobs:
  build:
    runs-on: ubuntu-latest
    outputs:
      version: ${{ steps.version.outputs.version }}
    steps:
      - id: version
        run: echo "version=1.0.0" >> $GITHUB_OUTPUT
"#;
        let workflow: Workflow = serde_yaml::from_str(yaml).unwrap();
        let job = workflow.jobs.get("build").unwrap();

        assert!(job.outputs.contains_key("version"));
        assert_eq!(job.steps[0].id, Some("version".to_string()));
    }

    #[test]
    fn test_parse_services() {
        let yaml = r#"
name: CI
on: push
jobs:
  test:
    runs-on: ubuntu-latest
    services:
      postgres:
        image: postgres:15
        env:
          POSTGRES_PASSWORD: postgres
        ports:
          - 5432:5432
    steps:
      - run: echo "Testing with postgres"
"#;
        let workflow: Workflow = serde_yaml::from_str(yaml).unwrap();
        let job = workflow.jobs.get("test").unwrap();

        let postgres = job.services.get("postgres").unwrap();
        assert_eq!(postgres.image, "postgres:15");
        assert!(postgres.env.contains_key("POSTGRES_PASSWORD"));
    }

    #[test]
    fn test_parse_env_at_all_levels() {
        let yaml = r#"
name: CI
on: push
env:
  WORKFLOW_VAR: workflow
jobs:
  build:
    runs-on: ubuntu-latest
    env:
      JOB_VAR: job
    steps:
      - run: echo "Hello"
        env:
          STEP_VAR: step
"#;
        let workflow: Workflow = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(
            workflow.env.get("WORKFLOW_VAR"),
            Some(&"workflow".to_string())
        );

        let job = workflow.jobs.get("build").unwrap();
        assert_eq!(job.env.get("JOB_VAR"), Some(&"job".to_string()));
        assert_eq!(job.steps[0].env.get("STEP_VAR"), Some(&"step".to_string()));
    }

    #[test]
    fn test_step_display_name() {
        let step_with_name = Step {
            id: None,
            name: Some("Build project".to_string()),
            if_condition: None,
            run: Some("cargo build".to_string()),
            shell: None,
            working_directory: None,
            uses: None,
            with: HashMap::new(),
            env: HashMap::new(),
            continue_on_error: false,
            timeout_minutes: None,
        };
        assert_eq!(step_with_name.display_name(), "Build project");

        let step_with_uses = Step {
            id: None,
            name: None,
            if_condition: None,
            run: None,
            shell: None,
            working_directory: None,
            uses: Some("actions/checkout@v4".to_string()),
            with: HashMap::new(),
            env: HashMap::new(),
            continue_on_error: false,
            timeout_minutes: None,
        };
        assert_eq!(step_with_uses.display_name(), "Run actions/checkout@v4");

        let step_with_run = Step {
            id: None,
            name: None,
            if_condition: None,
            run: Some("echo hello".to_string()),
            shell: None,
            working_directory: None,
            uses: None,
            with: HashMap::new(),
            env: HashMap::new(),
            continue_on_error: false,
            timeout_minutes: None,
        };
        assert_eq!(step_with_run.display_name(), "Run echo hello");
    }

    #[test]
    fn test_job_needs_to_vec() {
        assert!(JobNeeds::None.to_vec().is_empty());
        assert_eq!(
            JobNeeds::Single("build".to_string()).to_vec(),
            vec!["build"]
        );
        assert_eq!(
            JobNeeds::Multiple(vec!["build".to_string(), "test".to_string()]).to_vec(),
            vec!["build", "test"]
        );
    }
}
