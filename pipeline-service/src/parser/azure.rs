// Azure DevOps Pipeline YAML Parser
// Parses azure-pipelines.yml files with template support

use crate::parser::error::{ParseError, ParseResult, ValidationError};
use crate::parser::models::*;

use std::fs;
use std::path::Path;

/// Azure DevOps pipeline parser
pub struct AzureParser;

impl AzureParser {
    /// Parse pipeline from YAML string
    pub fn parse(content: &str) -> ParseResult<Pipeline> {
        let pipeline: Pipeline =
            serde_yaml::from_str(content).map_err(|e| ParseError::from_yaml_error(&e, content))?;

        Ok(pipeline)
    }

    /// Parse pipeline from file
    pub fn parse_file<P: AsRef<Path>>(path: P) -> ParseResult<Pipeline> {
        let path = path.as_ref();
        let content = fs::read_to_string(path).map_err(|e| {
            ParseError::new(format!("failed to read file: {}", e), 0, 0)
                .with_kind(crate::parser::error::ParseErrorKind::IoError)
        })?;

        Self::parse(&content)
    }

    /// Parse pipeline with template resolution
    ///
    /// Resolves all template references (step, job, stage, variable, extends)
    /// relative to the given repository root directory.
    pub fn parse_with_templates<P: AsRef<Path>>(path: P, repo_root: P) -> ParseResult<Pipeline> {
        let pipeline = Self::parse_file(&path)?;
        let mut engine =
            crate::parser::template::TemplateEngine::new(repo_root.as_ref().to_path_buf());
        engine.resolve_pipeline(pipeline)
    }
}

/// Validator for parsed pipelines
pub struct PipelineValidator;

