// Matrix Strategy Expansion
// Expands matrix strategies into concrete job instances

use crate::parser::models::{MatrixStrategy, Strategy, Value};

use std::collections::HashMap;

/// A single matrix instance (one combination of matrix values)
#[derive(Debug, Clone)]
pub struct MatrixInstance {
    /// Name of this instance (combination of values)
    pub name: String,
    /// Variable values for this instance
    pub variables: HashMap<String, Value>,
}

/// Matrix expander for job strategies
pub struct MatrixExpander;

impl MatrixExpander {
    /// Expand a strategy into matrix instances
    pub fn expand(strategy: &Strategy) -> Vec<MatrixInstance> {
        let mut instances = Vec::new();

        // Handle matrix strategy
        if let Some(matrix) = &strategy.matrix {
            instances = Self::expand_matrix(matrix);
        }

        // Handle parallel strategy (creates N identical instances)
        if let Some(parallel) = strategy.parallel {
            if instances.is_empty() {
                // No matrix, just parallel copies
                instances = Self::expand_parallel(parallel);
            }
            // If we have both matrix and parallel, the matrix takes precedence
            // and parallel just limits concurrency (handled by executor)
        }

        // Apply maxParallel limit info (stored for executor to use)
        // The actual limiting happens during execution

        instances
    }

    /// Expand an inline matrix into instances
    fn expand_matrix(matrix: &MatrixStrategy) -> Vec<MatrixInstance> {
        match matrix {
            MatrixStrategy::Inline(config) => {
                // Each top-level key is an instance name, values are the variables
                config
                    .iter()
                    .map(|(name, vars)| MatrixInstance {
                        name: name.clone(),
                        variables: vars
                            .iter()
                            .map(|(k, v)| (k.clone(), Self::yaml_to_value(v)))
                            .collect(),
                    })
                    .collect()
            }
            MatrixStrategy::Expression(_expr) => {
                // Expression-based matrices need to be evaluated at runtime
                // For now, return empty and let the executor handle it
                Vec::new()
            }
        }
    }

    /// Expand a parallel count into instances
    fn expand_parallel(count: u32) -> Vec<MatrixInstance> {
        (0..count)
            .map(|i| {
                let mut variables = HashMap::new();
                variables.insert(
                    "System.JobPositionInPhase".to_string(),
                    Value::Number((i + 1) as f64),
                );
                variables.insert(
                    "System.TotalJobsInPhase".to_string(),
                    Value::Number(count as f64),
                );
                MatrixInstance {
                    name: format!("Job {}", i + 1),
                    variables,
                }
            })
            .collect()
    }

    /// Convert serde_yaml::Value to our Value type
    fn yaml_to_value(yaml: &serde_yaml::Value) -> Value {
        match yaml {
            serde_yaml::Value::Null => Value::Null,
            serde_yaml::Value::Bool(b) => Value::Bool(*b),
            serde_yaml::Value::Number(n) => {
                Value::Number(n.as_f64().unwrap_or(n.as_i64().unwrap_or(0) as f64))
            }
            serde_yaml::Value::String(s) => Value::String(s.clone()),
            serde_yaml::Value::Sequence(seq) => {
                Value::Array(seq.iter().map(Self::yaml_to_value).collect())
            }
            serde_yaml::Value::Mapping(map) => Value::Object(
                map.iter()
                    .filter_map(|(k, v)| {
                        k.as_str()
                            .map(|key| (key.to_string(), Self::yaml_to_value(v)))
                    })
                    .collect(),
            ),
            serde_yaml::Value::Tagged(_) => Value::Null, // Not supported
        }
    }

    /// Get the maximum parallel limit from a strategy
    pub fn max_parallel(strategy: &Strategy) -> Option<u32> {
        strategy.max_parallel
    }

    /// Check if a strategy has matrix expansion
    pub fn has_matrix(strategy: &Strategy) -> bool {
        strategy.matrix.is_some()
    }

    /// Check if a strategy has parallel expansion
    pub fn has_parallel(strategy: &Strategy) -> bool {
        strategy.parallel.is_some()
    }
}

/// Builder for creating matrix configurations programmatically
pub struct MatrixBuilder {
    instances: HashMap<String, HashMap<String, Value>>,
}

impl MatrixBuilder {
    pub fn new() -> Self {
        Self {
            instances: HashMap::new(),
        }
    }

    /// Add an instance with given name and variables
    pub fn add_instance(
        mut self,
        name: impl Into<String>,
        variables: HashMap<String, Value>,
    ) -> Self {
        self.instances.insert(name.into(), variables);
        self
    }

