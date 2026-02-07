// Testing Framework Module
// Provides pipeline test definitions, execution, assertions, and reporting

pub mod assertions;
pub mod parser;
pub mod reporter;
pub mod runner;

// Re-export key types
pub use assertions::{Assertion, AssertionResult};
pub use parser::TestFileParser;
pub use reporter::{ReportFormat, TestReporter};
pub use runner::{TestResult, TestRunner, TestSuiteResult};

use crate::parser::models::Value;

use std::collections::HashMap;
use std::path::PathBuf;

use serde::de::{self, MapAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};

// =============================================================================
// Test Definition Models
// =============================================================================

/// A complete test suite loaded from a roxid-test.yml file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestSuite {
    /// Optional suite name
    #[serde(default)]
    pub name: Option<String>,
    /// Test definitions
    pub tests: Vec<PipelineTest>,
    /// Default variables applied to all tests
    #[serde(default)]
    pub defaults: Option<TestDefaults>,
}

/// Default values applied to all tests in a suite
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestDefaults {
    /// Default variables
    #[serde(default)]
    pub variables: HashMap<String, String>,
    /// Default parameters
    #[serde(default)]
    pub parameters: HashMap<String, serde_yaml::Value>,
    /// Default working directory
    #[serde(default)]
    pub working_dir: Option<String>,
}

/// A single pipeline test definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineTest {
    /// Test name (used in reporting)
    pub name: String,
    /// Path to the pipeline YAML file (relative to test file)
    pub pipeline: PathBuf,
    /// Variables to set for this test run
    #[serde(default)]
    pub variables: HashMap<String, String>,
    /// Parameters to pass for this test run
    #[serde(default)]
    pub parameters: HashMap<String, serde_yaml::Value>,
    /// Working directory for execution
    #[serde(default)]
    pub working_dir: Option<String>,
    /// Assertions to evaluate after execution
    #[serde(default)]
    pub assertions: Vec<AssertionDef>,
}

/// An assertion definition as parsed from YAML
///
/// Each variant maps to a YAML key in the assertions list.
/// This is the serializable form; it gets converted to `Assertion`
/// for evaluation.
///
/// Supports YAML formats:
/// - Bare string: `pipeline_succeeded`
/// - Key-value: `step_succeeded: Build`
/// - Key-struct: `step_output_contains: { step: Build, pattern: "..." }`
#[derive(Debug, Clone, Serialize)]
pub enum AssertionDef {
    /// Assert a step succeeded
    StepSucceeded(String),

    /// Assert a step failed
    StepFailed(String),

    /// Assert a step was skipped
    StepSkipped(String),

    /// Assert a job succeeded
    JobSucceeded(String),

    /// Assert a job failed
    JobFailed(String),

    /// Assert a job was skipped
    JobSkipped(String),

    /// Assert a stage succeeded
    StageSucceeded(String),

    /// Assert a stage failed
    StageFailed(String),

    /// Assert a stage was skipped
    StageSkipped(String),

    /// Assert step output equals a value
    StepOutputEquals(StepOutputAssertion),

    /// Assert step output contains a pattern
    StepOutputContains(StepOutputPatternAssertion),

    /// Assert a step ran before another step
    StepRanBefore(OrderAssertion),

    /// Assert steps ran in parallel (within the same stage/job level)
    StepsRanInParallel(ParallelAssertion),

    /// Assert a variable has a specific value after execution
    VariableEquals(VariableAssertion),

    /// Assert a variable contains a pattern
    VariableContains(VariablePatternAssertion),

    /// Assert the pipeline succeeded overall
    PipelineSucceeded,

    /// Assert the pipeline failed overall
    PipelineFailed,
}

impl<'de> Deserialize<'de> for AssertionDef {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct AssertionDefVisitor;

