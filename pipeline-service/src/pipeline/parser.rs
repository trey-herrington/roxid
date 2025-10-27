use crate::pipeline::models::Pipeline;
use crate::ServiceResult;
use std::fs;
use std::path::Path;

pub struct PipelineParser;

impl PipelineParser {
    pub fn from_file<P: AsRef<Path>>(path: P) -> ServiceResult<Pipeline> {
        let content = fs::read_to_string(path)?;
        Self::from_str(&content)
    }

    pub fn from_str(content: &str) -> ServiceResult<Pipeline> {
        let pipeline: Pipeline = serde_yaml::from_str(content)?;
        Ok(pipeline)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_pipeline() {
        let yaml = r#"
name: test-pipeline
steps:
  - name: Build
    command: cargo build
  - name: Test
    command: cargo test
"#;
        let pipeline = PipelineParser::from_str(yaml).unwrap();
        assert_eq!(pipeline.name, "test-pipeline");
        assert_eq!(pipeline.steps.len(), 2);
    }

    #[test]
    fn test_parse_with_env() {
        let yaml = r#"
name: test-pipeline
env:
  RUST_LOG: debug
steps:
  - name: Build
    command: cargo build
    env:
      BUILD_MODE: release
"#;
        let pipeline = PipelineParser::from_str(yaml).unwrap();
        assert_eq!(pipeline.env.get("RUST_LOG"), Some(&"debug".to_string()));
        assert_eq!(pipeline.steps[0].env.get("BUILD_MODE"), Some(&"release".to_string()));
    }

    #[test]
    fn test_parse_shell_script() {
        let yaml = r#"
name: test-pipeline
steps:
  - name: Multi-line script
    shell:
      script: |
        echo "Hello"
        echo "World"
"#;
        let pipeline = PipelineParser::from_str(yaml).unwrap();
        assert_eq!(pipeline.steps.len(), 1);
    }
}
