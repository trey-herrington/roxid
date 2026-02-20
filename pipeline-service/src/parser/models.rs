// Azure DevOps Pipeline Data Models
// Comprehensive types representing the full Azure DevOps YAML schema

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// A value that can be either a boolean literal or a runtime expression string.
/// Azure DevOps allows fields like `continueOnError` to use runtime expressions
/// such as `$[eq(variables.rustToolchain, 'nightly')]`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum BoolOrExpression {
    Bool(bool),
    Expression(String),
}

impl Default for BoolOrExpression {
    fn default() -> Self {
        BoolOrExpression::Bool(false)
    }
}

impl BoolOrExpression {
    /// Returns the boolean value if this is a literal bool, or false for expressions
    /// (expressions must be evaluated at runtime).
    pub fn as_bool(&self) -> bool {
        match self {
            BoolOrExpression::Bool(b) => *b,
            BoolOrExpression::Expression(_) => false,
        }
    }
}

/// Root pipeline structure supporting all Azure DevOps formats
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Pipeline {
    /// Pipeline name
    pub name: Option<String>,

    /// CI trigger configuration
    pub trigger: Option<Trigger>,

    /// PR trigger configuration
    pub pr: Option<PrTrigger>,

    /// Scheduled triggers
    pub schedules: Option<Vec<Schedule>>,

    /// Resource definitions (repositories, containers, pipelines, packages)
    pub resources: Option<Resources>,

    /// Pipeline-level variables
    #[serde(default, deserialize_with = "deserialize_variables")]
    pub variables: Vec<Variable>,

    /// Pipeline parameters (template inputs)
    #[serde(default)]
    pub parameters: Vec<Parameter>,

    /// Full pipeline structure with stages
    #[serde(default, deserialize_with = "deserialize_tolerant_vec")]
    pub stages: Vec<Stage>,

    /// Shorthand: jobs without stages
    #[serde(default, deserialize_with = "deserialize_tolerant_vec")]
    pub jobs: Vec<Job>,

    /// Shorthand: steps without stages/jobs
    #[serde(default, deserialize_with = "deserialize_tolerant_vec")]
    pub steps: Vec<Step>,

    /// Default pool for all jobs
    pub pool: Option<Pool>,

    /// Template extension
    pub extends: Option<Extends>,

    /// Lock behavior for resources
    #[serde(rename = "lockBehavior")]
    pub lock_behavior: Option<LockBehavior>,

    /// Whether stages/jobs/steps lists contained compile-time template directives
    /// (${{ if }}, ${{ each }}) that were dropped during deserialization.
    #[serde(skip)]
    pub has_template_directives: bool,
}

// =============================================================================
// Triggers
// =============================================================================

