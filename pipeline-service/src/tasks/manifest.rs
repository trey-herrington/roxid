// Task Manifest Parser
// Parses Azure DevOps task.json files

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::str::FromStr;
use thiserror::Error;

/// Errors that can occur when parsing task manifests
#[derive(Debug, Error)]
pub enum TaskManifestError {
    #[error("Failed to read task manifest: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Failed to parse task manifest: {0}")]
    ParseError(#[from] serde_json::Error),

    #[error("Task manifest not found at: {0}")]
    NotFound(String),

    #[error("Invalid task manifest: {0}")]
    Invalid(String),
}

/// Task manifest (task.json)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskManifest {
    /// Unique task ID
    pub id: String,

    /// Task name
    pub name: String,

    /// Friendly name for display
    pub friendly_name: Option<String>,

    /// Task description
    pub description: Option<String>,

    /// Help URL
    pub help_url: Option<String>,

    /// Help markdown file
    pub help_mark_down: Option<String>,

    /// Task category
    pub category: Option<String>,

    /// Visibility in different contexts
    pub visibility: Option<Vec<String>>,

    /// Whether to run on the agent
    pub runs_on: Option<Vec<String>>,

    /// Task author
    pub author: Option<String>,

    /// Version information
    pub version: TaskVersion,

    /// Minimum agent version required
    pub minimum_agent_version: Option<String>,

    /// Instance name format
    pub instance_name_format: Option<String>,

    /// Variable groups
    pub groups: Option<Vec<TaskGroup>>,

    /// Task inputs
    #[serde(default)]
    pub inputs: Vec<TaskInput>,

    /// Output variables
    #[serde(default)]
    pub output_variables: Option<Vec<TaskOutputVariable>>,

    /// Task execution
    pub execution: Option<TaskExecutionSection>,

    /// Pre-job execution
    pub pre_job_execution: Option<TaskExecutionSection>,

    /// Post-job execution (cleanup)
    pub post_job_execution: Option<TaskExecutionSection>,

    /// Data source bindings
    pub data_source_bindings: Option<Vec<DataSourceBinding>>,

    /// Messages for localization
    pub messages: Option<HashMap<String, String>>,

    /// Restrictions
    pub restrictions: Option<TaskRestrictions>,

    /// Demands (capabilities required on agent)
    pub demands: Option<Vec<String>>,
}

impl FromStr for TaskManifest {
    type Err = TaskManifestError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let manifest: TaskManifest = serde_json::from_str(s)?;
        Ok(manifest)
    }
}

impl TaskManifest {
    /// Parse a task manifest from a file path
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, TaskManifestError> {
        let path = path.as_ref();
        if !path.exists() {
            return Err(TaskManifestError::NotFound(path.display().to_string()));
        }

        let content = fs::read_to_string(path)?;
        Self::parse_str(&content)
    }

    /// Parse a task manifest from JSON string
    pub fn parse_str(content: &str) -> Result<Self, TaskManifestError> {
        content.parse()
    }

    /// Get the full version string (major.minor.patch)
    pub fn version_string(&self) -> String {
        format!(
            "{}.{}.{}",
            self.version.major, self.version.minor, self.version.patch
        )
    }

    /// Get the primary execution method
    pub fn primary_execution(&self) -> Option<&TaskExecution> {
        self.execution.as_ref().and_then(|e| {
            // Prefer Node, then Node10, then Node16, then Node20, then PowerShell3
            e.node
                .as_ref()
                .or(e.node10.as_ref())
                .or(e.node16.as_ref())
                .or(e.node20.as_ref())
                .or(e.powershell3.as_ref())
                .or(e.powershell.as_ref())
        })
    }

    /// Check if this is a Node.js task
    pub fn is_node_task(&self) -> bool {
        self.execution.as_ref().is_some_and(|e| {
            e.node.is_some() || e.node10.is_some() || e.node16.is_some() || e.node20.is_some()
        })
    }

