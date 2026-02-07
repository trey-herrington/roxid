// Test File Parser
// Loads and validates roxid-test.yml test suite files

use crate::testing::{PipelineTest, TestDefaults, TestSuite};

use std::fs;
use std::path::{Path, PathBuf};

/// Parser for roxid-test.yml test suite files
pub struct TestFileParser;

/// Errors that can occur during test file parsing
#[derive(Debug)]
pub enum TestParseError {
    /// File not found
    NotFound(PathBuf),
    /// IO error reading file
    IoError(std::io::Error),
    /// YAML parsing error
    YamlError(serde_yaml::Error),
    /// Validation error
    ValidationError(String),
}

impl std::fmt::Display for TestParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TestParseError::NotFound(path) => {
                write!(f, "Test file not found: {}", path.display())
            }
            TestParseError::IoError(e) => write!(f, "IO error reading test file: {}", e),
            TestParseError::YamlError(e) => write!(f, "YAML parse error in test file: {}", e),
            TestParseError::ValidationError(msg) => {
                write!(f, "Test file validation error: {}", msg)
            }
        }
    }
}

impl std::error::Error for TestParseError {}

impl From<std::io::Error> for TestParseError {
    fn from(err: std::io::Error) -> Self {
        TestParseError::IoError(err)
    }
}

impl From<serde_yaml::Error> for TestParseError {
    fn from(err: serde_yaml::Error) -> Self {
        TestParseError::YamlError(err)
    }
}

impl TestFileParser {
    /// Parse a test suite from a YAML string
    pub fn parse(content: &str) -> Result<TestSuite, TestParseError> {
        let suite: TestSuite = serde_yaml::from_str(content)?;
        Self::validate(&suite)?;
        Ok(suite)
    }

    /// Parse a test suite from a file path
    pub fn parse_file(path: &Path) -> Result<TestSuite, TestParseError> {
        if !path.exists() {
            return Err(TestParseError::NotFound(path.to_path_buf()));
        }

        let content = fs::read_to_string(path)?;
        let mut suite = Self::parse(&content)?;

        // Resolve pipeline paths relative to the test file's directory
        let base_dir = path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf();

        for test in &mut suite.tests {
            if test.pipeline.is_relative() {
                test.pipeline = base_dir.join(&test.pipeline);
            }
        }

        Ok(suite)
    }

