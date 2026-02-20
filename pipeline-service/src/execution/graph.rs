// Execution Graph (DAG) Builder
// Builds a directed acyclic graph from pipeline definition for execution ordering

use crate::parser::models::{BoolOrExpression, DependsOn, Job, Pipeline, Stage, Variable};

use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt;

/// Error type for graph operations
#[derive(Debug, Clone)]
pub struct GraphError {
    pub message: String,
    pub kind: GraphErrorKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GraphErrorKind {
    /// Circular dependency detected
    CyclicDependency,
    /// Reference to unknown stage/job
    UnknownDependency,
    /// Invalid pipeline structure
    InvalidStructure,
}

impl fmt::Display for GraphError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "graph error: {}", self.message)
    }
}

impl std::error::Error for GraphError {}

impl GraphError {
    pub fn cyclic(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            kind: GraphErrorKind::CyclicDependency,
        }
    }

    pub fn unknown_dependency(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            kind: GraphErrorKind::UnknownDependency,
        }
    }

    pub fn invalid_structure(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            kind: GraphErrorKind::InvalidStructure,
        }
    }
}

/// Execution graph representing the DAG of stages and jobs
#[derive(Debug, Clone)]
pub struct ExecutionGraph {
    /// All stages in the pipeline
    pub stages: Vec<StageNode>,
    /// Quick lookup of stage index by name
    stage_indices: HashMap<String, usize>,
    /// Pipeline-level variables
    pub variables: Vec<Variable>,
}

/// A node representing a stage in the execution graph
#[derive(Debug, Clone)]
pub struct StageNode {
    /// Stage definition
    pub stage: Stage,
    /// Names of stages this stage depends on
    pub dependencies: Vec<String>,
    /// Jobs within this stage
    pub jobs: Vec<JobNode>,
    /// Quick lookup of job index by name
    job_indices: HashMap<String, usize>,
}

/// A node representing a job in the execution graph
#[derive(Debug, Clone)]
pub struct JobNode {
    /// Job definition
    pub job: Job,
    /// Names of jobs this job depends on (within the same stage)
    pub dependencies: Vec<String>,
    /// Matrix instances (empty if no matrix strategy)
    pub matrix_instances: Vec<super::matrix::MatrixInstance>,
}

impl ExecutionGraph {
    /// Build an execution graph from a pipeline definition
    pub fn from_pipeline(pipeline: &Pipeline) -> Result<Self, GraphError> {
        // Normalize pipeline to always have stages
        let stages = Self::normalize_to_stages(pipeline)?;

        // Build stage nodes
        let mut stage_nodes = Vec::with_capacity(stages.len());
        let mut stage_indices = HashMap::new();

        for (i, stage) in stages.iter().enumerate() {
            let stage_name = stage.stage.clone().unwrap_or_default();
            stage_indices.insert(stage_name.clone(), i);

            // Calculate stage dependencies
            let dependencies = Self::calculate_stage_dependencies(stage, i, &stages);

            // Build job nodes for this stage
            let jobs = Self::build_job_nodes(&stage.jobs)?;

            stage_nodes.push(StageNode {
                stage: stage.clone(),
                dependencies,
                jobs,
                job_indices: stage
                    .jobs
                    .iter()
                    .enumerate()
                    .filter_map(|(idx, j)| j.identifier().map(|name| (name.to_string(), idx)))
                    .collect(),
            });
        }

        let graph = Self {
            stages: stage_nodes,
            stage_indices,
            variables: pipeline.variables.clone(),
        };

        // Validate the graph
        graph.validate()?;

        Ok(graph)
    }

