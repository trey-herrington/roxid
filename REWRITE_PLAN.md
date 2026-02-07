# Roxid Rewrite Plan: Complete Azure DevOps Pipeline Emulator

**Created:** 2026-02-04  
**Updated:** 2026-02-07  
**Status:** Complete - All Phases Done  
**Goal:** 100% Azure DevOps Pipelines compatible local execution environment with TUI

## Progress Summary

| Phase | Status | Completion |
|-------|--------|------------|
| Phase 1: Core Foundation | ✅ Complete | 100% |
| Phase 2: Execution Engine | ✅ Complete | 100% |
| Phase 3: Runners | ✅ Complete | 100% |
| Phase 4: Template System | ✅ Complete | 100% |
| Phase 5: Testing Framework | ✅ Complete | 100% |
| Phase 6: TUI Rewrite | ✅ Complete | 100% |
| Phase 7: CLI Enhancements | ✅ Complete | 100% |

---

## Executive Summary

A full rewrite of Roxid to become a **100% Azure DevOps Pipelines compatible** local execution environment with a Ratatui TUI. The goal: run actual `azure-pipelines.yml` files locally and write unit tests for pipeline logic.

### Key Decisions

| Decision | Choice |
|----------|--------|
| Compatibility Target | 100% Azure DevOps compatible |
| Primary Format | Azure DevOps first (GitHub Actions later) |
| Action/Task Handling | Download & run actual Azure DevOps tasks |
| Container Support | Docker-based container jobs |
| Expression Engine | Full support (${{ }}, $[ ], $(var)) |
| Trigger Behavior | Manual execution only |
| Architecture | Direct library calls (TUI), gRPC (optional server) |
| Test Output | Both terminal streaming and JUnit/TAP reports |
| Timeline | Full rewrite (weeks) |

### Testing Capabilities (Priority)

- ✅ Assert step outputs
- ✅ Assert execution order
- ✅ JUnit XML / TAP / Terminal reports
- ⏳ Mock external services (future)
- ⏳ Dry-run mode (future)
- ⏳ Snapshot testing (future)

---

## Architecture Overview

```
┌──────────────────────────────────────────────────────────────────┐
│                         roxid (CLI)                               │
│  • Pipeline testing commands (roxid test)                        │
│  • Interactive TUI (roxid tui)                                   │
│  • Direct execution (roxid run pipeline.yml)                     │
└─────────────────────────────┬────────────────────────────────────┘
                              │ Direct library calls
┌─────────────────────────────▼────────────────────────────────────┐
│                    pipeline-service                               │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │  Parser                                                      │ │
│  │  • Azure DevOps YAML schema                                  │ │
│  │  • Template resolution (${{ template }})                     │ │
│  │  • Expression parsing                                        │ │
│  └─────────────────────────────────────────────────────────────┘ │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │  Expression Engine                                           │ │
│  │  • Compile-time expressions ${{ }}                           │ │
│  │  • Runtime expressions $[ ]                                  │ │
│  │  • Macro substitution $(var)                                 │ │
│  │  • Built-in functions (eq, ne, contains, format, etc.)       │ │
│  └─────────────────────────────────────────────────────────────┘ │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │  Execution Engine                                            │ │
│  │  • DAG builder (stages → jobs → steps)                       │ │
│  │  • Dependency resolution (dependsOn)                         │ │
│  │  • Parallel execution with maxParallel                       │ │
│  │  • Matrix expansion                                          │ │
│  │  • Condition evaluation                                      │ │
│  └─────────────────────────────────────────────────────────────┘ │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │  Runners                                                     │ │
│  │  • Shell runner (script, bash, pwsh, powershell)             │ │
│  │  • Task runner (download + execute Azure tasks)              │ │
│  │  • Container runner (Docker-based job execution)             │ │
│  └─────────────────────────────────────────────────────────────┘ │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │  Task Cache                                                  │ │
│  │  • Download tasks from Azure DevOps marketplace              │ │
│  │  • Version management                                        │ │
│  │  • Local cache (~/.roxid/tasks/)                             │ │
│  └─────────────────────────────────────────────────────────────┘ │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │  Test Framework                                              │ │
│  │  • Output assertions                                         │ │
│  │  • Execution order verification                              │ │
│  │  • JUnit XML / TAP output                                    │ │
│  └─────────────────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────────────┘
```

---

## Phase 1: Core Foundation (Week 1-2) ✅ COMPLETE

**Completed:** 2026-02-05  
**Test Coverage:** 101 tests passing (129 total after Phase 2)

### 1.1 Data Models ✅

Comprehensive Azure DevOps pipeline models implemented in `pipeline-service/src/parser/models.rs`:

