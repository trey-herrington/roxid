use crate::workflow::models::Workflow;
use crate::ServiceResult;

use std::fs;
use std::path::Path;

/// Parser for GitHub Actions workflow YAML files.
pub struct WorkflowParser;

impl WorkflowParser {
    /// Parse a workflow from a file path.
    pub fn from_file<P: AsRef<Path>>(path: P) -> ServiceResult<Workflow> {
        let content = fs::read_to_string(path)?;
        Self::parse(&content)
    }

    /// Parse a workflow from a YAML string.
    pub fn parse(content: &str) -> ServiceResult<Workflow> {
        let workflow: Workflow = serde_yaml::from_str(content)?;
        Ok(workflow)
    }

    /// Parse and validate a workflow from a YAML string.
    ///
    /// This performs additional validation beyond basic YAML parsing.
    pub fn parse_and_validate(content: &str) -> ServiceResult<Workflow> {
        let workflow = Self::parse(content)?;
        Self::validate(&workflow)?;
        Ok(workflow)
    }

    /// Validate a parsed workflow for semantic correctness.
    pub fn validate(workflow: &Workflow) -> ServiceResult<()> {
        // Validate job dependencies exist
        for (job_id, job) in &workflow.jobs {
            for needed_job in job.needs.to_vec() {
                if !workflow.jobs.contains_key(&needed_job) {
                    return Err(crate::ServiceError::InvalidInput(format!(
                        "Job '{}' depends on non-existent job '{}'",
                        job_id, needed_job
                    )));
                }
            }
        }

        // Validate no circular dependencies
        Self::check_circular_dependencies(workflow)?;

        // Validate each job has at least one step
        for (job_id, job) in &workflow.jobs {
            if job.steps.is_empty() {
                return Err(crate::ServiceError::InvalidInput(format!(
                    "Job '{}' has no steps",
                    job_id
                )));
            }
        }

        // Validate steps have either 'run' or 'uses' (not both, not neither)
        for (job_id, job) in &workflow.jobs {
            for (step_idx, step) in job.steps.iter().enumerate() {
                let has_run = step.run.is_some();
                let has_uses = step.uses.is_some();

                if !has_run && !has_uses {
                    let step_name = step.name.as_deref().unwrap_or("unnamed");
                    return Err(crate::ServiceError::InvalidInput(format!(
                        "Step {} '{}' in job '{}' must have either 'run' or 'uses'",
                        step_idx, step_name, job_id
                    )));
                }

                if has_run && has_uses {
                    let step_name = step.name.as_deref().unwrap_or("unnamed");
                    return Err(crate::ServiceError::InvalidInput(format!(
                        "Step {} '{}' in job '{}' cannot have both 'run' and 'uses'",
                        step_idx, step_name, job_id
                    )));
                }
            }
        }

        Ok(())
    }