    /// Normalize pipeline to stage-based structure
    fn normalize_to_stages(pipeline: &Pipeline) -> Result<Vec<Stage>, GraphError> {
        // If we have stages, use them directly
        if !pipeline.stages.is_empty() {
            return Ok(pipeline.stages.clone());
        }

        // If we have jobs but no stages, create a single stage
        if !pipeline.jobs.is_empty() {
            return Ok(vec![Stage {
                stage: Some("__default".to_string()),
                display_name: None,
                depends_on: DependsOn::None,
                condition: None,
                variables: Vec::new(),
                jobs: pipeline.jobs.clone(),
                lock_behavior: None,
                template: None,
                parameters: HashMap::new(),
                pool: pipeline.pool.clone(),
                has_template_directives: false,
            }]);
        }

        // If we have steps but no jobs/stages, create a single job in a single stage
        if !pipeline.steps.is_empty() {
            let job = Job {
                job: Some("__default".to_string()),
                deployment: None,
                display_name: None,
                depends_on: DependsOn::None,
                condition: None,
                strategy: None,
                pool: pipeline.pool.clone(),
                container: None,
                services: HashMap::new(),
                variables: Vec::new(),
                steps: pipeline.steps.clone(),
                timeout_in_minutes: None,
                cancel_timeout_in_minutes: None,
                continue_on_error: BoolOrExpression::default(),
                workspace: None,
                uses: None,
                template: None,
                parameters: HashMap::new(),
                environment: None,
                has_template_directives: false,
            };

            return Ok(vec![Stage {
                stage: Some("__default".to_string()),
                display_name: None,
                depends_on: DependsOn::None,
                condition: None,
                variables: Vec::new(),
                jobs: vec![job],
                lock_behavior: None,
                template: None,
                parameters: HashMap::new(),
                pool: pipeline.pool.clone(),
                has_template_directives: false,
            }]);
        }

        // Empty pipeline
        Ok(Vec::new())
    }

    /// Calculate dependencies for a stage based on dependsOn field
    fn calculate_stage_dependencies(
        stage: &Stage,
        index: usize,
        all_stages: &[Stage],
    ) -> Vec<String> {
        match &stage.depends_on {
            DependsOn::Default => {
                // Default: depends on the previous stage (if any)
                if index > 0 {
                    vec![all_stages[index - 1].stage.clone().unwrap_or_default()]
                } else {
                    vec![]
                }
            }
            DependsOn::None => vec![],
            DependsOn::Single(name) => vec![name.clone()],
            DependsOn::Multiple(names) => names.clone(),
        }
    }

    /// Build job nodes for a stage
    fn build_job_nodes(jobs: &[Job]) -> Result<Vec<JobNode>, GraphError> {
        let mut job_nodes = Vec::with_capacity(jobs.len());
        let job_names: HashSet<_> = jobs.iter().filter_map(|j| j.identifier()).collect();

        for (i, job) in jobs.iter().enumerate() {
            let dependencies = Self::calculate_job_dependencies(job, i, jobs, &job_names)?;

            job_nodes.push(JobNode {
                job: job.clone(),
                dependencies,
                matrix_instances: Vec::new(), // Populated later by matrix expander
            });
        }

        Ok(job_nodes)
    }

    /// Calculate dependencies for a job based on dependsOn field
    fn calculate_job_dependencies(
        job: &Job,
        index: usize,
        all_jobs: &[Job],
        job_names: &HashSet<&str>,
    ) -> Result<Vec<String>, GraphError> {
        let deps = match &job.depends_on {
            DependsOn::Default => {
                // Default: depends on the previous job (if any)
                if index > 0 {
                    if let Some(prev_name) = all_jobs[index - 1].identifier() {
                        vec![prev_name.to_string()]
                    } else {
                        vec![]
                    }
                } else {
                    vec![]
                }
            }
            DependsOn::None => vec![],
            DependsOn::Single(name) => vec![name.clone()],
            DependsOn::Multiple(names) => names.clone(),
        };

        // Validate that all dependencies exist
        for dep in &deps {
            if !job_names.contains(dep.as_str()) {
                return Err(GraphError::unknown_dependency(format!(
                    "job '{}' depends on unknown job '{}'",
                    job.identifier().unwrap_or("unknown"),
                    dep
                )));
            }
        }

        Ok(deps)
    }

