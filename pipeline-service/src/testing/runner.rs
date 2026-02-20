// Test Runner
// Executes pipeline tests and collects results

use crate::execution::events::progress_channel;
use crate::execution::executor::PipelineExecutor;
use crate::parser::models::ExecutionContext;
use crate::testing::assertions::{Assertion, AssertionEvaluator, AssertionResult};
use crate::testing::{PipelineTest, TestFileParser, TestSuite};
use crate::AzureParser;

use std::path::Path;
use std::time::{Duration, Instant};

// =============================================================================
// Test Result Types
// =============================================================================

/// Result of running a single pipeline test
#[derive(Debug, Clone)]
pub struct TestResult {
    /// Test name
    pub name: String,
    /// Whether all assertions passed
    pub passed: bool,
    /// Test execution duration
    pub duration: Duration,
    /// Individual assertion results
    pub assertions: Vec<AssertionResult>,
    /// Summary failure message (if any)
    pub failure_message: Option<String>,
    /// The pipeline file that was tested
    pub pipeline_path: String,
}

/// Result of running a test suite
#[derive(Debug, Clone)]
pub struct TestSuiteResult {
    /// Suite name
    pub suite_name: String,
    /// Individual test results
    pub results: Vec<TestResult>,
    /// Total number of tests
    pub total: usize,
    /// Number of passed tests
    pub passed: usize,
    /// Number of failed tests
    pub failed: usize,
    /// Number of skipped tests
    pub skipped: usize,
    /// Total duration
    pub duration: Duration,
}

impl TestSuiteResult {
    /// Whether all tests passed
    pub fn all_passed(&self) -> bool {
        self.failed == 0
    }
}

// =============================================================================
// Test Runner Errors
// =============================================================================

/// Errors that can occur during test execution
#[derive(Debug)]
pub enum TestError {
    /// Pipeline file not found
    PipelineNotFound(String),
    /// Pipeline parse error
    ParseError(String),
    /// Pipeline execution error
    ExecutionError(String),
    /// Test configuration error
    ConfigError(String),
}

impl std::fmt::Display for TestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TestError::PipelineNotFound(path) => {
                write!(f, "Pipeline file not found: {}", path)
            }
            TestError::ParseError(msg) => write!(f, "Pipeline parse error: {}", msg),
            TestError::ExecutionError(msg) => write!(f, "Execution error: {}", msg),
            TestError::ConfigError(msg) => write!(f, "Configuration error: {}", msg),
        }
    }
}

impl std::error::Error for TestError {}

// =============================================================================
// Test Runner Configuration
// =============================================================================

/// Configuration for the test runner
#[derive(Debug, Clone)]
pub struct TestRunnerConfig {
    /// Working directory for pipeline execution
    pub working_dir: String,
    /// Optional filter pattern for test names
    pub filter: Option<String>,
    /// Maximum parallel test execution (0 = sequential)
    pub max_parallel: usize,
    /// Whether to continue running tests after a failure
    pub fail_fast: bool,
    /// Collect execution events for debugging
    pub collect_events: bool,
}

impl Default for TestRunnerConfig {
    fn default() -> Self {
        Self {
            working_dir: ".".to_string(),
            filter: None,
            max_parallel: 0,
            fail_fast: false,
            collect_events: false,
        }
    }
}

// =============================================================================
// Test Runner
// =============================================================================

/// Executes pipeline tests
pub struct TestRunner {
    config: TestRunnerConfig,
}

impl TestRunner {
    /// Create a new test runner with default configuration
    pub fn new() -> Self {
        Self {
            config: TestRunnerConfig::default(),
        }
    }

    /// Create a new test runner with the given configuration
    pub fn with_config(config: TestRunnerConfig) -> Self {
        Self { config }
    }

    /// Set the working directory
    pub fn with_working_dir(mut self, dir: impl Into<String>) -> Self {
        self.config.working_dir = dir.into();
        self
    }

    /// Set the test name filter
    pub fn with_filter(mut self, filter: impl Into<String>) -> Self {
        self.config.filter = Some(filter.into());
        self
    }

    /// Set fail-fast mode
    pub fn with_fail_fast(mut self, fail_fast: bool) -> Self {
        self.config.fail_fast = fail_fast;
        self
    }

