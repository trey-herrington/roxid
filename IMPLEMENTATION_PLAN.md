# GitHub Actions Compatibility Implementation Plan

This document outlines the plan to extend Roxid to support GitHub Actions workflow syntax.

**Status:** Future phase (post-MVP). Basic workflow models and parser already exist in `pipeline-service/src/workflow/`.

## Phase 1: Core Schema (P0)

### 1.1 Extend Models for GitHub Actions Structure

Extend the existing `pipeline-service/src/workflow/models.rs` to support:

```rust
// Workflow (top-level)
pub struct Workflow {
    pub name: Option<String>,
    pub on: Trigger,                    // Event triggers
    pub env: HashMap<String, String>,   // Workflow-level env
    pub defaults: Option<Defaults>,     // Default shell/working-directory
    pub jobs: HashMap<String, Job>,     // Jobs map
}

// Job definition
pub struct Job {
    pub name: Option<String>,           // Display name
    pub needs: Vec<String>,             // Job dependencies
    pub runs_on: Option<String>,        // Ignored locally, but parsed
    pub if_condition: Option<String>,   // Conditional execution
    pub env: HashMap<String, String>,   // Job-level env
    pub defaults: Option<Defaults>,     // Job-level defaults
    pub outputs: HashMap<String, String>, // Job outputs
    pub strategy: Option<Strategy>,     // Matrix strategy
    pub steps: Vec<Step>,               // Steps
    pub services: HashMap<String, Service>, // Service containers
    pub container: Option<Container>,   // Job container
    pub timeout_minutes: Option<u32>,
    pub continue_on_error: bool,
}

// Step definition (GitHub Actions compatible)
pub struct Step {
    pub id: Option<String>,             // Step identifier for outputs
    pub name: Option<String>,           // Display name
    pub if_condition: Option<String>,   // Conditional
    pub run: Option<String>,            // Shell command
    pub shell: Option<String>,          // Shell to use
    pub working_directory: Option<String>,
    pub uses: Option<String>,           // Action reference
    pub with: HashMap<String, Value>,   // Action inputs
    pub env: HashMap<String, String>,
    pub continue_on_error: bool,
    pub timeout_minutes: Option<u32>,
}
```

### 1.2 Expression System

Extend the existing `pipeline-service/src/expression/` module to support GitHub Actions expression syntax:

**Expression Features:**
- Literals: strings, numbers, booleans, null
- Property access: `github.event_name`, `steps.build.outputs.result`
- Operators: `==`, `!=`, `<`, `>`, `<=`, `>=`, `&&`, `||`, `!`
- Functions: `contains()`, `startsWith()`, `endsWith()`, `format()`, `join()`, `toJSON()`, `fromJSON()`, `hashFiles()`
- Status functions: `success()`, `failure()`, `always()`, `cancelled()`

Note: The expression engine already supports most of these operators and functions for Azure DevOps. GitHub Actions uses the same `${{ }}` syntax. The main work is mapping GitHub Actions-specific context names and adding a few GitHub-specific functions.

### 1.3 Context System

Implement GitHub Actions context providers:

```rust
pub struct WorkflowContext {
    pub github: GitHubContext,      // Event info, repo, ref, etc.
    pub env: HashMap<String, String>,
    pub job: JobContext,            // Current job info
    pub steps: HashMap<String, StepContext>, // Completed steps
    pub runner: RunnerContext,      // Runner info (simulated)
    pub secrets: HashMap<String, String>,
    pub inputs: HashMap<String, Value>, // Workflow inputs
    pub matrix: HashMap<String, Value>, // Current matrix values
}
```

## Phase 2: Job Execution (P1)

### 2.1 Job DAG Executor

The existing DAG builder (`pipeline-service/src/execution/graph.rs`) can be reused. Extend to:
- Parse `needs` dependencies (maps to Azure DevOps `dependsOn`)
- Job output passing via `needs.job_id.outputs.output_name`

### 2.2 Matrix Expansion

The existing matrix expander (`pipeline-service/src/execution/matrix.rs`) can be extended to support:
- `include` and `exclude` directives
- `fail-fast` and `max-parallel` options

### 2.3 Step Execution Enhancements

- Evaluate `if` conditions before running steps
- Capture outputs via `$GITHUB_OUTPUT` file (or `::set-output` deprecated syntax)
- Set environment variables via `$GITHUB_ENV` file

## Phase 3: Actions Support (P2)

### 3.1 Action Resolution

- Parse `uses` references:
  - `actions/checkout@v4` -> GitHub action
  - `./path/to/action` -> Local action
  - `docker://image:tag` -> Docker action
- Download and cache GitHub actions
- Parse `action.yml` metadata

### 3.2 Action Runners