    /// Validate the execution graph (check for cycles and unknown dependencies)
    pub fn validate(&self) -> Result<(), GraphError> {
        // Validate stage dependencies exist
        let stage_names: HashSet<_> = self
            .stages
            .iter()
            .map(|s| s.stage.stage.as_deref().unwrap_or(""))
            .collect();

        for stage_node in &self.stages {
            for dep in &stage_node.dependencies {
                if !stage_names.contains(dep.as_str()) {
                    return Err(GraphError::unknown_dependency(format!(
                        "stage '{}' depends on unknown stage '{}'",
                        stage_node.stage.stage.as_deref().unwrap_or("unknown"),
                        dep
                    )));
                }
            }
        }

        // Check for cycles at stage level
        self.detect_stage_cycles()?;

        // Check for cycles at job level within each stage
        for stage_node in &self.stages {
            self.detect_job_cycles(stage_node)?;
        }

        Ok(())
    }

    /// Detect cycles in stage dependencies using DFS
    fn detect_stage_cycles(&self) -> Result<(), GraphError> {
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();

        for stage_node in &self.stages {
            if !visited.contains(stage_node.stage.stage.as_deref().unwrap_or("")) {
                if let Some(cycle) = self.dfs_stage_cycle(stage_node, &mut visited, &mut rec_stack)
                {
                    return Err(GraphError::cyclic(format!(
                        "circular dependency detected in stages: {}",
                        cycle.join(" -> ")
                    )));
                }
            }
        }

        Ok(())
    }

    fn dfs_stage_cycle(
        &self,
        node: &StageNode,
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
    ) -> Option<Vec<String>> {
        let name = node.stage.stage.clone().unwrap_or_default();
        visited.insert(name.clone());
        rec_stack.insert(name.clone());

        for dep in &node.dependencies {
            if !visited.contains(dep) {
                if let Some(stage_idx) = self.stage_indices.get(dep) {
                    if let Some(mut cycle) =
                        self.dfs_stage_cycle(&self.stages[*stage_idx], visited, rec_stack)
                    {
                        cycle.insert(0, name.clone());
                        return Some(cycle);
                    }
                }
            } else if rec_stack.contains(dep) {
                return Some(vec![name.clone(), dep.clone()]);
            }
        }

        rec_stack.remove(&name);
        None
    }

    /// Detect cycles in job dependencies within a stage using DFS
    fn detect_job_cycles(&self, stage: &StageNode) -> Result<(), GraphError> {
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();

        for job_node in &stage.jobs {
            let job_name = job_node.job.identifier().unwrap_or("unknown").to_string();
            if !visited.contains(&job_name) {
                if let Some(cycle) =
                    self.dfs_job_cycle(stage, job_node, &mut visited, &mut rec_stack)
                {
                    return Err(GraphError::cyclic(format!(
                        "circular dependency detected in jobs of stage '{}': {}",
                        stage.stage.stage.as_deref().unwrap_or("unknown"),
                        cycle.join(" -> ")
                    )));
                }
            }
        }

        Ok(())
    }

    #[allow(clippy::only_used_in_recursion)]
    fn dfs_job_cycle(
        &self,
        stage: &StageNode,
        node: &JobNode,
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
    ) -> Option<Vec<String>> {
        let name = node.job.identifier().unwrap_or("unknown").to_string();
        visited.insert(name.clone());
        rec_stack.insert(name.clone());

        for dep in &node.dependencies {
            if !visited.contains(dep) {
                if let Some(job_idx) = stage.job_indices.get(dep) {
                    if let Some(mut cycle) =
                        self.dfs_job_cycle(stage, &stage.jobs[*job_idx], visited, rec_stack)
                    {
                        cycle.insert(0, name.clone());
                        return Some(cycle);
                    }
                }
            } else if rec_stack.contains(dep) {
                return Some(vec![name.clone(), dep.clone()]);
            }
        }

        rec_stack.remove(&name);
        None
    }