    /// Check for circular dependencies in job `needs`.
    fn check_circular_dependencies(workflow: &Workflow) -> ServiceResult<()> {
        use std::collections::HashSet;

        fn has_cycle(
            job_id: &str,
            workflow: &Workflow,
            visited: &mut HashSet<String>,
            rec_stack: &mut HashSet<String>,
        ) -> Option<String> {
            visited.insert(job_id.to_string());
            rec_stack.insert(job_id.to_string());

            if let Some(job) = workflow.jobs.get(job_id) {
                for needed_job in job.needs.to_vec() {
                    if !visited.contains(&needed_job) {
                        if let Some(cycle) = has_cycle(&needed_job, workflow, visited, rec_stack) {
                            return Some(cycle);
                        }
                    } else if rec_stack.contains(&needed_job) {
                        return Some(format!("{} -> {}", job_id, needed_job));
                    }
                }
            }

            rec_stack.remove(job_id);
            None
        }

        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();

        for job_id in workflow.jobs.keys() {
            if !visited.contains(job_id) {
                if let Some(cycle) = has_cycle(job_id, workflow, &mut visited, &mut rec_stack) {
                    return Err(crate::ServiceError::InvalidInput(format!(
                        "Circular dependency detected: {}",
                        cycle
                    )));
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_minimal_workflow() {
        let yaml = r#"
on: push
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - run: echo "Hello"
"#;
        let workflow = WorkflowParser::parse(yaml).unwrap();
        assert!(workflow.name.is_none());
        assert!(workflow.jobs.contains_key("build"));
    }

    #[test]
    fn test_validate_missing_dependency() {
        let yaml = r#"
on: push
jobs:
  deploy:
    needs: build
    runs-on: ubuntu-latest
    steps:
      - run: echo "Deploying"
"#;
        let workflow = WorkflowParser::parse(yaml).unwrap();
        let result = WorkflowParser::validate(&workflow);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("non-existent job"));
    }

    #[test]
    fn test_validate_circular_dependency() {
        let yaml = r#"
on: push
jobs:
  a:
    needs: c
    runs-on: ubuntu-latest
    steps:
      - run: echo "a"
  b:
    needs: a
    runs-on: ubuntu-latest
    steps:
      - run: echo "b"
  c:
    needs: b
    runs-on: ubuntu-latest
    steps:
      - run: echo "c"
"#;
        let workflow = WorkflowParser::parse(yaml).unwrap();
        let result = WorkflowParser::validate(&workflow);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Circular dependency"));
    }

    #[test]
    fn test_validate_empty_job() {
        let yaml = r#"
on: push
jobs:
  empty:
    runs-on: ubuntu-latest
    steps: []
"#;
        let workflow = WorkflowParser::parse(yaml).unwrap();
        let result = WorkflowParser::validate(&workflow);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("no steps"));
    }

    #[test]
    fn test_validate_step_without_run_or_uses() {
        let yaml = r#"
on: push
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - name: Invalid step
        env:
          FOO: bar
"#;
        let workflow = WorkflowParser::parse(yaml).unwrap();
        let result = WorkflowParser::validate(&workflow);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("must have either 'run' or 'uses'"));
    }

    #[test]
    fn test_validate_step_with_both_run_and_uses() {
        let yaml = r#"
on: push
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - name: Invalid step
        run: echo "Hello"
        uses: actions/checkout@v4
"#;
        let workflow = WorkflowParser::parse(yaml).unwrap();
        let result = WorkflowParser::validate(&workflow);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("cannot have both 'run' and 'uses'"));
    }

    #[test]
    fn test_parse_and_validate_success() {
        let yaml = r#"
name: CI
on: push
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: cargo build
  test:
    needs: build
    runs-on: ubuntu-latest
    steps:
      - run: cargo test
"#;
        let result = WorkflowParser::parse_and_validate(yaml);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_complex_workflow() {
        let yaml = r#"
name: Rust CI

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main]

env:
  CARGO_TERM_COLOR: always

jobs:
  check:
    name: Check
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Run check
        run: cargo check --all-features

  test:
    name: Test Suite
    needs: check
    runs-on: ubuntu-latest
    strategy:
      matrix:
        rust: [stable, beta, nightly]
    steps:
      - uses: actions/checkout@v4
      - name: Run tests
        run: cargo test --all-features
        env:
          RUST_BACKTRACE: 1

  fmt:
    name: Rustfmt
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: cargo fmt --all -- --check

  clippy:
    name: Clippy
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: cargo clippy -- -D warnings

  deploy:
    name: Deploy
    needs: [test, fmt, clippy]
    runs-on: ubuntu-latest
    if: github.ref == 'refs/heads/main'
    steps:
      - uses: actions/checkout@v4
      - name: Deploy
        run: echo "Deploying..."
        env:
          DEPLOY_TOKEN: ${{ secrets.DEPLOY_TOKEN }}
"#;
        let result = WorkflowParser::parse_and_validate(yaml);
        assert!(result.is_ok());

        let workflow = result.unwrap();
        assert_eq!(workflow.name, Some("Rust CI".to_string()));
        assert_eq!(workflow.jobs.len(), 5);

        // Check job dependencies
        let test = workflow.jobs.get("test").unwrap();
        assert_eq!(test.needs.to_vec(), vec!["check"]);

        let deploy = workflow.jobs.get("deploy").unwrap();
        assert_eq!(deploy.needs.to_vec(), vec!["test", "fmt", "clippy"]);

        // Check matrix
        let strategy = test.strategy.as_ref().unwrap();
        let matrix = strategy.matrix.as_ref().unwrap();
        assert!(matrix.dimensions.contains_key("rust"));
    }
}