    /// Discover test files in a directory
    ///
    /// Looks for files matching: `roxid-test.yml`, `roxid-test.yaml`,
    /// `*.roxid-test.yml`, `*.roxid-test.yaml`, or files in a `tests/` directory.
    pub fn discover(dir: &Path) -> Vec<PathBuf> {
        let mut test_files = Vec::new();

        // Check for standard names
        for name in &[
            "roxid-test.yml",
            "roxid-test.yaml",
            ".roxid-test.yml",
            ".roxid-test.yaml",
        ] {
            let path = dir.join(name);
            if path.exists() {
                test_files.push(path);
            }
        }

        // Check tests/ directory
        let tests_dir = dir.join("tests");
        if tests_dir.is_dir() {
            if let Ok(entries) = fs::read_dir(&tests_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        if (name.ends_with(".roxid-test.yml")
                            || name.ends_with(".roxid-test.yaml")
                            || name == "roxid-test.yml"
                            || name == "roxid-test.yaml")
                            && path.is_file()
                        {
                            test_files.push(path);
                        }
                    }
                }
            }
        }

        test_files.sort();
        test_files
    }

    /// Apply suite defaults to a test, merging variables and parameters
    pub fn apply_defaults(test: &mut PipelineTest, defaults: &TestDefaults) {
        // Merge variables (test-specific overrides defaults)
        for (key, value) in &defaults.variables {
            test.variables.entry(key.clone()).or_insert(value.clone());
        }

        // Merge parameters (test-specific overrides defaults)
        for (key, value) in &defaults.parameters {
            test.parameters.entry(key.clone()).or_insert(value.clone());
        }

        // Apply working directory if not set
        if test.working_dir.is_none() {
            test.working_dir.clone_from(&defaults.working_dir);
        }
    }

    /// Validate a test suite
    fn validate(suite: &TestSuite) -> Result<(), TestParseError> {
        if suite.tests.is_empty() {
            return Err(TestParseError::ValidationError(
                "Test suite must contain at least one test".to_string(),
            ));
        }

        for (i, test) in suite.tests.iter().enumerate() {
            if test.name.is_empty() {
                return Err(TestParseError::ValidationError(format!(
                    "Test at index {} must have a non-empty name",
                    i
                )));
            }

            if test.pipeline.as_os_str().is_empty() {
                return Err(TestParseError::ValidationError(format!(
                    "Test '{}' must specify a pipeline file",
                    test.name
                )));
            }
        }

        // Check for duplicate test names
        let mut names = std::collections::HashSet::new();
        for test in &suite.tests {
            if !names.insert(&test.name) {
                return Err(TestParseError::ValidationError(format!(
                    "Duplicate test name: '{}'",
                    test.name
                )));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic_suite() {
        let yaml = r#"
tests:
  - name: "Basic test"
    pipeline: pipeline.yml
    assertions:
      - pipeline_succeeded
"#;
        let suite = TestFileParser::parse(yaml).unwrap();
        assert_eq!(suite.tests.len(), 1);
        assert_eq!(suite.tests[0].name, "Basic test");
    }

    #[test]
    fn test_parse_full_suite() {
        let yaml = r#"
name: "My test suite"
defaults:
  variables:
    ENV: test
  working_dir: /tmp
tests:
  - name: "Build test"
    pipeline: azure-pipelines.yml
    variables:
      BUILD_CONFIG: Release
    assertions:
      - step_succeeded: Build
      - step_output_contains:
          step: Build
          pattern: "Build succeeded"

  - name: "Deploy skipped on PR"
    pipeline: azure-pipelines.yml
    variables:
      BUILD_REASON: PullRequest
    assertions:
      - step_skipped: Deploy
      - pipeline_succeeded
"#;
        let suite = TestFileParser::parse(yaml).unwrap();
        assert_eq!(suite.name, Some("My test suite".to_string()));
        assert!(suite.defaults.is_some());
        assert_eq!(suite.tests.len(), 2);
    }

    #[test]
    fn test_parse_empty_tests_fails() {
        let yaml = r#"
tests: []
"#;
        let result = TestFileParser::parse(yaml);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            TestParseError::ValidationError(_)
        ));
    }

    #[test]
    fn test_parse_duplicate_names_fails() {
        let yaml = r#"
tests:
  - name: "Test A"
    pipeline: pipeline.yml
    assertions: []
  - name: "Test A"
    pipeline: pipeline.yml
    assertions: []
"#;
        let result = TestFileParser::parse(yaml);
        assert!(result.is_err());
    }

    #[test]
    fn test_apply_defaults() {
        let defaults = TestDefaults {
            variables: {
                let mut m = std::collections::HashMap::new();
                m.insert("ENV".to_string(), "test".to_string());
                m.insert("DEBUG".to_string(), "false".to_string());
                m
            },
            parameters: std::collections::HashMap::new(),
            working_dir: Some("/tmp".to_string()),
        };

        let mut test = PipelineTest {
            name: "test".to_string(),
            pipeline: PathBuf::from("pipeline.yml"),
            variables: {
                let mut m = std::collections::HashMap::new();
                m.insert("ENV".to_string(), "prod".to_string()); // Should NOT be overridden
                m
            },
            parameters: std::collections::HashMap::new(),
            working_dir: None,
            assertions: vec![],
        };

        TestFileParser::apply_defaults(&mut test, &defaults);
        assert_eq!(test.variables.get("ENV").unwrap(), "prod"); // Test value wins
        assert_eq!(test.variables.get("DEBUG").unwrap(), "false"); // Default applied
        assert_eq!(test.working_dir, Some("/tmp".to_string())); // Default applied
    }

    #[test]
    fn test_parse_file_not_found() {
        let result = TestFileParser::parse_file(Path::new("/nonexistent/roxid-test.yml"));
        assert!(matches!(result.unwrap_err(), TestParseError::NotFound(_)));
    }

    #[test]
    fn test_discover_no_test_files() {
        let dir = tempfile::tempdir().unwrap();
        let files = TestFileParser::discover(dir.path());
        assert!(files.is_empty());
    }

    #[test]
    fn test_discover_standard_names() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("roxid-test.yml"), "tests: []").unwrap();
        let files = TestFileParser::discover(dir.path());
        assert_eq!(files.len(), 1);
    }
}