**Implemented Types:**
- `Pipeline` - Root structure with all Azure DevOps properties
- `Trigger`, `PrTrigger`, `Schedule` - CI/PR/scheduled trigger configurations
- `Resources` - Repository, container, pipeline, package, webhook resources
- `Variable` - Key-value, group references, and template variables
- `Parameter` - Typed parameters with all Azure DevOps types
- `Stage`, `Job`, `Step` - Full pipeline hierarchy
- `StepAction` - All step types (Script, Bash, Pwsh, PowerShell, Checkout, Task, Template, Download, Publish)
- `Strategy` - Matrix and deployment strategies (runOnce, rolling, canary)
- `Pool`, `Container`, `Environment` - Execution environment types
- `DependsOn` - Dependency declarations
- `Value` - Runtime value type for expression evaluation
- Execution result types: `StepResult`, `JobResult`, `StageResult`, `ExecutionContext`

```rust
// Core pipeline structure
pub struct Pipeline {
    pub name: Option<String>,
    pub trigger: Option<Trigger>,
    pub pr: Option<PrTrigger>,
    pub schedules: Option<Vec<Schedule>>,
    pub resources: Option<Resources>,
    pub variables: Vec<Variable>,
    pub parameters: Vec<Parameter>,
    pub stages: Vec<Stage>,      // Full structure
    pub jobs: Vec<Job>,          // Shorthand (no stages)
    pub steps: Vec<Step>,        // Shorthand (no stages/jobs)
    pub pool: Option<Pool>,
    pub extends: Option<Extends>,
}

pub struct Stage {
    pub stage: String,
    pub display_name: Option<String>,
    pub depends_on: DependsOn,
    pub condition: Option<String>,
    pub variables: Vec<Variable>,
    pub jobs: Vec<Job>,
    pub template: Option<TemplateRef>,
}

pub struct Job {
    pub job: String,
    pub display_name: Option<String>,
    pub depends_on: DependsOn,
    pub condition: Option<String>,
    pub strategy: Option<Strategy>,
    pub pool: Option<Pool>,
    pub container: Option<Container>,
    pub services: HashMap<String, Container>,
    pub variables: Vec<Variable>,
    pub steps: Vec<Step>,
    pub timeout_in_minutes: Option<u32>,
    pub cancel_timeout_in_minutes: Option<u32>,
    pub continue_on_error: bool,
}

pub struct Step {
    pub name: Option<String>,
    pub display_name: Option<String>,
    pub condition: Option<String>,
    pub continue_on_error: bool,
    pub enabled: bool,
    pub timeout_in_minutes: Option<u32>,
    pub retry_count_on_task_failure: Option<u32>,
    pub env: HashMap<String, String>,
    pub action: StepAction,
}

pub enum StepAction {
    Script { script: String, working_directory: Option<String>, fail_on_stderr: bool },
    Bash { bash: String, working_directory: Option<String>, fail_on_stderr: bool },
    Pwsh { pwsh: String, working_directory: Option<String>, fail_on_stderr: bool, error_action_preference: Option<String> },
    PowerShell { powershell: String, working_directory: Option<String>, fail_on_stderr: bool },
    Checkout { checkout: CheckoutSource, clean: bool, fetch_depth: Option<u32>, lfs: bool, submodules: SubmoduleOption },
    Task { task: String, inputs: HashMap<String, String> },
    Template { template: String, parameters: HashMap<String, Value> },
    Download { download: DownloadSource, artifact: Option<String>, patterns: Option<Vec<String>> },
    Publish { publish: String, artifact: Option<String>, display_name: Option<String> },
}
```

### 1.2 Parser with Validation ✅

Implemented in `pipeline-service/src/parser/azure.rs` and `pipeline-service/src/parser/error.rs`:

**Features:**
- `AzureParser::parse()` - Parse YAML string to Pipeline
- `AzureParser::parse_file()` - Parse pipeline from file path
- `AzureParser::parse_with_templates()` - Parse with template resolution (stub for Phase 4)
- `PipelineValidator::validate()` - Semantic validation with helpful errors
- `normalize_pipeline()` - Convert steps-only/jobs-only to full stage structure
- Cycle detection for stage/job dependencies
- Dependency validation (unknown references)

**Error Types (error.rs):**
- `ParseError` - Detailed errors with line/column, context, and suggestions
- `ParseErrorKind` - YamlSyntax, InvalidSchema, UnknownField, InvalidValue, etc.
- `ValidationError` - Semantic validation errors with paths
- Rust compiler-style error formatting

Implement a robust YAML parser with helpful error messages:

```rust
pub struct PipelineParser {
    template_resolver: TemplateResolver,
}

impl PipelineParser {
    pub fn parse(content: &str) -> Result<Pipeline, ParseError>;
    pub fn parse_file(path: &Path) -> Result<Pipeline, ParseError>;
    pub fn parse_with_templates(path: &Path, repo_root: &Path) -> Result<Pipeline, ParseError>;
}

pub struct ParseError {
    pub message: String,
    pub line: usize,
    pub column: usize,
    pub context: String,  // Show surrounding lines
    pub suggestion: Option<String>,
}
```

### 1.3 Expression Engine ✅

Implemented in `pipeline-service/src/expression/`:

**Lexer (lexer.rs):**
- Tokenizes all Azure DevOps expression syntax
- Operators: `+`, `-`, `*`, `/`, `%`, `==`, `!=`, `<`, `<=`, `>`, `>=`, `&&`, `||`, `!`
- Literals: null, true, false, numbers (int/float), strings (with `''` escaping)
- Delimiters: `()`, `[]`, `{}`, `.`, `,`, `:`, `?`
- `extract_expressions()` - Extract `${{ }}`, `$[ ]`, `$(var)` from text

**Parser (parser.rs):**
- Recursive descent parser with correct operator precedence
- AST types: `Expr`, `Reference`, `ReferencePart`, `BinaryOp`, `UnaryOp`
- Supports: literals, references, function calls, index/member access, ternary, arrays, objects

**Evaluator (evaluator.rs):**
- `ExpressionContext` - Full context with variables, parameters, pipeline, stage, job, steps, dependencies, env, resources
- `Evaluator` - Evaluates AST against context with short-circuit logic
- `ExpressionEngine` - High-level API:
  - `evaluate_compile_time()` - Evaluate `${{ expression }}`
  - `evaluate_runtime()` - Evaluate `$[ expression ]`
  - `substitute_macros()` - Replace `$(variableName)` references

Full Azure DevOps expression support:

```rust
pub struct ExpressionEngine {
    context: ExpressionContext,
}

pub struct ExpressionContext {
    pub variables: HashMap<String, Value>,
    pub parameters: HashMap<String, Value>,
    pub pipeline: PipelineContext,
    pub stage: Option<StageContext>,
    pub job: Option<JobContext>,
    pub steps: HashMap<String, StepContext>,
    pub dependencies: DependenciesContext,
}

impl ExpressionEngine {
    // Compile-time: ${{ expression }}
    pub fn evaluate_compile_time(&self, expr: &str) -> Result<Value, ExprError>;
    
    // Runtime: $[ expression ]
    pub fn evaluate_runtime(&self, expr: &str) -> Result<Value, ExprError>;
    
    // Macro: $(variableName)
    pub fn substitute_macros(&self, text: &str) -> Result<String, ExprError>;
}
```

### 1.4 Built-in Functions

| Category | Functions |
|----------|-----------|
| Comparison | `eq`, `ne`, `lt`, `le`, `gt`, `ge`, `in`, `notIn` |
| Logical | `and`, `or`, `not`, `xor` |
| String | `contains`, `startsWith`, `endsWith`, `format`, `join`, `replace`, `split`, `lower`, `upper`, `trim` |
| Conversion | `convertToJson` |
| Status | `succeeded`, `failed`, `canceled`, `always`, `succeededOrFailed` |
| Utility | `coalesce`, `counter`, `iif` |

---

## Phase 2: Execution Engine (Week 2-3) ✅ COMPLETE

**Completed:** 2026-02-05  
**Test Coverage:** 28 new tests (129 total)

### 2.1 DAG Builder ✅

Implemented in `pipeline-service/src/execution/graph.rs`:

**Features:**
- `ExecutionGraph::from_pipeline()` - Build DAG from pipeline definition
- `ExecutionGraph::validate()` - Cycle detection for stages and jobs
- `ExecutionGraph::topological_order()` - Get stages in dependency order
- `ExecutionGraph::parallel_stages()` - Get stages grouped by parallel execution level
- `ExecutionGraph::parallel_jobs()` - Get jobs grouped by parallel execution level
- Pipeline normalization (steps-only → jobs-only → stages with proper defaults)
- Stage and job dependency validation with helpful error messages

Build execution graph from pipeline definition:

```rust
pub struct ExecutionGraph {
    stages: Vec<StageNode>,
}

pub struct StageNode {
    pub stage: Stage,
    pub dependencies: Vec<String>,
    pub jobs: Vec<JobNode>,
}

pub struct JobNode {
    pub job: Job,
    pub dependencies: Vec<String>,
    pub matrix_instances: Vec<MatrixInstance>,
}

impl ExecutionGraph {
    pub fn from_pipeline(pipeline: &Pipeline) -> Result<Self, GraphError>;
    pub fn validate(&self) -> Result<(), GraphError>;  // Cycle detection
    pub fn topological_order(&self) -> Vec<&StageNode>;
}
```

### 2.2 Executor ✅

Implemented in `pipeline-service/src/execution/executor.rs`:

**Features:**
- `PipelineExecutor::from_pipeline()` - Create executor from pipeline
- `PipelineExecutor::execute()` - Execute full pipeline with progress events
- `execute_stage()` - Stage execution with condition evaluation
- `execute_job()` - Job execution with matrix support
- `execute_step()` - Step execution for all step types (Script, Bash, Pwsh, PowerShell, Checkout, Task, etc.)
- Condition evaluation using ExpressionEngine
- Variable substitution (`$(variable)` syntax)
- Azure DevOps logging command parsing (`##vso[task.setvariable]`)
- Progress events for TUI/CLI integration