        impl<'de> Visitor<'de> for AssertionDefVisitor {
            type Value = AssertionDef;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str(
                    "a string like 'pipeline_succeeded' or a mapping like 'step_succeeded: Build'",
                )
            }

            // Handle bare strings: `- pipeline_succeeded`
            fn visit_str<E>(self, value: &str) -> Result<AssertionDef, E>
            where
                E: de::Error,
            {
                match value {
                    "pipeline_succeeded" => Ok(AssertionDef::PipelineSucceeded),
                    "pipeline_failed" => Ok(AssertionDef::PipelineFailed),
                    _ => Err(de::Error::unknown_variant(
                        value,
                        &["pipeline_succeeded", "pipeline_failed"],
                    )),
                }
            }

            // Handle mappings: `- step_succeeded: Build` or `- step_output_contains: { ... }`
            fn visit_map<M>(self, mut map: M) -> Result<AssertionDef, M::Error>
            where
                M: MapAccess<'de>,
            {
                let key: String = map
                    .next_key()?
                    .ok_or_else(|| de::Error::custom("expected assertion key"))?;

                let result = match key.as_str() {
                    "step_succeeded" => {
                        let val: String = map.next_value()?;
                        Ok(AssertionDef::StepSucceeded(val))
                    }
                    "step_failed" => {
                        let val: String = map.next_value()?;
                        Ok(AssertionDef::StepFailed(val))
                    }
                    "step_skipped" => {
                        let val: String = map.next_value()?;
                        Ok(AssertionDef::StepSkipped(val))
                    }
                    "job_succeeded" => {
                        let val: String = map.next_value()?;
                        Ok(AssertionDef::JobSucceeded(val))
                    }
                    "job_failed" => {
                        let val: String = map.next_value()?;
                        Ok(AssertionDef::JobFailed(val))
                    }
                    "job_skipped" => {
                        let val: String = map.next_value()?;
                        Ok(AssertionDef::JobSkipped(val))
                    }
                    "stage_succeeded" => {
                        let val: String = map.next_value()?;
                        Ok(AssertionDef::StageSucceeded(val))
                    }
                    "stage_failed" => {
                        let val: String = map.next_value()?;
                        Ok(AssertionDef::StageFailed(val))
                    }
                    "stage_skipped" => {
                        let val: String = map.next_value()?;
                        Ok(AssertionDef::StageSkipped(val))
                    }
                    "step_output_equals" => {
                        let val: StepOutputAssertion = map.next_value()?;
                        Ok(AssertionDef::StepOutputEquals(val))
                    }
                    "step_output_contains" => {
                        let val: StepOutputPatternAssertion = map.next_value()?;
                        Ok(AssertionDef::StepOutputContains(val))
                    }
                    "step_ran_before" => {
                        let val: OrderAssertion = map.next_value()?;
                        Ok(AssertionDef::StepRanBefore(val))
                    }
                    "steps_ran_in_parallel" => {
                        let val: ParallelAssertion = map.next_value()?;
                        Ok(AssertionDef::StepsRanInParallel(val))
                    }
                    "variable_equals" => {
                        let val: VariableAssertion = map.next_value()?;
                        Ok(AssertionDef::VariableEquals(val))
                    }
                    "variable_contains" => {
                        let val: VariablePatternAssertion = map.next_value()?;
                        Ok(AssertionDef::VariableContains(val))
                    }
                    "pipeline_succeeded" => {
                        // Allow `pipeline_succeeded:` with null/empty value in mapping form
                        let _: serde_yaml::Value = map.next_value()?;
                        Ok(AssertionDef::PipelineSucceeded)
                    }
                    "pipeline_failed" => {
                        let _: serde_yaml::Value = map.next_value()?;
                        Ok(AssertionDef::PipelineFailed)
                    }
                    _ => Err(de::Error::unknown_field(
                        &key,
                        &[
                            "step_succeeded",
                            "step_failed",
                            "step_skipped",
                            "job_succeeded",
                            "job_failed",
                            "job_skipped",
                            "stage_succeeded",
                            "stage_failed",
                            "stage_skipped",
                            "step_output_equals",
                            "step_output_contains",
                            "step_ran_before",
                            "steps_ran_in_parallel",
                            "variable_equals",
                            "variable_contains",
                            "pipeline_succeeded",
                            "pipeline_failed",
                        ],
                    )),
                };

                result
            }
        }

        deserializer.deserialize_any(AssertionDefVisitor)
    }
}