- **JavaScript actions**: Run with Node.js (reuse task runner's Node.js execution support)
- **Composite actions**: Expand steps inline
- **Docker actions**: Run in container (reuse container runner)

## Phase 4: Advanced Features (P3)

### 4.1 Service Containers

The existing container runner (`pipeline-service/src/runners/container.rs`) already supports service containers. Extend for GitHub Actions service syntax.

### 4.2 Artifacts & Caching

- Local artifact storage in `.roxid/artifacts/`
- Local cache in `.roxid/cache/`
- Implement cache key matching

### 4.3 Reusable Workflows

- Support `workflow_call` trigger
- Input/output passing
- Secret inheritance

---

## File Structure After Implementation

```
pipeline-service/src/
├── lib.rs
├── error.rs
├── parser/
│   ├── mod.rs
│   ├── azure.rs              # Azure DevOps YAML parser
│   ├── error.rs              # ParseError, ValidationError
│   ├── models.rs             # Azure DevOps pipeline models
│   └── template.rs           # Template resolution engine
├── expression/
│   ├── mod.rs
│   ├── lexer.rs              # Tokenizer (shared)
│   ├── parser.rs             # Expression AST parser (shared)
│   ├── evaluator.rs          # Evaluator (shared, extended for GH Actions)
│   └── functions.rs          # Built-in functions (shared + GH-specific)
├── execution/
│   ├── mod.rs
│   ├── executor.rs           # Pipeline executor (shared)
│   ├── graph.rs              # DAG builder (shared)
│   ├── matrix.rs             # Matrix expansion (shared + GH include/exclude)
│   ├── context.rs            # Runtime context
│   └── events.rs             # Execution events
├── runners/
│   ├── mod.rs
│   ├── shell.rs              # Shell runner (shared)
│   ├── task.rs               # Azure DevOps task runner
│   └── container.rs          # Container runner (shared)
├── tasks/
│   ├── mod.rs
│   ├── cache.rs              # Task cache (Azure DevOps tasks)
│   └── manifest.rs           # task.json parser
├── testing/
│   ├── mod.rs
│   ├── runner.rs             # Test runner
│   ├── assertions.rs         # Assertion engine
│   ├── parser.rs             # Test file parser
│   └── reporter.rs           # JUnit/TAP/terminal output
├── workflow/                  # GitHub Actions workflow support
│   ├── mod.rs
│   ├── models.rs             # Workflow, Job, Step structs (exists, to be extended)
│   ├── parser.rs             # YAML parsing (exists, to be extended)
│   ├── context.rs            # NEW: GitHub Actions context providers
│   └── trigger.rs            # NEW: Trigger parsing and simulation
└── actions/                   # NEW: GitHub Actions action support
    ├── mod.rs
    ├── resolver.rs            # Action resolution and download
    ├── cache.rs               # Action caching
    └── runners/
        ├── composite.rs       # Composite action runner
        ├── javascript.rs      # JavaScript action runner
        └── docker.rs          # Docker action runner
```

---

## Implementation Order

### Week 1: Foundation
1. [ ] Extend `workflow/models.rs` with full GitHub Actions structs
2. [ ] Extend `workflow/parser.rs` for full YAML parsing
3. [ ] Add tests for parsing real GitHub Actions workflows

### Week 2: Expressions
4. [ ] Add GitHub Actions-specific context names to expression evaluator
5. [ ] Add GitHub-specific functions (`hashFiles`, `toJSON`, `fromJSON`)
6. [ ] Add `success()`, `failure()`, `always()`, `cancelled()` status functions

### Week 3: Context & Execution
7. [ ] Implement GitHub Actions context system
8. [ ] Extend DAG executor for `needs` dependencies
9. [ ] Implement `if` conditional evaluation with GitHub Actions contexts
10. [ ] Implement output capture (`$GITHUB_OUTPUT`)

### Week 4: Matrix & Integration
11. [ ] Extend matrix expansion for `include`/`exclude`
12. [ ] Add secrets support (from `.env` file)
13. [ ] CLI integration for running workflows
14. [ ] End-to-end testing

### Future Weeks
15. [ ] Action resolution and execution
16. [ ] Service containers
17. [ ] Caching and artifacts
18. [ ] Reusable workflows

---

## Testing Strategy

### Unit Tests
- Expression lexer/parser (GitHub Actions contexts)
- Context evaluation
- Matrix expansion with include/exclude
- DAG ordering from `needs`

### Integration Tests
- Parse real GitHub Actions workflows from popular repos
- Execute simple workflows end-to-end
- Test conditional execution

### Example Workflows for Testing
```yaml
# test-fixtures/simple.yml
name: Simple CI
on: push
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - run: echo "Hello, World!"

# test-fixtures/matrix.yml
name: Matrix CI
on: push
jobs:
  test:
    strategy:
      matrix:
        node: [16, 18, 20]
    steps:
      - run: echo "Node ${{ matrix.node }}"

# test-fixtures/dependencies.yml
name: Job Dependencies
on: push
jobs:
  build:
    steps:
      - run: echo "Building..."
      - run: echo "result=success" >> $GITHUB_OUTPUT
        id: build
    outputs:
      result: ${{ steps.build.outputs.result }}
  
  test:
    needs: build
    steps:
      - run: echo "Build result was ${{ needs.build.outputs.result }}"
```