    /// Get stages in topological order (respecting dependencies)
    pub fn topological_order(&self) -> Vec<&StageNode> {
        // Kahn's algorithm for topological sort
        let mut in_degree: HashMap<&str, usize> = HashMap::new();
        let mut adj_list: HashMap<&str, Vec<&str>> = HashMap::new();

        // Initialize
        for stage in &self.stages {
            let name = stage.stage.stage.as_deref().unwrap_or("");
            in_degree.entry(name).or_insert(0);
            adj_list.entry(name).or_default();

            for dep in &stage.dependencies {
                adj_list.entry(dep.as_str()).or_default().push(name);
                *in_degree.entry(name).or_insert(0) += 1;
            }
        }

        // Find all nodes with in-degree 0
        let mut queue: VecDeque<&str> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(&name, _)| name)
            .collect();

        let mut result = Vec::new();

        while let Some(name) = queue.pop_front() {
            if let Some(idx) = self.stage_indices.get(name) {
                result.push(&self.stages[*idx]);
            }

            if let Some(neighbors) = adj_list.get(name) {
                for &neighbor in neighbors {
                    if let Some(deg) = in_degree.get_mut(neighbor) {
                        *deg -= 1;
                        if *deg == 0 {
                            queue.push_back(neighbor);
                        }
                    }
                }
            }
        }

        result
    }

    /// Get a stage by name
    pub fn get_stage(&self, name: &str) -> Option<&StageNode> {
        self.stage_indices.get(name).map(|&idx| &self.stages[idx])
    }

    /// Get jobs in topological order for a stage
    pub fn jobs_topological_order<'a>(&self, stage: &'a StageNode) -> Vec<&'a JobNode> {
        let mut in_degree: HashMap<&str, usize> = HashMap::new();
        let mut adj_list: HashMap<&str, Vec<&str>> = HashMap::new();

        // Initialize
        for job in &stage.jobs {
            let name = job.job.identifier().unwrap_or("unknown");
            in_degree.entry(name).or_insert(0);
            adj_list.entry(name).or_default();

            for dep in &job.dependencies {
                adj_list.entry(dep.as_str()).or_default().push(name);
                *in_degree.entry(name).or_insert(0) += 1;
            }
        }

        // Find all nodes with in-degree 0
        let mut queue: VecDeque<&str> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(&name, _)| name)
            .collect();

        let mut result = Vec::new();

        while let Some(name) = queue.pop_front() {
            if let Some(idx) = stage.job_indices.get(name) {
                result.push(&stage.jobs[*idx]);
            }

            if let Some(neighbors) = adj_list.get(name) {
                for &neighbor in neighbors {
                    if let Some(deg) = in_degree.get_mut(neighbor) {
                        *deg -= 1;
                        if *deg == 0 {
                            queue.push_back(neighbor);
                        }
                    }
                }
            }
        }

        result
    }

    /// Get stages that can run in parallel (no dependencies between them)
    pub fn parallel_stages(&self) -> Vec<Vec<&StageNode>> {
        let mut levels: Vec<Vec<&StageNode>> = Vec::new();
        let mut assigned: HashMap<&str, usize> = HashMap::new();

        for stage in self.topological_order() {
            let name = stage.stage.stage.as_deref().unwrap_or("");
            let level = if stage.dependencies.is_empty() {
                0
            } else {
                stage
                    .dependencies
                    .iter()
                    .filter_map(|dep| assigned.get(dep.as_str()))
                    .max()
                    .map(|l| l + 1)
                    .unwrap_or(0)
            };

            assigned.insert(name, level);

            if level >= levels.len() {
                levels.resize(level + 1, Vec::new());
            }
            levels[level].push(stage);
        }

        levels
    }

    /// Get jobs that can run in parallel within a stage
    pub fn parallel_jobs<'a>(&self, stage: &'a StageNode) -> Vec<Vec<&'a JobNode>> {
        let mut levels: Vec<Vec<&'a JobNode>> = Vec::new();
        let mut assigned: HashMap<&str, usize> = HashMap::new();

        for job in self.jobs_topological_order(stage) {
            let name = job.job.identifier().unwrap_or("unknown");
            let level = if job.dependencies.is_empty() {
                0
            } else {
                job.dependencies
                    .iter()
                    .filter_map(|dep| assigned.get(dep.as_str()))
                    .max()
                    .map(|l| l + 1)
                    .unwrap_or(0)
            };

            assigned.insert(name, level);

            if level >= levels.len() {
                levels.resize(level + 1, Vec::new());
            }
            levels[level].push(job);
        }

        levels
    }
}