/// Assertion for step output equality
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepOutputAssertion {
    /// Step name (the `name:` field of the step)
    pub step: String,
    /// Output variable name
    pub output: String,
    /// Expected value
    pub expected: serde_yaml::Value,
}

/// Assertion for step output pattern matching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepOutputPatternAssertion {
    /// Step name
    pub step: String,
    /// Substring or pattern to search for in stdout
    pub pattern: String,
    /// Optional: which output to check ("stdout", "stderr", or specific output variable)
    #[serde(default)]
    pub output: Option<String>,
}

/// Assertion for execution ordering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderAssertion {
    /// The step that should run first
    pub step: String,
    /// The step that should run after
    pub before: String,
}

/// Assertion for parallel execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelAssertion {
    /// Steps that should have run in parallel
    pub steps: Vec<String>,
}

/// Assertion for variable values
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariableAssertion {
    /// Variable name
    pub name: String,
    /// Expected value
    pub expected: serde_yaml::Value,
}

/// Assertion for variable pattern matching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariablePatternAssertion {
    /// Variable name
    pub name: String,
    /// Pattern to match
    pub pattern: String,
}

// =============================================================================
// Conversion helpers
// =============================================================================

impl AssertionDef {
    /// Convert this YAML assertion definition into an evaluable `Assertion`
    pub fn to_assertion(&self) -> Assertion {
        match self {
            AssertionDef::StepSucceeded(name) => Assertion::StepSucceeded { step: name.clone() },
            AssertionDef::StepFailed(name) => Assertion::StepFailed { step: name.clone() },
            AssertionDef::StepSkipped(name) => Assertion::StepSkipped { step: name.clone() },
            AssertionDef::JobSucceeded(name) => Assertion::JobSucceeded { job: name.clone() },
            AssertionDef::JobFailed(name) => Assertion::JobFailed { job: name.clone() },
            AssertionDef::JobSkipped(name) => Assertion::JobSkipped { job: name.clone() },
            AssertionDef::StageSucceeded(name) => Assertion::StageSucceeded {
                stage: name.clone(),
            },
            AssertionDef::StageFailed(name) => Assertion::StageFailed {
                stage: name.clone(),
            },
            AssertionDef::StageSkipped(name) => Assertion::StageSkipped {
                stage: name.clone(),
            },
            AssertionDef::StepOutputEquals(a) => Assertion::StepOutputEquals {
                step: a.step.clone(),
                output: a.output.clone(),
                expected: yaml_to_value(&a.expected),
            },
            AssertionDef::StepOutputContains(a) => Assertion::StepOutputContains {
                step: a.step.clone(),
                pattern: a.pattern.clone(),
                output: a.output.clone(),
            },
            AssertionDef::StepRanBefore(a) => Assertion::StepRanBefore {
                step: a.step.clone(),
                before: a.before.clone(),
            },
            AssertionDef::StepsRanInParallel(a) => Assertion::StepsRanInParallel {
                steps: a.steps.clone(),
            },
            AssertionDef::VariableEquals(a) => Assertion::VariableEquals {
                name: a.name.clone(),
                expected: yaml_to_value(&a.expected),
            },
            AssertionDef::VariableContains(a) => Assertion::VariableContains {
                name: a.name.clone(),
                pattern: a.pattern.clone(),
            },
            AssertionDef::PipelineSucceeded => Assertion::PipelineSucceeded,
            AssertionDef::PipelineFailed => Assertion::PipelineFailed,
        }
    }
}