/// CI trigger configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Trigger {
    /// Simple: trigger: none
    None,
    /// Branches list
    Branches(Vec<String>),
    /// Full configuration
    Full(TriggerConfig),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TriggerConfig {
    pub batch: Option<bool>,
    pub branches: Option<BranchFilter>,
    pub paths: Option<PathFilter>,
    pub tags: Option<TagFilter>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BranchFilter {
    #[serde(default)]
    pub include: Vec<String>,
    #[serde(default)]
    pub exclude: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PathFilter {
    #[serde(default)]
    pub include: Vec<String>,
    #[serde(default)]
    pub exclude: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TagFilter {
    #[serde(default)]
    pub include: Vec<String>,
    #[serde(default)]
    pub exclude: Vec<String>,
}

/// PR trigger configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PrTrigger {
    /// Simple: pr: none
    None,
    /// Branches list
    Branches(Vec<String>),
    /// Full configuration
    Full(PrTriggerConfig),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PrTriggerConfig {
    pub auto_cancel: Option<bool>,
    pub branches: Option<BranchFilter>,
    pub paths: Option<PathFilter>,
    pub drafts: Option<bool>,
}

/// Scheduled trigger
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Schedule {
    pub cron: String,
    pub display_name: Option<String>,
    pub branches: Option<BranchFilter>,
    #[serde(default = "default_true")]
    pub always: bool,
    #[serde(default)]
    pub batch: bool,
}

fn default_true() -> bool {
    true
}

// =============================================================================
// Resources
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Resources {
    #[serde(default)]
    pub repositories: Vec<RepositoryResource>,
    #[serde(default)]
    pub containers: Vec<ContainerResource>,
    #[serde(default)]
    pub pipelines: Vec<PipelineResource>,
    #[serde(default)]
    pub packages: Vec<PackageResource>,
    #[serde(default)]
    pub webhooks: Vec<WebhookResource>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryResource {
    pub repository: String,
    #[serde(rename = "type")]
    pub repo_type: Option<String>,
    pub name: Option<String>,
    #[serde(rename = "ref")]
    pub git_ref: Option<String>,
    pub endpoint: Option<String>,
    pub trigger: Option<Trigger>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerResource {
    pub container: String,
    pub image: String,
    pub endpoint: Option<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default)]
    pub ports: Vec<String>,
    #[serde(default)]
    pub volumes: Vec<String>,
    pub options: Option<String>,
    #[serde(rename = "mapDockerSocket")]
    pub map_docker_socket: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelineResource {
    pub pipeline: String,
    pub source: String,
    pub project: Option<String>,
    pub trigger: Option<PipelineResourceTrigger>,
    pub version: Option<String>,
    pub branch: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineResourceTrigger {
    pub enabled: Option<bool>,
    pub branches: Option<BranchFilter>,
    pub stages: Option<Vec<String>>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageResource {
    pub package: String,
    #[serde(rename = "type")]
    pub package_type: String,
    pub connection: String,
    pub name: String,
    pub version: Option<String>,
    pub tag: Option<String>,
    pub trigger: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookResource {
    pub webhook: String,
    pub connection: String,
    #[serde(rename = "type")]
    pub webhook_type: Option<String>,
    pub filters: Option<Vec<WebhookFilter>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookFilter {
    pub path: String,
    pub value: String,
}

// =============================================================================
// Variables
// =============================================================================

/// Variable can be a simple key-value, a group reference, or a template
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Variable {
    /// Simple variable: { name: foo, value: bar }
    KeyValue {
        name: String,
        value: String,
        #[serde(default)]
        readonly: bool,
    },
    /// Variable group reference: { group: my-group }
    Group { group: String },
    /// Template reference: { template: vars.yml }
    Template {
        template: String,
        #[serde(default)]
        parameters: HashMap<String, serde_yaml::Value>,
    },
}

/// Custom deserializer for variables supporting both map and list formats
fn deserialize_variables<'de, D>(deserializer: D) -> Result<Vec<Variable>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::{MapAccess, SeqAccess, Visitor};

    struct VariablesVisitor;

    impl<'de> Visitor<'de> for VariablesVisitor {
        type Value = Vec<Variable>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a map of variables or a list of variable definitions")
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: SeqAccess<'de>,
        {
            let mut vars = Vec::new();
            while let Some(var) = seq.next_element::<Variable>()? {
                vars.push(var);
            }
            Ok(vars)
        }

        fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
        where
            M: MapAccess<'de>,
        {
            let mut vars = Vec::new();
            while let Some((key, value)) = map.next_entry::<String, String>()? {
                vars.push(Variable::KeyValue {
                    name: key,
                    value,
                    readonly: false,
                });
            }
            Ok(vars)
        }
    }

    deserializer.deserialize_any(VariablesVisitor)
}

/// Tolerant deserializer for Vec<T> that skips items which fail to deserialize.
/// This handles Azure DevOps template expressions like `${{ if ... }}:` and
/// `${{ each ... }}:` that appear as list items but cannot be deserialized into
/// typed structs. These are compile-time template directives that should be
/// preprocessed but may appear in raw YAML discovery.
fn deserialize_tolerant_vec<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::de::DeserializeOwned,
{
    use serde::de::{SeqAccess, Visitor};

    struct TolerantVecVisitor<T>(std::marker::PhantomData<T>);

    impl<'de, T: serde::de::DeserializeOwned> Visitor<'de> for TolerantVecVisitor<T> {
        type Value = Vec<T>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a sequence")
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: SeqAccess<'de>,
        {
            let mut items = Vec::new();
            // Try to deserialize each element; use serde_yaml::Value as a fallback
            // to consume items that fail typed deserialization (e.g., template directives)
            while let Some(value) = seq.next_element::<serde_yaml::Value>()? {
                // Skip compile-time template directives (${{ if }}, ${{ each }}, etc.)
                // These appear as mappings with a ${{ ... }} key and must not be
                // deserialized into typed structs (which would create phantom entries
                // with empty/default fields).
                if is_template_directive(&value) {
                    continue;
                }
                if let Ok(item) = serde_yaml::from_value::<T>(value) {
                    items.push(item);
                }
                // Silently skip items that fail to deserialize
            }
            Ok(items)
        }
    }

    deserializer.deserialize_seq(TolerantVecVisitor::<T>(std::marker::PhantomData))
}

/// Check whether a YAML value is a compile-time template directive.
/// Template directives appear as mappings with a key that starts with `${{`.
pub fn is_template_directive(value: &serde_yaml::Value) -> bool {
    if let Some(mapping) = value.as_mapping() {
        mapping.keys().any(|key| {
            key.as_str()
                .is_some_and(|s| s.trim_start().starts_with("${{"))
        })
    } else {
        false
    }
}

// =============================================================================
// Parameters
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Parameter {
    pub name: String,
    pub display_name: Option<String>,
    #[serde(rename = "type", default)]
    pub param_type: ParameterType,
    pub default: Option<serde_yaml::Value>,
    pub values: Option<Vec<serde_yaml::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ParameterType {
    #[default]
    String,
    Number,
    Boolean,
    Object,
    Step,
    StepList,
    Job,
    JobList,
    Stage,
    StageList,
}

// =============================================================================
// Pool
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Pool {
    /// Named pool: pool: my-pool
    Name(String),
    /// Full pool spec
    Full(PoolSpec),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PoolSpec {
    pub name: Option<String>,
    pub vm_image: Option<String>,
    pub demands: Option<PoolDemands>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PoolDemands {
    List(Vec<String>),
    Map(HashMap<String, String>),
}

// =============================================================================
// Stage
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Stage {
    /// Stage identifier
    pub stage: Option<String>,

    /// Display name in UI
    pub display_name: Option<String>,

    /// Dependency on other stages
    #[serde(default)]
    pub depends_on: DependsOn,

    /// Condition for running this stage
    pub condition: Option<String>,

    /// Stage-level variables
    #[serde(default, deserialize_with = "deserialize_variables")]
    pub variables: Vec<Variable>,

    /// Jobs in this stage
    #[serde(default, deserialize_with = "deserialize_tolerant_vec")]
    pub jobs: Vec<Job>,

    /// Lock behavior
    pub lock_behavior: Option<LockBehavior>,

    /// Template reference
    pub template: Option<String>,

    /// Template parameters
    #[serde(default)]
    pub parameters: HashMap<String, serde_yaml::Value>,

    /// Pool override for all jobs
    pub pool: Option<Pool>,

    /// Whether the jobs list contained compile-time template directives
    /// (${{ if }}, ${{ each }}) that were dropped during deserialization.
    /// When true, the validator should not require jobs to be non-empty.
    #[serde(skip)]
    pub has_template_directives: bool,
}

// =============================================================================
// Job
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Job {
    /// Job identifier (mutually exclusive with deployment)
    pub job: Option<String>,

    /// Deployment job identifier
    pub deployment: Option<String>,

    /// Display name in UI
    pub display_name: Option<String>,

    /// Dependency on other jobs
    #[serde(default)]
    pub depends_on: DependsOn,

    /// Condition for running this job
    pub condition: Option<String>,

    /// Execution strategy (matrix, parallel)
    pub strategy: Option<Strategy>,

    /// Agent pool for this job
    pub pool: Option<Pool>,

    /// Container to run the job in
    pub container: Option<ContainerRef>,

    /// Service containers
    #[serde(default)]
    pub services: HashMap<String, ContainerRef>,

    /// Job-level variables
    #[serde(default, deserialize_with = "deserialize_variables")]
    pub variables: Vec<Variable>,

    /// Steps to execute
    #[serde(default, deserialize_with = "deserialize_tolerant_vec")]
    pub steps: Vec<Step>,

    /// Job timeout
    pub timeout_in_minutes: Option<u32>,

    /// Cancel timeout
    pub cancel_timeout_in_minutes: Option<u32>,

    /// Continue pipeline on error
    #[serde(default)]
    pub continue_on_error: BoolOrExpression,

    /// Workspace settings
    pub workspace: Option<Workspace>,

    /// Uses statement (template reference)
    pub uses: Option<UsesSpec>,

    /// Template reference
    pub template: Option<String>,

    /// Template parameters
    #[serde(default)]
    pub parameters: HashMap<String, serde_yaml::Value>,

    /// Deployment environment (for deployment jobs)
    pub environment: Option<Environment>,

    /// Whether the steps list contained compile-time template directives
    /// (${{ if }}, ${{ each }}) that were dropped during deserialization.
    /// When true, the validator should not require steps to be non-empty.
    #[serde(skip)]
    pub has_template_directives: bool,
}

impl Job {
    /// Returns the job identifier (either job or deployment name)
    pub fn identifier(&self) -> Option<&str> {
        self.job.as_deref().or(self.deployment.as_deref())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ContainerRef {
    /// Simple image reference
    Image(String),
    /// Full container spec
    Spec(ContainerSpec),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContainerSpec {
    pub image: String,
    pub endpoint: Option<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default)]
    pub ports: Vec<String>,
    #[serde(default)]
    pub volumes: Vec<String>,
    pub options: Option<String>,
    pub map_docker_socket: Option<bool>,
    #[serde(rename = "mountReadOnly")]
    pub mount_read_only: Option<MountReadOnly>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MountReadOnly {
    pub work: Option<bool>,
    pub externals: Option<bool>,
    pub tools: Option<bool>,
    pub tasks: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    pub clean: Option<WorkspaceClean>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WorkspaceClean {
    Outputs,
    Resources,
    All,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsesSpec {
    pub repositories: Option<Vec<String>>,
    pub pools: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Environment {
    Name(String),
    Full(EnvironmentSpec),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentSpec {
    pub name: String,
    pub resource_name: Option<String>,
    pub resource_id: Option<u64>,
    pub resource_type: Option<String>,
    pub tags: Option<Vec<String>>,
}

// =============================================================================
// Strategy
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Strategy {
    /// Matrix strategy
    pub matrix: Option<MatrixStrategy>,

    /// Parallel jobs count
    pub parallel: Option<u32>,

    /// Maximum parallel jobs
    pub max_parallel: Option<u32>,

    /// Deployment strategy (runOnce, rolling, canary)
    pub run_once: Option<DeploymentHooks>,
    pub rolling: Option<RollingStrategy>,
    pub canary: Option<CanaryStrategy>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MatrixStrategy {
    /// Inline matrix definition
    Inline(HashMap<String, HashMap<String, serde_yaml::Value>>),
    /// Expression reference
    Expression(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentHooks {
    pub pre_deploy: Option<HookSteps>,
    pub deploy: Option<HookSteps>,
    pub route_traffic: Option<HookSteps>,
    pub post_route_traffic: Option<HookSteps>,
    pub on_failure: Option<HookSteps>,
    pub on_success: Option<HookSteps>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookSteps {
    pub pool: Option<Pool>,
    #[serde(default, deserialize_with = "deserialize_tolerant_vec")]
    pub steps: Vec<Step>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RollingStrategy {
    pub max_parallel: Option<u32>,
    #[serde(flatten)]
    pub hooks: DeploymentHooks,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CanaryStrategy {
    pub increments: Vec<u32>,
    #[serde(flatten)]
    pub hooks: DeploymentHooks,
}

// =============================================================================
// DependsOn
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(untagged)]
pub enum DependsOn {
    /// No dependencies (default: depends on previous)
    #[default]
    Default,
    /// Explicitly no dependencies
    None,
    /// Single dependency
    Single(String),
    /// Multiple dependencies
    Multiple(Vec<String>),
}

impl DependsOn {
    pub fn as_vec(&self) -> Vec<String> {
        match self {
            DependsOn::Default => vec![],
            DependsOn::None => vec![],
            DependsOn::Single(s) => vec![s.clone()],
            DependsOn::Multiple(v) => v.clone(),
        }
    }

    pub fn is_explicit_none(&self) -> bool {
        matches!(self, DependsOn::None)
    }
}

// =============================================================================
// Step
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Step {
    /// Step name for output references
    pub name: Option<String>,

    /// Display name in UI
    pub display_name: Option<String>,

    /// Condition for running this step
    pub condition: Option<String>,

    /// Continue job on step failure
    #[serde(default)]
    pub continue_on_error: BoolOrExpression,

    /// Enable/disable step
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Step timeout
    pub timeout_in_minutes: Option<u32>,

    /// Retry count on failure
    pub retry_count_on_task_failure: Option<u32>,

    /// Step-level environment variables
    #[serde(default)]
    pub env: HashMap<String, String>,

    /// The action to perform (flattened from different step types)
    #[serde(flatten)]
    pub action: StepAction,
}

/// The specific action a step performs
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum StepAction {
    /// Script step: - script: echo hello
    Script(ScriptStep),
    /// Bash step: - bash: echo hello
    Bash(BashStep),
    /// PowerShell Core step: - pwsh: Write-Host hello
    Pwsh(PwshStep),
    /// Windows PowerShell step: - powershell: Write-Host hello
    PowerShell(PowerShellStep),
    /// Checkout step: - checkout: self
    Checkout(CheckoutStep),
    /// Task step: - task: Bash@3
    Task(TaskStep),
    /// Template step: - template: steps.yml
    Template(TemplateStep),
    /// Download step: - download: current
    Download(DownloadStep),
    /// Publish step: - publish: $(Build.ArtifactStagingDirectory)
    Publish(PublishStep),
    /// Get package step
    GetPackage(GetPackageStep),
    /// Review app step (deployment)
    ReviewApp(ReviewAppStep),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScriptStep {
    pub script: String,
    pub working_directory: Option<String>,
    #[serde(default)]
    pub fail_on_stderr: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BashStep {
    pub bash: String,
    pub working_directory: Option<String>,
    #[serde(default)]
    pub fail_on_stderr: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PwshStep {
    pub pwsh: String,
    pub working_directory: Option<String>,
    #[serde(default)]
    pub fail_on_stderr: bool,
    pub error_action_preference: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PowerShellStep {
    pub powershell: String,
    pub working_directory: Option<String>,
    #[serde(default)]
    pub fail_on_stderr: bool,
    pub error_action_preference: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckoutStep {
    pub checkout: CheckoutSource,
    #[serde(default)]
    pub clean: bool,
    pub fetch_depth: Option<u32>,
    pub fetch_tags: Option<bool>,
    #[serde(default)]
    pub lfs: bool,
    #[serde(default)]
    pub submodules: SubmoduleOption,
    pub path: Option<String>,
    pub persistent_credentials: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CheckoutSource {
    /// checkout: self
    SelfRepo(CheckoutSelf),
    /// checkout: none
    None(CheckoutNone),
    /// checkout: repository-name
    Repository(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CheckoutSelf {
    #[serde(rename = "self")]
    SelfRepo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CheckoutNone {
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(untagged)]
pub enum SubmoduleOption {
    #[default]
    False,
    True,
    Recursive,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskStep {
    pub task: String,
    #[serde(default)]
    pub inputs: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateStep {
    pub template: String,
    #[serde(default)]
    pub parameters: HashMap<String, serde_yaml::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadStep {
    pub download: DownloadSource,
    pub artifact: Option<String>,
    pub patterns: Option<String>,
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DownloadSource {
    Current(DownloadCurrent),
    None(DownloadNone),
    Pipeline(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DownloadCurrent {
    Current,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DownloadNone {
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishStep {
    pub publish: String,
    pub artifact: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetPackageStep {
    pub get_package: String,
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewAppStep {
    pub review_app: String,
}

// =============================================================================
// Extends
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Extends {
    pub template: String,
    #[serde(default)]
    pub parameters: HashMap<String, serde_yaml::Value>,
}

// =============================================================================
// Lock Behavior
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LockBehavior {
    RunLatest,
    Sequential,
}

// =============================================================================
// Execution Results (for runtime)
// =============================================================================

#[derive(Debug, Clone)]
pub struct StepResult {
    pub step_name: Option<String>,
    pub display_name: Option<String>,
    pub status: StepStatus,
    pub output: String,
    pub error: Option<String>,
    pub duration: Duration,
    pub exit_code: Option<i32>,
    pub outputs: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepStatus {
    Pending,
    Running,
    Succeeded,
    SucceededWithIssues,
    Failed,
    Canceled,
    Skipped,
}

#[derive(Debug, Clone)]
pub struct JobResult {
    pub job_name: String,
    pub display_name: Option<String>,
    pub status: JobStatus,
    pub steps: Vec<StepResult>,
    pub duration: Duration,
    pub outputs: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobStatus {
    Pending,
    Running,
    Succeeded,
    SucceededWithIssues,
    Failed,
    Canceled,
    Skipped,
}

#[derive(Debug, Clone)]
pub struct StageResult {
    pub stage_name: String,
    pub display_name: Option<String>,
    pub status: StageStatus,
    pub jobs: Vec<JobResult>,
    pub duration: Duration,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StageStatus {
    Pending,
    Running,
    Succeeded,
    SucceededWithIssues,
    Failed,
    Canceled,
    Skipped,
}

#[derive(Debug, Clone)]
pub struct ExecutionContext {
    pub pipeline_name: String,
    pub env: HashMap<String, String>,
    pub working_dir: String,
    pub variables: HashMap<String, String>,
    pub parameters: HashMap<String, serde_yaml::Value>,
}

impl ExecutionContext {
    pub fn new(pipeline_name: String, working_dir: String) -> Self {
        Self {
            pipeline_name,
            env: HashMap::new(),
            working_dir,
            variables: HashMap::new(),
            parameters: HashMap::new(),
        }
    }

    pub fn with_env(mut self, env: HashMap<String, String>) -> Self {
        self.env = env;
        self
    }

    pub fn with_variables(mut self, variables: HashMap<String, String>) -> Self {
        self.variables = variables;
        self
    }

    pub fn with_parameters(mut self, parameters: HashMap<String, serde_yaml::Value>) -> Self {
        self.parameters = parameters;
        self
    }
}

// =============================================================================
// Value type for expressions
// =============================================================================

/// Runtime value type used in expression evaluation
#[derive(Debug, Clone, PartialEq, Default)]
pub enum Value {
    #[default]
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<Value>),
    Object(HashMap<String, Value>),
}

impl Value {
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Null => false,
            Value::Bool(b) => *b,
            Value::Number(n) => *n != 0.0,
            Value::String(s) => !s.is_empty(),
            Value::Array(a) => !a.is_empty(),
            Value::Object(o) => !o.is_empty(),
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_number(&self) -> Option<f64> {
        match self {
            Value::Number(n) => Some(*n),
            Value::String(s) => s.parse().ok(),
            _ => None,
        }
    }

    pub fn as_string(&self) -> String {
        match self {
            Value::Null => "".to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Number(n) => {
                if n.fract() == 0.0 {
                    (*n as i64).to_string()
                } else {
                    n.to_string()
                }
            }
            Value::String(s) => s.clone(),
            Value::Array(_) | Value::Object(_) => self.to_json(),
        }
    }

    pub fn to_json(&self) -> String {
        match self {
            Value::Null => "null".to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Number(n) => n.to_string(),
            Value::String(s) => format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"")),
            Value::Array(arr) => {
                let items: Vec<String> = arr.iter().map(|v| v.to_json()).collect();
                format!("[{}]", items.join(","))
            }
            Value::Object(obj) => {
                let items: Vec<String> = obj
                    .iter()
                    .map(|(k, v)| format!("\"{}\":{}", k, v.to_json()))
                    .collect();
                format!("{{{}}}", items.join(","))
            }
        }
    }
}

impl From<bool> for Value {
    fn from(b: bool) -> Self {
        Value::Bool(b)
    }
}

impl From<i64> for Value {
    fn from(n: i64) -> Self {
        Value::Number(n as f64)
    }
}

impl From<f64> for Value {
    fn from(n: f64) -> Self {
        Value::Number(n)
    }
}

impl From<String> for Value {
    fn from(s: String) -> Self {
        Value::String(s)
    }
}

impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Value::String(s.to_string())
    }
}

impl<T: Into<Value>> From<Vec<T>> for Value {
    fn from(v: Vec<T>) -> Self {
        Value::Array(v.into_iter().map(Into::into).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_value_is_truthy() {
        assert!(!Value::Null.is_truthy());
        assert!(!Value::Bool(false).is_truthy());
        assert!(Value::Bool(true).is_truthy());
        assert!(!Value::Number(0.0).is_truthy());
        assert!(Value::Number(1.0).is_truthy());
        assert!(!Value::String("".to_string()).is_truthy());
        assert!(Value::String("hello".to_string()).is_truthy());
    }

    #[test]
    fn test_value_as_string() {
        assert_eq!(Value::Null.as_string(), "");
        assert_eq!(Value::Bool(true).as_string(), "true");
        assert_eq!(Value::Number(42.0).as_string(), "42");
        assert_eq!(Value::Number(3.14).as_string(), "3.14");
        assert_eq!(Value::String("hello".to_string()).as_string(), "hello");
    }

    #[test]
    fn test_depends_on_as_vec() {
        assert_eq!(DependsOn::Default.as_vec(), Vec::<String>::new());
        assert_eq!(DependsOn::None.as_vec(), Vec::<String>::new());
        assert_eq!(
            DependsOn::Single("build".to_string()).as_vec(),
            vec!["build".to_string()]
        );
        assert_eq!(
            DependsOn::Multiple(vec!["a".to_string(), "b".to_string()]).as_vec(),
            vec!["a".to_string(), "b".to_string()]
        );
    }
}