    /// Check if this is a PowerShell task
    pub fn is_powershell_task(&self) -> bool {
        self.execution
            .as_ref()
            .is_some_and(|e| e.powershell.is_some() || e.powershell3.is_some())
    }

    /// Get required input names
    pub fn required_inputs(&self) -> Vec<&str> {
        self.inputs
            .iter()
            .filter(|i| i.required.unwrap_or(false))
            .map(|i| i.name.as_str())
            .collect()
    }

    /// Get default values for inputs
    pub fn default_values(&self) -> HashMap<String, String> {
        self.inputs
            .iter()
            .filter_map(|i| {
                i.default_value
                    .as_ref()
                    .map(|v| (i.name.clone(), v.clone()))
            })
            .collect()
    }
}

/// Task version information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct TaskVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

/// Task input grouping
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskGroup {
    pub name: String,
    pub display_name: Option<String>,
    pub is_expanded: Option<bool>,
}

/// Task input definition
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskInput {
    /// Input name
    pub name: String,

    /// Input type
    #[serde(rename = "type")]
    pub input_type: Option<String>,

    /// Display label
    pub label: Option<String>,

    /// Default value
    pub default_value: Option<String>,

    /// Whether input is required
    pub required: Option<bool>,

    /// Help text
    pub help_mark_down: Option<String>,

    /// Group this input belongs to
    pub group_name: Option<String>,

    /// Visibility rules
    pub visible_rule: Option<String>,

    /// Options for select/picklist inputs
    pub options: Option<HashMap<String, String>>,

    /// Properties
    pub properties: Option<InputProperties>,

    /// Validation
    pub validation: Option<InputValidation>,

    /// Aliases for the input name
    pub aliases: Option<Vec<String>>,
}

/// Input properties
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InputProperties {
    pub editable_options: Option<String>,
    pub multi_select: Option<String>,
    pub multi_select_flatlist: Option<String>,
    pub disable_manage_link: Option<String>,
    pub is_search_required: Option<String>,
}

/// Input validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputValidation {
    pub expression: Option<String>,
    pub message: Option<String>,
}

/// Task output variable
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskOutputVariable {
    pub name: String,
    pub description: Option<String>,
}

/// Task execution section containing all execution handlers
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct TaskExecutionSection {
    /// Node.js handler (legacy)
    pub node: Option<TaskExecution>,

    /// Node 10 handler
    pub node10: Option<TaskExecution>,

    /// Node 16 handler
    pub node16: Option<TaskExecution>,

    /// Node 20 handler
    pub node20: Option<TaskExecution>,

    /// PowerShell handler (legacy)
    pub powershell: Option<TaskExecution>,

    /// PowerShell 3 handler
    #[serde(rename = "PowerShell3")]
    pub powershell3: Option<TaskExecution>,
}

/// Execution handler
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskExecution {
    /// Script target file
    pub target: String,

    /// Working directory
    pub working_directory: Option<String>,

    /// Platforms this handler runs on
    pub platforms: Option<Vec<String>>,

    /// Argument format
    pub argument_format: Option<String>,
}

impl TaskExecution {
    /// Get the execution type as a string
    pub fn execution_type(&self) -> &'static str {
        // This is determined by which field in TaskExecutionSection contains this
        // For now, we'll default to "Node" since that's most common
        "Node"
    }
}

/// Data source binding for service connection
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DataSourceBinding {
    pub target: String,
    pub endpoint_id: Option<String>,
    pub data_source_name: Option<String>,
    pub parameters: Option<HashMap<String, String>>,
    pub result_template: Option<String>,
}