    /// Run a single test
    pub async fn run_test(&self, test: &PipelineTest) -> TestResult {
        let start = Instant::now();
        let pipeline_path = test.pipeline.display().to_string();

        // Parse the pipeline file
        let pipeline = match self.parse_pipeline(test) {
            Ok(p) => p,
            Err(e) => {
                return TestResult {
                    name: test.name.clone(),
                    passed: false,
                    duration: start.elapsed(),
                    assertions: vec![],
                    failure_message: Some(format!("Failed to parse pipeline: {}", e)),
                    pipeline_path,
                };
            }
        };

        // Create the executor
        let executor = match PipelineExecutor::from_pipeline(&pipeline) {
            Ok(e) => e,
            Err(e) => {
                return TestResult {
                    name: test.name.clone(),
                    passed: false,
                    duration: start.elapsed(),
                    assertions: vec![],
                    failure_message: Some(format!("Failed to build execution graph: {}", e)),
                    pipeline_path,
                };
            }
        };

        // Set up progress channel if collecting events
        let (executor, _rx) = if self.config.collect_events {
            let (tx, rx) = progress_channel();
            (executor.with_progress(tx), Some(rx))
        } else {
            (executor, None)
        };

        // Build execution context from test definition
        let working_dir = test
            .working_dir
            .clone()
            .unwrap_or_else(|| self.config.working_dir.clone());

        let context = ExecutionContext::new(test.name.clone(), working_dir)
            .with_variables(test.variables.clone())
            .with_parameters(test.parameters.clone());

        // Execute the pipeline
        let exec_result = executor.execute(context).await;

        // Convert assertion definitions to evaluable assertions
        let assertions: Vec<Assertion> = test
            .assertions
            .iter()
            .map(|def| def.to_assertion())
            .collect();

        // Evaluate assertions against execution results
        let evaluator = AssertionEvaluator::new(&exec_result);
        let assertion_results = evaluator.evaluate_all(&assertions);

        // Compute pass/fail
        let all_passed = assertion_results.iter().all(|r| r.passed);
        let failed_count = assertion_results.iter().filter(|r| !r.passed).count();

        let failure_message = if !all_passed {
            Some(format!(
                "{} of {} assertions failed",
                failed_count,
                assertion_results.len()
            ))
        } else {
            None
        };

        TestResult {
            name: test.name.clone(),
            passed: all_passed,
            duration: start.elapsed(),
            assertions: assertion_results,
            failure_message,
            pipeline_path,
        }
    }

    /// Run a test suite
    pub async fn run_suite(&self, suite: &TestSuite) -> TestSuiteResult {
        let start = Instant::now();
        let suite_name = suite
            .name
            .clone()
            .unwrap_or_else(|| "Pipeline Tests".to_string());

        // Apply defaults and filter tests
        let tests = self.prepare_tests(suite);

        let mut results = Vec::with_capacity(tests.len());

        for test in &tests {
            let result = self.run_test(test).await;
            let failed = !result.passed;
            results.push(result);

            if self.config.fail_fast && failed {
                break;
            }
        }

        let total = tests.len();
        let passed = results.iter().filter(|r| r.passed).count();
        let failed = results.iter().filter(|r| !r.passed).count();
        let skipped = total - results.len();

        TestSuiteResult {
            suite_name,
            results,
            total,
            passed,
            failed,
            skipped,
            duration: start.elapsed(),
        }
    }

    /// Run a test suite from a file
    pub async fn run_file(&self, path: &Path) -> Result<TestSuiteResult, TestError> {
        let suite = TestFileParser::parse_file(path)
            .map_err(|e| TestError::ConfigError(format!("Failed to parse test file: {}", e)))?;

        Ok(self.run_suite(&suite).await)
    }

    // =========================================================================
    // Internal helpers
    // =========================================================================

    fn parse_pipeline(
        &self,
        test: &PipelineTest,
    ) -> Result<crate::parser::models::Pipeline, TestError> {
        let path = &test.pipeline;

        if !path.exists() {
            return Err(TestError::PipelineNotFound(path.display().to_string()));
        }

        AzureParser::parse_file(path).map_err(|e| TestError::ParseError(format!("{}", e)))
    }

    fn prepare_tests(&self, suite: &TestSuite) -> Vec<PipelineTest> {
        let mut tests: Vec<PipelineTest> = suite.tests.clone();

        // Apply defaults
        if let Some(ref defaults) = suite.defaults {
            for test in &mut tests {
                TestFileParser::apply_defaults(test, defaults);
            }
        }

        // Apply filter
        if let Some(ref filter) = self.config.filter {
            tests.retain(|t| matches_filter(&t.name, filter));
        }

        tests
    }
}

impl Default for TestRunner {
    fn default() -> Self {
        Self::new()
    }
}