impl StageNode {
    /// Get a job by name
    pub fn get_job(&self, name: &str) -> Option<&JobNode> {
        self.job_indices.get(name).map(|&idx| &self.jobs[idx])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_pipeline_with_stages(stages: Vec<Stage>) -> Pipeline {
        Pipeline {
            stages,
            ..Default::default()
        }
    }

    fn make_stage(name: &str, depends_on: DependsOn) -> Stage {
        Stage {
            stage: Some(name.to_string()),
            display_name: None,
            depends_on,
            condition: None,
            variables: Vec::new(),
            jobs: vec![make_job("Job1", DependsOn::None)],
            lock_behavior: None,
            template: None,
            parameters: HashMap::new(),
            pool: None,
            has_template_directives: false,
        }
    }

    fn make_job(name: &str, depends_on: DependsOn) -> Job {
        Job {
            job: Some(name.to_string()),
            deployment: None,
            display_name: None,
            depends_on,
            condition: None,
            strategy: None,
            pool: None,
            container: None,
            services: HashMap::new(),
            variables: Vec::new(),
            steps: Vec::new(),
            timeout_in_minutes: None,
            cancel_timeout_in_minutes: None,
            continue_on_error: BoolOrExpression::default(),
            workspace: None,
            uses: None,
            template: None,
            parameters: HashMap::new(),
            environment: None,
            has_template_directives: false,
        }
    }

    #[test]
    fn test_simple_linear_stages() {
        let pipeline = make_pipeline_with_stages(vec![
            make_stage("Build", DependsOn::None),
            make_stage("Test", DependsOn::Default), // depends on Build
            make_stage("Deploy", DependsOn::Default), // depends on Test
        ]);

        let graph = ExecutionGraph::from_pipeline(&pipeline).unwrap();

        assert_eq!(graph.stages.len(), 3);
        assert!(graph.stages[0].dependencies.is_empty());
        assert_eq!(graph.stages[1].dependencies, vec!["Build"]);
        assert_eq!(graph.stages[2].dependencies, vec!["Test"]);

        // Topological order should be Build -> Test -> Deploy
        let order: Vec<_> = graph.topological_order();
        assert_eq!(order.len(), 3);
        assert_eq!(order[0].stage.stage, Some("Build".to_string()));
        assert_eq!(order[1].stage.stage, Some("Test".to_string()));
        assert_eq!(order[2].stage.stage, Some("Deploy".to_string()));
    }

    #[test]
    fn test_parallel_stages() {
        let pipeline = make_pipeline_with_stages(vec![
            make_stage("Build", DependsOn::None),
            make_stage("UnitTest", DependsOn::Single("Build".to_string())),
            make_stage("IntegrationTest", DependsOn::Single("Build".to_string())),
            make_stage(
                "Deploy",
                DependsOn::Multiple(vec!["UnitTest".to_string(), "IntegrationTest".to_string()]),
            ),
        ]);

        let graph = ExecutionGraph::from_pipeline(&pipeline).unwrap();

        let parallel = graph.parallel_stages();
        assert_eq!(parallel.len(), 3);

        // Level 0: Build
        assert_eq!(parallel[0].len(), 1);
        assert_eq!(parallel[0][0].stage.stage, Some("Build".to_string()));

        // Level 1: UnitTest, IntegrationTest (can run in parallel)
        assert_eq!(parallel[1].len(), 2);

        // Level 2: Deploy
        assert_eq!(parallel[2].len(), 1);
        assert_eq!(parallel[2][0].stage.stage, Some("Deploy".to_string()));
    }

    #[test]
    fn test_cycle_detection_stages() {
        let pipeline = make_pipeline_with_stages(vec![
            make_stage("A", DependsOn::Single("C".to_string())),
            make_stage("B", DependsOn::Single("A".to_string())),
            make_stage("C", DependsOn::Single("B".to_string())),
        ]);

        let result = ExecutionGraph::from_pipeline(&pipeline);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind, GraphErrorKind::CyclicDependency);
    }

