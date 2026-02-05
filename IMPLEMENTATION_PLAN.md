# GitHub Actions Compatibility Implementation Plan

This document outlines the plan to extend Roxid to support GitHub Actions workflow syntax.

## Phase 1: Core Schema (P0)

### 1.1 Extend Models for GitHub Actions Structure

Update `pipeline-service/src/pipeline/models.rs` to support:

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

Create `pipeline-service/src/expression/` module:

```
expression/
├── mod.rs
├── lexer.rs      # Tokenize ${{ ... }} expressions
├── parser.rs     # Parse expression AST
├── evaluator.rs  # Evaluate expressions against context
├── functions.rs  # Built-in functions (contains, startsWith, etc.)
└── context.rs    # Context providers (github, env, job, steps, etc.)
```

**Expression Features:**
- Literals: strings, numbers, booleans, null
- Property access: `github.event_name`, `steps.build.outputs.result`
- Operators: `==`, `!=`, `<`, `>`, `<=`, `>=`, `&&`, `||`, `!`
- Functions: `contains()`, `startsWith()`, `endsWith()`, `format()`, `join()`, `toJSON()`, `fromJSON()`, `hashFiles()`
- Status functions: `success()`, `failure()`, `always()`, `cancelled()`

### 1.3 Context System

Implement context providers:

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

- Parse `needs` dependencies to build execution graph
- Topological sort for execution order
- Parallel execution of independent jobs
- Job output passing via `needs.job_id.outputs.output_name`

### 2.2 Matrix Expansion

- Expand `strategy.matrix` into job instances
- Support `include` and `exclude`
- Pass matrix values to context
- Handle `fail-fast` and `max-parallel`

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

- **JavaScript actions**: Run with Node.js
- **Composite actions**: Expand steps inline
- **Docker actions**: Run in container

## Phase 4: Advanced Features (P3)

### 4.1 Service Containers

- Start Docker services before job
- Network configuration
- Health checks

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
├── grpc.rs
├── workflow/                    # NEW: GitHub Actions workflow support
│   ├── mod.rs
│   ├── models.rs               # Workflow, Job, Step structs
│   ├── parser.rs               # YAML parsing with GitHub Actions schema
│   ├── validator.rs            # Schema validation
│   └── trigger.rs              # Trigger parsing and simulation
├── expression/                  # NEW: Expression evaluation
│   ├── mod.rs
│   ├── lexer.rs
│   ├── parser.rs
│   ├── evaluator.rs
│   ├── functions.rs
│   └── context.rs
├── executor/                    # NEW: Enhanced execution engine
│   ├── mod.rs
│   ├── job_executor.rs         # Job-level execution with DAG
│   ├── step_executor.rs        # Step-level execution
│   ├── matrix.rs               # Matrix expansion
│   └── output.rs               # Output capture (GITHUB_OUTPUT, GITHUB_ENV)
├── actions/                     # NEW: Action support
│   ├── mod.rs
│   ├── resolver.rs             # Action resolution and download
│   ├── cache.rs                # Action caching
│   └── runners/
│       ├── composite.rs
│       ├── javascript.rs
│       └── docker.rs
├── pipeline/                    # KEEP: Original simple pipeline (legacy)
│   ├── mod.rs
│   ├── models.rs
│   ├── parser.rs
│   ├── executor.rs
│   └── runners/
│       └── shell.rs
└── services/                    # NEW: Service containers
    ├── mod.rs
    └── docker.rs
```

---

## Implementation Order

### Week 1: Foundation
1. [ ] Create `workflow/models.rs` with GitHub Actions structs
2. [ ] Create `workflow/parser.rs` for YAML parsing
3. [ ] Add tests for parsing real GitHub Actions workflows

### Week 2: Expressions
4. [ ] Implement expression lexer
5. [ ] Implement expression parser (AST)
6. [ ] Implement expression evaluator
7. [ ] Add built-in functions

### Week 3: Context & Execution
8. [ ] Implement context system
9. [ ] Create job DAG executor
10. [ ] Implement `if` conditional evaluation
11. [ ] Implement output capture (GITHUB_OUTPUT)

### Week 4: Matrix & Integration
12. [ ] Implement matrix expansion
13. [ ] Add secrets support (from .env file)
14. [ ] CLI integration for running workflows
15. [ ] End-to-end testing

### Future Weeks
16. [ ] Action resolution and execution
17. [ ] Service containers
18. [ ] Caching and artifacts
19. [ ] Reusable workflows

---

## Testing Strategy

### Unit Tests
- Expression lexer/parser
- Context evaluation
- Matrix expansion
- DAG ordering

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