impl PipelineValidator {
    /// Validate a parsed pipeline for semantic correctness
    pub fn validate(pipeline: &Pipeline) -> Result<(), Vec<ValidationError>> {
        let mut errors = Vec::new();

        // Check for valid structure (must have stages, jobs, or steps)
        if pipeline.stages.is_empty()
            && pipeline.jobs.is_empty()
            && pipeline.steps.is_empty()
            && pipeline.extends.is_none()
        {
            errors.push(ValidationError::new(
                "pipeline must have stages, jobs, steps, or extends",
                "pipeline",
            ));
        }

        // Validate stages
        for (i, stage) in pipeline.stages.iter().enumerate() {
            Self::validate_stage(stage, &format!("stages[{}]", i), &mut errors);
        }

        // Validate jobs
        for (i, job) in pipeline.jobs.iter().enumerate() {
            Self::validate_job(job, &format!("jobs[{}]", i), &mut errors);
        }

        // Validate steps
        for (i, step) in pipeline.steps.iter().enumerate() {
            Self::validate_step(step, &format!("steps[{}]", i), &mut errors);
        }

        // Validate dependencies
        Self::validate_stage_dependencies(&pipeline.stages, &mut errors);
        Self::validate_job_dependencies(&pipeline.jobs, &mut errors);

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    fn validate_stage(stage: &Stage, path: &str, errors: &mut Vec<ValidationError>) {
        // Stage must have jobs or template
        if stage.jobs.is_empty() && stage.template.is_none() {
            errors.push(
                ValidationError::new("stage must have jobs or reference a template", path)
                    .with_suggestion("add 'jobs:' or 'template:' to the stage"),
            );
        }

        // Validate jobs within the stage
        for (i, job) in stage.jobs.iter().enumerate() {
            Self::validate_job(job, &format!("{}.jobs[{}]", path, i), errors);
        }
    }

    fn validate_job(job: &Job, path: &str, errors: &mut Vec<ValidationError>) {
        // Job must have identifier
        if job.job.is_none() && job.deployment.is_none() && job.template.is_none() {
            errors.push(
                ValidationError::new("job must have 'job:', 'deployment:', or 'template:'", path)
                    .with_suggestion("add 'job: MyJobName' to identify this job"),
            );
        }

        // Job must have steps (unless template or deployment)
        if job.steps.is_empty() && job.template.is_none() && job.deployment.is_none() {
            errors.push(
                ValidationError::new("job must have steps", path)
                    .with_suggestion("add 'steps:' to define what the job should do"),
            );
        }

        // Validate steps within the job
        for (i, step) in job.steps.iter().enumerate() {
            Self::validate_step(step, &format!("{}.steps[{}]", path, i), errors);
        }
    }

    fn validate_step(_step: &Step, _path: &str, _errors: &mut Vec<ValidationError>) {
        // Steps are validated during parsing due to StepAction enum
        // Additional semantic validation can be added here
    }

    fn validate_stage_dependencies(stages: &[Stage], errors: &mut Vec<ValidationError>) {
        let stage_names: Vec<&str> = stages.iter().map(|s| s.stage.as_str()).collect();

        for stage in stages {
            for dep in stage.depends_on.as_vec() {
                if !stage_names.contains(&dep.as_str()) {
                    errors.push(
                        ValidationError::new(
                            format!("stage '{}' depends on unknown stage '{}'", stage.stage, dep),
                            format!("stages.{}.dependsOn", stage.stage),
                        )
                        .with_suggestion(format!("available stages: {}", stage_names.join(", "))),
                    );
                }
            }
        }

        // Check for circular dependencies
        if let Err(cycle) = Self::detect_cycles(&stage_names, |name| {
            stages
                .iter()
                .find(|s| s.stage == name)
                .map(|s| s.depends_on.as_vec())
                .unwrap_or_default()
        }) {
            errors.push(ValidationError::new(
                format!("circular dependency detected: {}", cycle.join(" -> ")),
                "stages",
            ));
        }
    }

    fn validate_job_dependencies(jobs: &[Job], errors: &mut Vec<ValidationError>) {
        let job_names: Vec<&str> = jobs.iter().filter_map(|j| j.identifier()).collect();

        for job in jobs {
            let Some(job_name) = job.identifier() else {
                continue;
            };

            for dep in job.depends_on.as_vec() {
                if !job_names.contains(&dep.as_str()) {
                    errors.push(
                        ValidationError::new(
                            format!("job '{}' depends on unknown job '{}'", job_name, dep),
                            format!("jobs.{}.dependsOn", job_name),
                        )
                        .with_suggestion(format!("available jobs: {}", job_names.join(", "))),
                    );
                }
            }
        }

        // Check for circular dependencies
        if let Err(cycle) = Self::detect_cycles(&job_names, |name| {
            jobs.iter()
                .find(|j| j.identifier() == Some(name))
                .map(|j| j.depends_on.as_vec())
                .unwrap_or_default()
        }) {
            errors.push(ValidationError::new(
                format!("circular dependency detected: {}", cycle.join(" -> ")),
                "jobs",
            ));
        }
    }

    /// Detect cycles in a dependency graph using DFS
    fn detect_cycles<F>(nodes: &[&str], get_deps: F) -> Result<(), Vec<String>>
    where
        F: Fn(&str) -> Vec<String>,
    {
        #[derive(Clone, Copy, PartialEq)]
        enum NodeState {
            Unvisited,
            Visiting,
            Visited,
        }

        let mut states: std::collections::HashMap<String, NodeState> = nodes
            .iter()
            .map(|n| (n.to_string(), NodeState::Unvisited))
            .collect();
        let mut path: Vec<String> = Vec::new();

        fn visit<F>(
            node: &str,
            states: &mut std::collections::HashMap<String, NodeState>,
            path: &mut Vec<String>,
            get_deps: &F,
        ) -> Result<(), Vec<String>>
        where
            F: Fn(&str) -> Vec<String>,
        {
            match states.get(node) {
                Some(NodeState::Visiting) => {
                    // Found a cycle
                    path.push(node.to_string());
                    return Err(path.clone());
                }
                Some(NodeState::Visited) => return Ok(()),
                _ => {}
            }

            states.insert(node.to_string(), NodeState::Visiting);
            path.push(node.to_string());

            for dep in get_deps(node) {
                visit(&dep, states, path, get_deps)?;
            }

            path.pop();
            states.insert(node.to_string(), NodeState::Visited);
            Ok(())
        }

        for node in nodes {
            visit(node, &mut states, &mut path, &get_deps)?;
        }

        Ok(())
    }
}

/// Normalize a pipeline to the full stages/jobs/steps structure
pub fn normalize_pipeline(mut pipeline: Pipeline) -> Pipeline {
    // If pipeline has direct steps (no jobs/stages), wrap in default job/stage
    if !pipeline.steps.is_empty() && pipeline.jobs.is_empty() && pipeline.stages.is_empty() {
        pipeline.jobs = vec![Job {
            job: Some("Job".to_string()),
            deployment: None,
            display_name: None,
            depends_on: DependsOn::Default,
            condition: None,
            strategy: None,
            pool: pipeline.pool.clone(),
            container: None,
            services: std::collections::HashMap::new(),
            variables: Vec::new(),
            steps: std::mem::take(&mut pipeline.steps),
            timeout_in_minutes: None,
            cancel_timeout_in_minutes: None,
            continue_on_error: false,
            workspace: None,
            uses: None,
            template: None,
            parameters: std::collections::HashMap::new(),
            environment: None,
        }];
    }

    // If pipeline has direct jobs (no stages), wrap in default stage
    if !pipeline.jobs.is_empty() && pipeline.stages.is_empty() {
        pipeline.stages = vec![Stage {
            stage: "Build".to_string(),
            display_name: None,
            depends_on: DependsOn::Default,
            condition: None,
            variables: Vec::new(),
            jobs: std::mem::take(&mut pipeline.jobs),
            lock_behavior: None,
            template: None,
            parameters: std::collections::HashMap::new(),
            pool: pipeline.pool.clone(),
        }];
    }

    pipeline
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_pipeline() {
        let yaml = r#"
trigger:
  - main

pool:
  vmImage: ubuntu-latest

steps:
  - script: echo Hello, world!
    displayName: Run a one-line script
"#;
        let pipeline = AzureParser::parse(yaml).unwrap();
        assert!(!pipeline.steps.is_empty());
    }

    #[test]
    fn test_parse_jobs_pipeline() {
        let yaml = r#"
trigger:
  - main

jobs:
  - job: Build
    pool:
      vmImage: ubuntu-latest
    steps:
      - script: cargo build
        displayName: Build

  - job: Test
    dependsOn: Build
    pool:
      vmImage: ubuntu-latest
    steps:
      - script: cargo test
        displayName: Test
"#;
        let pipeline = AzureParser::parse(yaml).unwrap();
        assert_eq!(pipeline.jobs.len(), 2);
        assert_eq!(pipeline.jobs[0].job, Some("Build".to_string()));
    }

    #[test]
    fn test_parse_stages_pipeline() {
        let yaml = r#"
trigger:
  - main

stages:
  - stage: Build
    jobs:
      - job: BuildJob
        pool:
          vmImage: ubuntu-latest
        steps:
          - script: cargo build

  - stage: Deploy
    dependsOn: Build
    jobs:
      - job: DeployJob
        pool:
          vmImage: ubuntu-latest
        steps:
          - script: echo deploying
"#;
        let pipeline = AzureParser::parse(yaml).unwrap();
        assert_eq!(pipeline.stages.len(), 2);
        assert_eq!(pipeline.stages[0].stage, "Build");
        assert_eq!(pipeline.stages[1].stage, "Deploy");
    }

    #[test]
    fn test_parse_variables() {
        let yaml = r#"
variables:
  buildConfiguration: Release
  buildPlatform: Any CPU

steps:
  - script: echo $(buildConfiguration)
"#;
        let pipeline = AzureParser::parse(yaml).unwrap();
        assert_eq!(pipeline.variables.len(), 2);
    }

    #[test]
    fn test_parse_task_step() {
        let yaml = r#"
steps:
  - task: Bash@3
    inputs:
      targetType: inline
      script: echo Hello
"#;
        let pipeline = AzureParser::parse(yaml).unwrap();
        assert_eq!(pipeline.steps.len(), 1);
    }

    #[test]
    fn test_parse_matrix_strategy() {
        let yaml = r#"
jobs:
  - job: Build
    strategy:
      matrix:
        linux:
          vmImage: ubuntu-latest
        mac:
          vmImage: macos-latest
        windows:
          vmImage: windows-latest
      maxParallel: 3
    pool:
      vmImage: $(vmImage)
    steps:
      - script: echo Building on $(Agent.OS)
"#;
        let pipeline = AzureParser::parse(yaml).unwrap();
        assert!(pipeline.jobs[0].strategy.is_some());
    }

    #[test]
    fn test_normalize_steps_only() {
        let yaml = r#"
steps:
  - script: echo Hello
"#;
        let pipeline = AzureParser::parse(yaml).unwrap();
        let normalized = normalize_pipeline(pipeline);

        assert_eq!(normalized.stages.len(), 1);
        assert_eq!(normalized.stages[0].jobs.len(), 1);
        assert_eq!(normalized.stages[0].jobs[0].steps.len(), 1);
    }

    #[test]
    fn test_validate_circular_dependency() {
        let yaml = r#"
stages:
  - stage: A
    dependsOn: C
    jobs:
      - job: J1
        steps:
          - script: echo A

  - stage: B
    dependsOn: A
    jobs:
      - job: J2
        steps:
          - script: echo B

  - stage: C
    dependsOn: B
    jobs:
      - job: J3
        steps:
          - script: echo C
"#;
        let pipeline = AzureParser::parse(yaml).unwrap();
        let result = PipelineValidator::validate(&pipeline);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.message.contains("circular")));
    }

    #[test]
    fn test_parse_error_context() {
        let yaml = r#"
trigger:
  - main

jobs:
  - job: Build
    pool:
      vmImage: ubuntu-latest
    # Missing steps
"#;
        // This should parse but fail validation
        let pipeline = AzureParser::parse(yaml).unwrap();
        let result = PipelineValidator::validate(&pipeline);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_checkout_step() {
        let yaml = r#"
steps:
  - checkout: self
    clean: true
    fetchDepth: 1
"#;
        let pipeline = AzureParser::parse(yaml).unwrap();
        assert_eq!(pipeline.steps.len(), 1);
    }

    #[test]
    fn test_parse_container_job() {
        let yaml = r#"
jobs:
  - job: Build
    container: ubuntu:20.04
    steps:
      - script: echo Hello from container
"#;
        let pipeline = AzureParser::parse(yaml).unwrap();
        assert!(pipeline.jobs[0].container.is_some());
    }
}