Implemented in `pipeline-service/src/execution/context.rs`:

**Features:**
- `RuntimeContext` - Manages execution state during pipeline run
- Variable merging at pipeline/stage/job levels
- Step output tracking
- Dependencies context for expression evaluation
- Environment variable management

Implemented in `pipeline-service/src/execution/events.rs`:

**Features:**
- `ExecutionEvent` enum with all execution events
- `ProgressSender`/`ProgressReceiver` channel types
- Events for: PipelineStarted/Completed, StageStarted/Completed/Skipped, JobStarted/Completed/Skipped, StepStarted/Completed/Skipped/Output, VariableSet, Log, Error

Orchestrate pipeline execution:

```rust
pub struct PipelineExecutor {
    graph: ExecutionGraph,
    runners: RunnerRegistry,
    task_cache: TaskCache,
    event_tx: ProgressSender,
}

impl PipelineExecutor {
    pub async fn execute(&self, context: ExecutionContext) -> ExecutionResult;
    
    async fn execute_stage(&self, stage: &StageNode) -> StageResult;
    async fn execute_job(&self, job: &JobNode) -> JobResult;
    async fn execute_step(&self, step: &Step) -> StepResult;
    
    fn should_run(&self, condition: Option<&str>, context: &EvalContext) -> bool;
}

pub struct ExecutionResult {
    pub stages: Vec<StageResult>,
    pub duration: Duration,
    pub success: bool,
    pub logs: Vec<LogEntry>,
}
```

### 2.3 Matrix Expansion ✅

Implemented in `pipeline-service/src/execution/matrix.rs`:

**Features:**
- `MatrixExpander::expand()` - Expand strategy to matrix instances
- `MatrixInstance` - Single matrix configuration with variables
- `MatrixBuilder` - Programmatic matrix construction
- Support for inline matrix definitions
- Support for parallel job count expansion
- `maxParallel` tracking for execution limiting

Support Azure DevOps matrix strategies:

```rust
pub struct MatrixStrategy {
    pub matrix: HashMap<String, HashMap<String, Value>>,
    pub max_parallel: Option<u32>,
}

impl MatrixStrategy {
    pub fn expand(&self) -> Vec<MatrixInstance>;
}
```

Example matrix YAML:
```yaml
strategy:
  matrix:
    linux:
      vm_image: ubuntu-latest
    windows:
      vm_image: windows-latest
  maxParallel: 2
```

---

## Phase 3: Runners (Week 3-4) ✅ COMPLETE

**Completed:** 2026-02-05  
**Test Coverage:** 158 tests passing (including runner tests)

### 3.1 Shell Runner ✅

Implemented in `pipeline-service/src/runners/shell.rs`:

**Features:**
- `ShellRunner::new()` - Create runner with default shell
- `ShellRunner::run_script()` - Execute with default shell (sh/cmd)
- `ShellRunner::run_bash()` - Execute bash scripts
- `ShellRunner::run_pwsh()` - Execute PowerShell Core scripts
- `ShellRunner::run_powershell()` - Execute Windows PowerShell scripts
- `ShellRunner::run_script_streaming()` - Real-time output streaming
- Azure DevOps logging command parsing (##vso[task.setvariable], etc.)
- Environment variable support
- Working directory configuration
- Timeout support
- fail_on_stderr handling

```rust
pub struct ShellRunner {
    default_shell: Shell,
}

impl ShellRunner {
    pub async fn run_script(&self, script: &str, env: &HashMap<String, String>, 
                           working_dir: &Path, config: &ShellConfig) -> ShellOutput;
    
    pub async fn run_bash(&self, script: &str, ...) -> ShellOutput;
    pub async fn run_pwsh(&self, script: &str, ...) -> ShellOutput;
    pub async fn run_powershell(&self, script: &str, ...) -> ShellOutput;
}
```

### 3.2 Task Runner ✅

Implemented in `pipeline-service/src/runners/task.rs` and `pipeline-service/src/tasks/`:

**Features:**
- `TaskRunner::execute_task()` - Execute Azure DevOps tasks by reference (e.g., "Bash@3")
- `TaskCache` - Download and cache tasks from Azure DevOps marketplace
- `TaskManifest` - Full task.json parsing with all Azure DevOps properties
- Built-in task stubs for Bash, PowerShell, and CmdLine tasks
- Node.js task execution support
- PowerShell task execution support
- Input validation and merging with defaults
- Environment variable mapping (INPUT_* convention)

```rust
pub struct TaskRunner {
    cache: TaskCache,
    shell_runner: ShellRunner,
    node_path: Option<PathBuf>,
    powershell_path: Option<PathBuf>,
}

pub struct TaskCache {
    cache_dir: PathBuf,  // ~/.roxid/tasks/
}

impl TaskCache {
    pub async fn get_task(&self, task_ref: &str) -> Result<CachedTask, TaskCacheError>;
    pub fn parse_task_reference(task_ref: &str) -> Result<(String, String), TaskCacheError>;
}

impl TaskRunner {
    pub async fn execute_task(&self, task_ref: &str, inputs: &HashMap<String, String>, 
                              env: &HashMap<String, String>, working_dir: &Path) -> Result<StepResult, TaskRunnerError>;
}
```

**Task Manifest Parsing (task.json):**
```rust
pub struct TaskManifest {
    pub name: String,
    pub version: TaskVersion,
    pub execution: Option<TaskExecutionSection>,
    pub inputs: Vec<TaskInput>,
    // ... full Azure DevOps task.json schema
}

pub struct TaskExecutionSection {
    pub node: Option<TaskExecution>,
    pub node10: Option<TaskExecution>,
    pub node16: Option<TaskExecution>,
    pub node20: Option<TaskExecution>,
    pub powershell: Option<TaskExecution>,
    pub powershell3: Option<TaskExecution>,
}
```

### 3.3 Container Runner ✅

Implemented in `pipeline-service/src/runners/container.rs`:

**Features:**
- `ContainerRunner::run_job_in_container()` - Execute jobs in Docker containers
- `ContainerRunner::start_service_containers()` - Start service containers
- `ContainerRunner::stop_service_containers()` - Clean up service containers
- Image pull policies (Always, IfNotPresent, Never)
- Volume mounting with workspace mapping
- Port mapping
- Environment variable injection
- Docker socket mapping option
- Container auto-removal

```rust
pub struct ContainerRunner {
    config: ContainerConfig,
}

impl ContainerRunner {
    pub async fn run_job_in_container(&self, job: &Job, container: &ContainerRef,
                                      env: &HashMap<String, String>, working_dir: &Path) -> Result<JobResult, ContainerError>;
    pub async fn start_service_containers(&self, services: &HashMap<String, ContainerRef>,
                                          env: &HashMap<String, String>, working_dir: &Path) -> Result<ServiceHandles, ContainerError>;
    pub async fn stop_service_containers(&self, handles: ServiceHandles) -> Result<(), ContainerError>;
    pub async fn is_available(&self) -> bool;
}
```

### 3.4 Executor Integration ✅

Updated `pipeline-service/src/execution/executor.rs`:

**Features:**
- `PipelineExecutor::with_task_runner()` - Enable task execution
- `PipelineExecutor::with_container_runner()` - Enable container jobs
- Automatic task execution for `- task: TaskName@Version` steps
- Execution configuration with `ExecutorConfig`

```rust
pub struct PipelineExecutor {
    graph: ExecutionGraph,
    config: ExecutorConfig,
    event_tx: Option<ProgressSender>,
    shell_runner: ShellRunner,
    task_runner: Option<TaskRunner>,
    container_runner: Option<ContainerRunner>,
}

pub struct ExecutorConfig {
    pub task_cache_dir: Option<PathBuf>,
    pub enable_containers: bool,
    // ...
}
```

---

## Phase 4: Template System (Week 4-5) ✅ Complete

**Started:** 2026-02-06  
**Completed:** 2026-02-06  
**Test Coverage:** 195 tests passing (including template-specific and if/each directive tests)

### 4.1 Template Resolution Engine ✅

Implemented in `pipeline-service/src/parser/template.rs`:

**Features:**
- `TemplateEngine::new()` - Create engine with repo root path
- `TemplateEngine::with_resource_repo()` - Add cross-repo template paths
- `TemplateEngine::resolve_pipeline()` - Fully resolve all template references
- **Extends resolution** - Pipeline inheritance with child overrides (trigger, pr, schedules, resources, variables, pool, name)
- **Step/Job/Stage/Variable template expansion** - Load template files, validate parameters, substitute `${{ }}` expressions
- **Template file loading** - Parse YAML to determine content type (steps/jobs/stages/variables/pipeline)
- **Parameter resolution** - Validate types, apply defaults, check required params, validate allowed values
- **Compile-time expression substitution** - Evaluate `${{ }}` expressions, preserve `$(macro)` and `$[ runtime ]` expressions
- **Nested template support** - Recursive template expansion with parameter passthrough
- **Cycle detection** with include stack and MAX_TEMPLATE_DEPTH=50
- **Cross-repo template paths** via `resource_repos` and `@` syntax
- Extra parameters passed through (matches Azure DevOps behavior)

**Error Types (TemplateError):**
- `NotFound` - Template file not found
- `CircularReference` - Circular template inclusion
- `MaxDepthExceeded` - Template depth limit exceeded
- `InvalidParameter` / `TypeMismatch` / `MissingParameter` - Parameter validation errors
- `ParseError` / `ExpressionError` - Parse/expression errors in templates

```rust
pub struct TemplateEngine {
    repo_root: PathBuf,
    resource_repos: HashMap<String, PathBuf>,
    include_stack: Vec<String>,
}

impl TemplateEngine {
    pub fn new(repo_root: PathBuf) -> Self;
    pub fn with_resource_repo(self, name: String, path: PathBuf) -> Self;
    pub fn resolve_pipeline(&mut self, pipeline: Pipeline) -> ParseResult<Pipeline>;
}
```

### 4.2 Parameter Types ✅

Full typed parameter support implemented in models and template engine:

```rust
pub enum ParameterType {
    String, Number, Boolean, Object,
    Step, StepList, Job, JobList, Stage, StageList,
}
```

- Simple key-value parameter format (name: default_value)
- Full parameter declarations with type, default, allowed values
- Type validation for all parameter types
- Azure DevOps passthrough behavior for undeclared parameters

### 4.3 Template Expressions (Conditional & Iterative) ✅

`${{ if }}` and `${{ each }}` compile-time template expressions for conditional inclusion
and iterative expansion of YAML nodes. These operate on the `serde_yaml::Value` tree
before deserialization to typed structs.

**Features:**
- Two-phase template loading: `load_template_file()` returns `RawTemplateFile` (raw `serde_yaml::Value`), then `resolve_raw_template()` processes directives after parameter resolution
- `${{ if }}` / `${{ elseif }}` / `${{ else }}` chaining with `chain_active`/`chain_taken` state tracking
- `${{ each }}` iteration over sequences and objects with iteration variable injection
- `process_template_expressions()` recursively processes the raw YAML tree
- Iteration variables from `${{ each }}` shadow built-in context names (except `variables` and `parameters`)
- Nested directive support (if/each within if/each bodies)

---

## Phase 5: Testing Framework (Week 5-6) ✅ COMPLETE

**Started:** 2026-02-07  
**Completed:** 2026-02-07  
**Test Coverage:** 245 tests passing (50 new testing framework tests)

### 5.1 Test Definition Models ✅

Implemented in `pipeline-service/src/testing/mod.rs`:

**Features:**
- `TestSuite` - Collection of tests with name, defaults, and test list
- `PipelineTest` - Individual test with pipeline path, variables, parameters, and assertions
- `TestDefaults` - Suite-level defaults for working_dir and variables
- `AssertionDef` - YAML-friendly assertion definitions with custom `Deserialize` impl
  - Handles bare strings (`pipeline_succeeded`) and key-value maps (`step_succeeded: Build`)
- `yaml_to_value()` - Convert `serde_yaml::Value` to pipeline `Value` type
- `AssertionDef::to_assertion()` - Convert definition to evaluable `Assertion` enum

### 5.2 Test File Parser ✅

Implemented in `pipeline-service/src/testing/parser.rs`:

**Features:**
- `TestFileParser::parse()` - Parse test suite from YAML string
- `TestFileParser::parse_file()` - Parse test suite from file path
- `TestFileParser::discover()` - Find `roxid-test.yml` / `roxid-test.yaml` files recursively
- `apply_defaults()` - Merge suite defaults into individual tests
- Validation: empty tests, empty names, duplicate test names

### 5.3 Assertion Engine ✅

Implemented in `pipeline-service/src/testing/assertions.rs`:

**Features:**
- `Assertion` enum - Evaluable assertion types:
  - `PipelineSucceeded` / `PipelineFailed`
  - `StepSucceeded` / `StepFailed` / `StepSkipped` (by name or display_name)
  - `JobSucceeded` / `JobFailed` / `StageSucceeded` / `StageFailed`
  - `StepOutputEquals` / `StepOutputContains`
  - `StepRanBefore` - Execution ordering verification
  - `StepsRanInParallel` - Parallel execution verification
  - `VariableEquals` / `VariableContains`
- `AssertionResult` - Result with passed/failed status, description, and failure details
- `AssertionEvaluator::new()` - Takes `ExecutionResult`, builds flattened `StepInfo` index
- `AssertionEvaluator::evaluate()` / `evaluate_all()` - Evaluate assertions against results
- Step lookup by name or display_name for flexible matching

### 5.4 Test Runner ✅

Implemented in `pipeline-service/src/testing/runner.rs`:

**Features:**
- `TestRunner::new()` - Create runner with builder pattern
- `TestRunner::with_working_dir()` - Set working directory
- `TestRunner::with_filter()` - Filter tests by glob pattern
- `TestRunner::with_fail_fast()` - Stop on first failure
- `TestRunner::run_test()` - Parse pipeline, execute, evaluate assertions
- `TestRunner::run_suite()` - Run all tests in a suite with filter/fail-fast support
- `TestRunner::run_file()` - Parse and run test file
- `TestResult` / `TestSuiteResult` - Structured results with timing
- `TestRunnerConfig` - Configuration options
- `matches_filter()` - Glob-style filter matching (case-insensitive, `*` wildcards)

### 5.5 Test Reporter ✅

Implemented in `pipeline-service/src/testing/reporter.rs`:

**Features:**
- `TestReporter::to_junit_xml()` - JUnit XML output for CI integration
- `TestReporter::to_tap()` - TAP (Test Anything Protocol) output
- `TestReporter::to_terminal()` - Human-readable terminal output with pass/fail indicators
- `ReportFormat` enum with `FromStr` impl (junit, tap, terminal)

---

## Phase 6: TUI Rewrite (Week 6-7) ✅ COMPLETE

**Started:** 2026-02-07  
**Completed:** 2026-02-07  
**Architecture Change:** Eliminated gRPC dependency — TUI now calls pipeline-service library directly

### 6.1 Enhanced UI States ✅

Six application states with navigation stack (`previous_states: Vec<AppState>`) and `push_state()`/`go_back()` pattern:

```rust
pub enum AppState {
    PipelineList,        // Browse discovered pipelines
    PipelineDetail,      // Expandable tree: stages → jobs → steps
    ExecutingPipeline,   // Real-time execution with progress
    ExecutionLog,        // Scrollable/searchable log viewer
    TestResults,         // Test suite pass/fail visualization
    VariableEditor,      // Edit variables before execution
}
```

**Key architectural decisions:**
- `App::new()` is synchronous — pipeline discovery uses `AzureParser::parse_file()` + `normalize_pipeline()` directly
- `pending_execution`/`pending_test_run` flags bridge sync keyboard handlers to async run loop
- Progress channel (`pipeline_service::execution::events::progress_channel()`) feeds `ExecutionEvent`s from spawned tokio task to TUI
- All 16 `ExecutionEvent` variants processed in `process_execution_events()`

### 6.2 TUI Features ✅

| Feature | Description | File |
|---------|-------------|------|
| Pipeline List | Browse pipelines with stage/job/step counts | `ui/pipeline_list.rs` |
| Pipeline Tree View | Expandable stages → jobs → steps with type indicators | `ui/pipeline_tree.rs` |
| Real-time Execution | Live progress bar, stage panel, output panel | `ui/execution.rs` |
| Log Viewer | Scrollable, searchable output with filtering | `ui/log_viewer.rs` |
| Test Results Panel | Summary bar + pass/fail list | `ui/test_results.rs` |
| Variable Editor | Inline edit variables before execution | `ui/mod.rs` (render_variable_editor) |
| Layout System | Standard, with-errors, and execution layouts | `ui/layout.rs` |
| Components | Header, footer, discovery errors, status helpers | `ui/components.rs` |

### 6.3 Keyboard Shortcuts ✅

| Key | Action |
|-----|--------|
| `↑/k` | Move up |
| `↓/j` | Move down |
| `Enter` | Execute/Expand |
| `Tab` | Switch panels |
| `v` | Edit variables |
| `t` | Run tests |
| `l` | View logs |
| `/` | Search |
| `q/Esc` | Back/Quit |

---

## Phase 7: CLI Enhancements (Week 7)

### 7.1 Commands

```bash
# Run a pipeline
roxid run azure-pipelines.yml
roxid run azure-pipelines.yml --var "foo=bar"
roxid run azure-pipelines.yml --stage Build

# Test pipelines
roxid test                           # Run all tests in roxid-test.yml
roxid test --filter "deploy*"        # Filter tests by name
roxid test --output junit            # JUnit XML output
roxid test --output tap              # TAP output

# Validate pipelines
roxid validate azure-pipelines.yml   # Check syntax and references
roxid validate --templates           # Validate template resolution

# TUI mode
roxid tui                            # Launch interactive TUI
roxid                                # No args also launches TUI

# Task management
roxid task list                      # List cached tasks
roxid task fetch Bash@3              # Pre-download a task
roxid task clear                     # Clear task cache
roxid task path                      # Show task cache path
```

---

## Project Structure

```
roxid/
├── Cargo.toml                    # Workspace manifest
├── pipeline-service/             # Core service
│   └── src/
│       ├── lib.rs
│       ├── error.rs
│       ├── parser/
│       │   ├── mod.rs
│       │   ├── azure.rs          # Azure DevOps YAML parser
│       │   ├── template.rs       # Template resolution
│       │   └── validation.rs     # Schema validation
│       ├── expression/
│       │   ├── mod.rs
│       │   ├── lexer.rs
│       │   ├── parser.rs
│       │   ├── evaluator.rs
│       │   └── functions.rs      # Built-in functions
│       ├── execution/
│       │   ├── mod.rs
│       │   ├── graph.rs          # DAG builder
│       │   ├── executor.rs       # Pipeline executor
│       │   ├── matrix.rs         # Matrix expansion
│       │   └── context.rs        # Execution context
│       ├── runners/
│       │   ├── mod.rs
│       │   ├── shell.rs          # Script/bash/pwsh
│       │   ├── task.rs           # Azure DevOps tasks
│       │   └── container.rs      # Docker runner
│       ├── tasks/
│       │   ├── mod.rs
│       │   ├── cache.rs          # Task cache
│       │   └── manifest.rs       # task.json parser
│       └── testing/
│           ├── mod.rs
│           ├── runner.rs         # Test executor
│           ├── assertions.rs     # Assertion logic
│           └── reporter.rs       # JUnit/TAP output
├── roxid-tui/                    # TUI client
│   └── src/
│       ├── lib.rs
│       ├── app.rs
│       ├── events.rs
│       └── ui/
│           ├── mod.rs
│           ├── pipeline_list.rs
│           ├── pipeline_tree.rs
│           ├── execution.rs
│           ├── log_viewer.rs
│           └── test_results.rs
└── roxid-cli/                    # CLI entry point
    └── src/
        ├── main.rs
        ├── output.rs             # Terminal formatting helpers
        └── commands/
            ├── mod.rs
            ├── run.rs
            ├── test.rs
            ├── validate.rs
            └── task.rs
```

---

## Implementation Timeline

| Week | Phase | Deliverables |
|------|-------|--------------|
| 1 | Core Models | Data models, basic parser, validation |
| 2 | Expression Engine | Lexer, parser, evaluator, built-in functions |
| 3 | Execution Engine | DAG builder, sequential execution, conditions |
| 4 | Runners | Shell runner, task runner (basic), Docker runner |
| 5 | Templates | Template resolution, extends, parameters |
| 6 | Testing Framework | Test definitions, assertions, runners, reports |
| 7 | TUI + CLI | Enhanced TUI, CLI commands, polish |

---

## Technical Decisions

| Component | Technology | Rationale |
|-----------|------------|-----------|
| Parser | `serde_yaml` + custom deserializers | Handle Azure DevOps YAML quirks |
| Expression Engine | Hand-written recursive descent | Full control over Azure expression syntax |
| Task Execution | System Node.js / embedded | Run actual Azure DevOps JS tasks |
| Docker | `bollard` crate | Rust-native Docker API |
| Async Runtime | Tokio | Already in use, proven performance |
| gRPC | Removed | CLI and TUI use direct library calls |
| TUI | Ratatui + Crossterm | Already in use, excellent ecosystem |

---

## Risk Mitigation

| Risk | Mitigation |
|------|------------|
| Task compatibility | Start with most common tasks (Bash, PowerShell, Checkout), add progressively |
| Expression edge cases | Build comprehensive test suite from Azure docs examples |
| Docker complexity | Start with simple container jobs, add service containers later |
| Template recursion | Implement depth limits (max 50) and cycle detection |
| Performance | Profile early, optimize hot paths (expression evaluation, YAML parsing) |

---

## Success Criteria

### Phase 1 Complete When:
- [x] Can parse real-world `azure-pipelines.yml` files without error
- [x] All Azure DevOps YAML schema elements represented in Rust types
- [x] Expression engine evaluates all built-in functions correctly

### Phase 2 Complete When:
- [x] DAG correctly orders stages/jobs based on `dependsOn`
- [x] Matrix strategies expand to correct number of job instances
- [x] Conditions correctly skip/run stages/jobs/steps

### Phase 3 Complete When:
- [x] `script`, `bash`, `pwsh`, `powershell` steps execute correctly
- [x] Can download and execute `Bash@3` and `PowerShell@2` tasks
- [x] Container jobs run in Docker with correct mounts/env

### Phase 4 Complete When:
- [x] Include templates resolve and expand correctly
- [x] `extends` templates enforce structure
- [x] Cross-repository templates work (local clone)
- [x] `${{ if }}` conditional template expressions work
- [x] `${{ each }}` iterative template expressions work

### Phase 5 Complete When:
- [x] `roxid test` runs test suite and reports results
- [x] Output assertions catch step output correctly
- [x] JUnit XML output works with CI systems

### Phase 6-7 Complete When:
- [x] TUI shows pipeline tree structure
- [x] Real-time execution updates work
- [x] CLI commands all functional
- [ ] Documentation complete

---

## Future Phases (Post-MVP)

### GitHub Actions Compatibility
- Parse `.github/workflows/*.yml` files
- Map GitHub Actions concepts to internal model
- Support `uses: actions/*` syntax

### Advanced Features
- Mock/stub system for external services
- Dry-run mode (validate without executing)
- Snapshot testing for pipeline outputs
- Watch mode for automatic re-runs
- Remote service deployment
- Web UI option

### Enterprise Features
- Secret management integration (Azure Key Vault, HashiCorp Vault)
- Audit logging
- Multi-tenant support
- RBAC for pipeline execution

---

## References

- [Azure DevOps YAML Schema](https://docs.microsoft.com/en-us/azure/devops/pipelines/yaml-schema)
- [Azure DevOps Expressions](https://docs.microsoft.com/en-us/azure/devops/pipelines/process/expressions)
- [Azure DevOps Templates](https://docs.microsoft.com/en-us/azure/devops/pipelines/process/templates)
- [Azure DevOps Tasks](https://docs.microsoft.com/en-us/azure/devops/pipelines/tasks)
- [Ratatui Documentation](https://ratatui.rs/)
- [Tonic gRPC](https://github.com/hyperium/tonic)