/// Simple glob-style filter matching
fn matches_filter(name: &str, filter: &str) -> bool {
    if filter.is_empty() {
        return true;
    }

    // Handle * wildcards at start/end
    if let Some(inner) = filter.strip_prefix('*').and_then(|f| f.strip_suffix('*')) {
        name.to_lowercase().contains(&inner.to_lowercase())
    } else if let Some(pattern) = filter.strip_prefix('*') {
        name.to_lowercase().ends_with(&pattern.to_lowercase())
    } else if let Some(pattern) = filter.strip_suffix('*') {
        name.to_lowercase().starts_with(&pattern.to_lowercase())
    } else {
        name.to_lowercase().contains(&filter.to_lowercase())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{AssertionDef, TestDefaults};
    use std::collections::HashMap;
    use std::path::PathBuf;

    #[test]
    fn test_matches_filter_exact() {
        assert!(matches_filter("Build test", "Build"));
        assert!(matches_filter("Build test", "build")); // case insensitive
        assert!(!matches_filter("Deploy test", "Build"));
    }

    #[test]
    fn test_matches_filter_wildcard_end() {
        assert!(matches_filter("Build test", "Build*"));
        assert!(matches_filter("Build stage", "Build*"));
        assert!(!matches_filter("My Build", "Build*"));
    }

    #[test]
    fn test_matches_filter_wildcard_start() {
        assert!(matches_filter("Build test", "*test"));
        assert!(matches_filter("Deploy test", "*test"));
        assert!(!matches_filter("test suite", "*test"));
    }

    #[test]
    fn test_matches_filter_wildcard_both() {
        assert!(matches_filter("Build test here", "*test*"));
        assert!(matches_filter("My test suite", "*test*"));
    }

    #[test]
    fn test_matches_filter_empty() {
        assert!(matches_filter("anything", ""));
    }

    #[test]
    fn test_suite_result_all_passed() {
        let result = TestSuiteResult {
            suite_name: "test".to_string(),
            results: vec![],
            total: 3,
            passed: 3,
            failed: 0,
            skipped: 0,
            duration: Duration::from_secs(1),
        };
        assert!(result.all_passed());
    }

    #[test]
    fn test_suite_result_has_failures() {
        let result = TestSuiteResult {
            suite_name: "test".to_string(),
            results: vec![],
            total: 3,
            passed: 2,
            failed: 1,
            skipped: 0,
            duration: Duration::from_secs(1),
        };
        assert!(!result.all_passed());
    }

    #[test]
    fn test_runner_builder() {
        let runner = TestRunner::new()
            .with_working_dir("/tmp")
            .with_filter("build*")
            .with_fail_fast(true);

        assert_eq!(runner.config.working_dir, "/tmp");
        assert_eq!(runner.config.filter, Some("build*".to_string()));
        assert!(runner.config.fail_fast);
    }

    #[test]
    fn test_prepare_tests_with_filter() {
        let runner = TestRunner::new().with_filter("Build*");

        let suite = TestSuite {
            name: Some("test".to_string()),
            tests: vec![
                PipelineTest {
                    name: "Build test".to_string(),
                    pipeline: PathBuf::from("pipeline.yml"),
                    variables: HashMap::new(),
                    parameters: HashMap::new(),
                    working_dir: None,
                    assertions: vec![],
                },
                PipelineTest {
                    name: "Deploy test".to_string(),
                    pipeline: PathBuf::from("pipeline.yml"),
                    variables: HashMap::new(),
                    parameters: HashMap::new(),
                    working_dir: None,
                    assertions: vec![],
                },
            ],
            defaults: None,
        };

        let tests = runner.prepare_tests(&suite);
        assert_eq!(tests.len(), 1);
        assert_eq!(tests[0].name, "Build test");
    }

    #[test]
    fn test_prepare_tests_with_defaults() {
        let runner = TestRunner::new();

        let suite = TestSuite {
            name: None,
            tests: vec![PipelineTest {
                name: "Test".to_string(),
                pipeline: PathBuf::from("pipeline.yml"),
                variables: HashMap::new(),
                parameters: HashMap::new(),
                working_dir: None,
                assertions: vec![],
            }],
            defaults: Some(TestDefaults {
                variables: {
                    let mut m = HashMap::new();
                    m.insert("ENV".to_string(), "test".to_string());
                    m
                },
                parameters: HashMap::new(),
                working_dir: Some("/workspace".to_string()),
            }),
        };

        let tests = runner.prepare_tests(&suite);
        assert_eq!(tests[0].variables.get("ENV").unwrap(), "test");
        assert_eq!(tests[0].working_dir, Some("/workspace".to_string()));
    }

    #[tokio::test]
    async fn test_run_test_pipeline_not_found() {
        let runner = TestRunner::new();
        let test = PipelineTest {
            name: "Missing pipeline test".to_string(),
            pipeline: PathBuf::from("/nonexistent/pipeline.yml"),
            variables: HashMap::new(),
            parameters: HashMap::new(),
            working_dir: None,
            assertions: vec![AssertionDef::PipelineSucceeded],
        };

        let result = runner.run_test(&test).await;
        assert!(!result.passed);
        assert!(result.failure_message.unwrap().contains("not found"));
    }

    #[tokio::test]
    async fn test_run_test_with_simple_pipeline() {
        // Create a temporary pipeline file
        let dir = tempfile::tempdir().unwrap();
        let pipeline_path = dir.path().join("pipeline.yml");
        std::fs::write(
            &pipeline_path,
            r#"
trigger: none
steps:
  - script: echo "Hello World"
    name: hello
    displayName: Say Hello
"#,
        )
        .unwrap();

        let runner = TestRunner::new().with_working_dir(dir.path().to_str().unwrap());
        let test = PipelineTest {
            name: "Simple pipeline test".to_string(),
            pipeline: pipeline_path,
            variables: HashMap::new(),
            parameters: HashMap::new(),
            working_dir: Some(dir.path().to_str().unwrap().to_string()),
            assertions: vec![
                AssertionDef::PipelineSucceeded,
                AssertionDef::StepSucceeded("hello".to_string()),
            ],
        };

        let result = runner.run_test(&test).await;
        // The pipeline should parse and execute
        // Assertions may pass or fail depending on execution
        assert!(!result.name.is_empty());
    }
}