    /// Add an instance with a single variable
    pub fn add_simple(
        mut self,
        instance_name: impl Into<String>,
        var_name: impl Into<String>,
        var_value: impl Into<Value>,
    ) -> Self {
        let mut vars = HashMap::new();
        vars.insert(var_name.into(), var_value.into());
        self.instances.insert(instance_name.into(), vars);
        self
    }

    /// Build into MatrixInstance list
    pub fn build(self) -> Vec<MatrixInstance> {
        self.instances
            .into_iter()
            .map(|(name, variables)| MatrixInstance { name, variables })
            .collect()
    }
}

impl Default for MatrixBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_inline_matrix() {
        let mut config = HashMap::new();

        let mut linux_vars = HashMap::new();
        linux_vars.insert(
            "vmImage".to_string(),
            serde_yaml::Value::String("ubuntu-latest".to_string()),
        );
        linux_vars.insert(
            "platform".to_string(),
            serde_yaml::Value::String("linux".to_string()),
        );
        config.insert("linux".to_string(), linux_vars);

        let mut windows_vars = HashMap::new();
        windows_vars.insert(
            "vmImage".to_string(),
            serde_yaml::Value::String("windows-latest".to_string()),
        );
        windows_vars.insert(
            "platform".to_string(),
            serde_yaml::Value::String("windows".to_string()),
        );
        config.insert("windows".to_string(), windows_vars);

        let strategy = Strategy {
            matrix: Some(MatrixStrategy::Inline(config)),
            parallel: None,
            max_parallel: Some(2),
            run_once: None,
            rolling: None,
            canary: None,
        };

        let instances = MatrixExpander::expand(&strategy);

        assert_eq!(instances.len(), 2);

        // Check that we have both instances (order may vary)
        let names: Vec<_> = instances.iter().map(|i| i.name.as_str()).collect();
        assert!(names.contains(&"linux"));
        assert!(names.contains(&"windows"));

        // Check variables
        let linux = instances.iter().find(|i| i.name == "linux").unwrap();
        assert_eq!(
            linux.variables.get("vmImage"),
            Some(&Value::String("ubuntu-latest".to_string()))
        );
        assert_eq!(
            linux.variables.get("platform"),
            Some(&Value::String("linux".to_string()))
        );
    }

    #[test]
    fn test_expand_parallel() {
        let strategy = Strategy {
            matrix: None,
            parallel: Some(4),
            max_parallel: None,
            run_once: None,
            rolling: None,
            canary: None,
        };

        let instances = MatrixExpander::expand(&strategy);

        assert_eq!(instances.len(), 4);

        for (i, instance) in instances.iter().enumerate() {
            assert_eq!(instance.name, format!("Job {}", i + 1));
            assert_eq!(
                instance.variables.get("System.JobPositionInPhase"),
                Some(&Value::Number((i + 1) as f64))
            );
            assert_eq!(
                instance.variables.get("System.TotalJobsInPhase"),
                Some(&Value::Number(4.0))
            );
        }
    }

    #[test]
    fn test_matrix_builder() {
        let instances = MatrixBuilder::new()
            .add_simple("debug", "configuration", "Debug")
            .add_simple("release", "configuration", "Release")
            .build();

        assert_eq!(instances.len(), 2);

        let names: Vec<_> = instances.iter().map(|i| i.name.as_str()).collect();
        assert!(names.contains(&"debug"));
        assert!(names.contains(&"release"));
    }

    #[test]
    fn test_max_parallel() {
        let strategy = Strategy {
            matrix: None,
            parallel: Some(10),
            max_parallel: Some(3),
            run_once: None,
            rolling: None,
            canary: None,
        };

        assert_eq!(MatrixExpander::max_parallel(&strategy), Some(3));
    }

    #[test]
    fn test_no_matrix_strategy() {
        let strategy = Strategy {
            matrix: None,
            parallel: None,
            max_parallel: None,
            run_once: None,
            rolling: None,
            canary: None,
        };

        let instances = MatrixExpander::expand(&strategy);
        assert!(instances.is_empty());
    }

    #[test]
    fn test_yaml_value_conversion() {
        let yaml_string = serde_yaml::Value::String("test".to_string());
        assert_eq!(
            MatrixExpander::yaml_to_value(&yaml_string),
            Value::String("test".to_string())
        );

        let yaml_number = serde_yaml::Value::Number(serde_yaml::Number::from(42));
        assert_eq!(
            MatrixExpander::yaml_to_value(&yaml_number),
            Value::Number(42.0)
        );

        let yaml_bool = serde_yaml::Value::Bool(true);
        assert_eq!(MatrixExpander::yaml_to_value(&yaml_bool), Value::Bool(true));

        let yaml_null = serde_yaml::Value::Null;
        assert_eq!(MatrixExpander::yaml_to_value(&yaml_null), Value::Null);
    }
}