    #[test]
    fn test_unknown_dependency() {
        let pipeline = make_pipeline_with_stages(vec![
            make_stage("Build", DependsOn::None),
            make_stage("Test", DependsOn::Single("Unknown".to_string())),
        ]);

        let result = ExecutionGraph::from_pipeline(&pipeline);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind, GraphErrorKind::UnknownDependency);
    }

    #[test]
    fn test_jobs_within_stage() {
        let mut stage = make_stage("Build", DependsOn::None);
        stage.jobs = vec![
            make_job("Compile", DependsOn::None),
            make_job("Lint", DependsOn::None),
            make_job(
                "Package",
                DependsOn::Multiple(vec!["Compile".to_string(), "Lint".to_string()]),
            ),
        ];

        let pipeline = make_pipeline_with_stages(vec![stage]);
        let graph = ExecutionGraph::from_pipeline(&pipeline).unwrap();

        let stage_node = &graph.stages[0];
        let parallel_jobs = graph.parallel_jobs(stage_node);

        // Level 0: Compile, Lint (can run in parallel)
        assert_eq!(parallel_jobs[0].len(), 2);

        // Level 1: Package
        assert_eq!(parallel_jobs[1].len(), 1);
        assert_eq!(parallel_jobs[1][0].job.identifier(), Some("Package"));
    }

    #[test]
    fn test_job_cycle_detection() {
        let mut stage = make_stage("Build", DependsOn::None);
        stage.jobs = vec![
            make_job("A", DependsOn::Single("C".to_string())),
            make_job("B", DependsOn::Single("A".to_string())),
            make_job("C", DependsOn::Single("B".to_string())),
        ];

        let pipeline = make_pipeline_with_stages(vec![stage]);
        let result = ExecutionGraph::from_pipeline(&pipeline);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind, GraphErrorKind::CyclicDependency);
    }

    #[test]
    fn test_normalize_steps_only_pipeline() {
        use crate::parser::models::{ScriptStep, Step, StepAction};

        let pipeline = Pipeline {
            steps: vec![Step {
                name: Some("echo".to_string()),
                display_name: Some("Echo Hello".to_string()),
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

        let graph = ExecutionGraph::from_pipeline(&pipeline).unwrap();

        assert_eq!(graph.stages.len(), 1);
        assert_eq!(graph.stages[0].stage.stage, Some("__default".to_string()));
        assert_eq!(graph.stages[0].jobs.len(), 1);
        assert_eq!(graph.stages[0].jobs[0].job.steps.len(), 1);
    }

    #[test]
    fn test_normalize_jobs_only_pipeline() {
        let pipeline = Pipeline {
            jobs: vec![
                make_job("Build", DependsOn::None),
                make_job("Test", DependsOn::Default),
            ],
            ..Default::default()
        };

        let graph = ExecutionGraph::from_pipeline(&pipeline).unwrap();

        assert_eq!(graph.stages.len(), 1);
        assert_eq!(graph.stages[0].stage.stage, Some("__default".to_string()));
        assert_eq!(graph.stages[0].jobs.len(), 2);
    }

    #[test]
    fn test_explicit_none_dependency() {
        let pipeline = make_pipeline_with_stages(vec![
            make_stage("Build", DependsOn::None),
            make_stage("Deploy", DependsOn::None), // Explicitly no dependencies
        ]);

        let graph = ExecutionGraph::from_pipeline(&pipeline).unwrap();

        // Both stages should have no dependencies and can run in parallel
        let parallel = graph.parallel_stages();
        assert_eq!(parallel.len(), 1);
        assert_eq!(parallel[0].len(), 2);
    }
}