/// Task restrictions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRestrictions {
    pub commands: Option<TaskCommandRestrictions>,
    #[serde(rename = "settableVariables")]
    pub settable_variables: Option<TaskSettableVariables>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskCommandRestrictions {
    pub mode: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskSettableVariables {
    pub allowed: Option<Vec<String>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_TASK_JSON: &str = r#"{
        "id": "6c731c3c-3c68-459a-a5c9-bde6e6595b5b",
        "name": "Bash",
        "friendlyName": "Bash",
        "description": "Run a Bash script on macOS, Linux, or Windows",
        "helpUrl": "https://docs.microsoft.com/azure/devops/pipelines/tasks/utility/bash",
        "category": "Utility",
        "visibility": ["Build", "Release"],
        "runsOn": ["Agent", "DeploymentGroup"],
        "author": "Microsoft Corporation",
        "version": {
            "Major": 3,
            "Minor": 231,
            "Patch": 0
        },
        "instanceNameFormat": "Bash Script",
        "groups": [
            {
                "name": "advanced",
                "displayName": "Advanced",
                "isExpanded": false
            }
        ],
        "inputs": [
            {
                "name": "targetType",
                "type": "radio",
                "label": "Type",
                "required": false,
                "defaultValue": "filePath",
                "options": {
                    "filePath": "File Path",
                    "inline": "Inline"
                }
            },
            {
                "name": "filePath",
                "type": "filePath",
                "label": "Script Path",
                "required": true,
                "visibleRule": "targetType = filePath"
            },
            {
                "name": "script",
                "type": "multiLine",
                "label": "Script",
                "required": true,
                "defaultValue": "echo Hello world",
                "visibleRule": "targetType = inline"
            },
            {
                "name": "workingDirectory",
                "type": "filePath",
                "label": "Working Directory",
                "groupName": "advanced"
            },
            {
                "name": "failOnStderr",
                "type": "boolean",
                "label": "Fail on Standard Error",
                "defaultValue": "false",
                "groupName": "advanced"
            }
        ],
        "execution": {
            "Node10": {
                "target": "bash.js"
            },
            "Node16": {
                "target": "bash.js"
            }
        }
    }"#;

    #[test]
    fn test_parse_task_manifest() {
        let manifest = TaskManifest::from_str(SAMPLE_TASK_JSON).unwrap();

        assert_eq!(manifest.name, "Bash");
        assert_eq!(manifest.friendly_name, Some("Bash".to_string()));
        assert_eq!(manifest.version.major, 3);
        assert_eq!(manifest.version.minor, 231);
        assert_eq!(manifest.version.patch, 0);
        assert_eq!(manifest.version_string(), "3.231.0");
    }

    #[test]
    fn test_task_inputs() {
        let manifest = TaskManifest::from_str(SAMPLE_TASK_JSON).unwrap();

        assert_eq!(manifest.inputs.len(), 5);
        assert_eq!(manifest.inputs[0].name, "targetType");
        assert_eq!(
            manifest.inputs[0].default_value,
            Some("filePath".to_string())
        );
    }

    #[test]
    fn test_required_inputs() {
        let manifest = TaskManifest::from_str(SAMPLE_TASK_JSON).unwrap();
        let required = manifest.required_inputs();

        assert!(required.contains(&"filePath"));
        assert!(required.contains(&"script"));
    }

    #[test]
    fn test_default_values() {
        let manifest = TaskManifest::from_str(SAMPLE_TASK_JSON).unwrap();
        let defaults = manifest.default_values();

        assert_eq!(defaults.get("targetType"), Some(&"filePath".to_string()));
        assert_eq!(defaults.get("failOnStderr"), Some(&"false".to_string()));
    }

    #[test]
    fn test_is_node_task() {
        let manifest = TaskManifest::from_str(SAMPLE_TASK_JSON).unwrap();
        assert!(manifest.is_node_task());
        assert!(!manifest.is_powershell_task());
    }

    #[test]
    fn test_primary_execution() {
        let manifest = TaskManifest::from_str(SAMPLE_TASK_JSON).unwrap();
        let exec = manifest.primary_execution().unwrap();

        assert_eq!(exec.target, "bash.js");
    }
}