/// Convert a serde_yaml::Value to our internal Value type
fn yaml_to_value(v: &serde_yaml::Value) -> Value {
    match v {
        serde_yaml::Value::Null => Value::Null,
        serde_yaml::Value::Bool(b) => Value::Bool(*b),
        serde_yaml::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::Number(i as f64)
            } else if let Some(f) = n.as_f64() {
                Value::Number(f)
            } else {
                Value::Null
            }
        }
        serde_yaml::Value::String(s) => Value::String(s.clone()),
        serde_yaml::Value::Sequence(seq) => Value::Array(seq.iter().map(yaml_to_value).collect()),
        serde_yaml::Value::Mapping(map) => {
            let mut obj = HashMap::new();
            for (k, v) in map {
                if let serde_yaml::Value::String(key) = k {
                    obj.insert(key.clone(), yaml_to_value(v));
                }
            }
            Value::Object(obj)
        }
        serde_yaml::Value::Tagged(tagged) => yaml_to_value(&tagged.value),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_assertion_def_to_assertion_step_succeeded() {
        let def = AssertionDef::StepSucceeded("Build".to_string());
        let assertion = def.to_assertion();
        assert!(matches!(
            assertion,
            Assertion::StepSucceeded { step } if step == "Build"
        ));
    }

    #[test]
    fn test_assertion_def_to_assertion_variable_equals() {
        let def = AssertionDef::VariableEquals(VariableAssertion {
            name: "BUILD_CONFIG".to_string(),
            expected: serde_yaml::Value::String("Release".to_string()),
        });
        let assertion = def.to_assertion();
        assert!(matches!(
            assertion,
            Assertion::VariableEquals { name, expected }
                if name == "BUILD_CONFIG" && expected == Value::String("Release".to_string())
        ));
    }

    #[test]
    fn test_yaml_to_value_primitives() {
        assert_eq!(yaml_to_value(&serde_yaml::Value::Null), Value::Null);
        assert_eq!(
            yaml_to_value(&serde_yaml::Value::Bool(true)),
            Value::Bool(true)
        );
        assert_eq!(
            yaml_to_value(&serde_yaml::Value::String("hello".to_string())),
            Value::String("hello".to_string())
        );
    }

    #[test]
    fn test_pipeline_test_deserialize() {
        let yaml = r#"
name: "Build test"
pipeline: azure-pipelines.yml
variables:
  BUILD_CONFIG: Release
assertions:
  - step_succeeded: Build
  - pipeline_succeeded
"#;
        let test: PipelineTest = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(test.name, "Build test");
        assert_eq!(test.pipeline, PathBuf::from("azure-pipelines.yml"));
        assert_eq!(test.variables.get("BUILD_CONFIG").unwrap(), "Release");
        assert_eq!(test.assertions.len(), 2);
    }

    #[test]
    fn test_test_suite_deserialize() {
        let yaml = r#"
tests:
  - name: "Build stage runs correctly"
    pipeline: azure-pipelines.yml
    variables:
      BUILD_CONFIG: Release
    assertions:
      - step_succeeded: Build
      - step_output_contains:
          step: Build
          pattern: "Build succeeded"
      - step_ran_before:
          step: Test
          before: Deploy

  - name: "Deploy is skipped on PR"
    pipeline: azure-pipelines.yml
    variables:
      BUILD_REASON: PullRequest
    assertions:
      - step_skipped: Deploy
"#;
        let suite: TestSuite = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(suite.tests.len(), 2);
        assert_eq!(suite.tests[0].name, "Build stage runs correctly");
        assert_eq!(suite.tests[0].assertions.len(), 3);
        assert_eq!(suite.tests[1].name, "Deploy is skipped on PR");
        assert_eq!(suite.tests[1].assertions.len(), 1);
    }
}
