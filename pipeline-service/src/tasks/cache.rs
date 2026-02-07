// Task Cache
// Downloads and caches Azure DevOps tasks from the marketplace

use crate::tasks::manifest::{TaskManifest, TaskManifestError};

use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;

/// Errors that can occur with task caching
#[derive(Debug, Error)]
pub enum TaskCacheError {
    #[error("Task not found: {0}@{1}")]
    TaskNotFound(String, String),

    #[error("Failed to download task: {0}")]
    DownloadError(String),

    #[error("IO error: {0}")]
    IoError(#[from] io::Error),

    #[error("Manifest error: {0}")]
    ManifestError(#[from] TaskManifestError),

    #[error("Invalid task reference: {0}")]
    InvalidTaskReference(String),

    #[error("HTTP error: {0}")]
    HttpError(String),

    #[error("Archive error: {0}")]
    ArchiveError(String),
}

/// Configuration for the task cache
#[derive(Debug, Clone)]
pub struct TaskCacheConfig {
    /// Cache directory (default: ~/.roxid/tasks/)
    pub cache_dir: PathBuf,

    /// Whether to allow downloading tasks
    pub allow_download: bool,

    /// Custom task sources (for testing)
    pub task_sources: Vec<TaskSource>,
}

impl Default for TaskCacheConfig {
    fn default() -> Self {
        let cache_dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".roxid")
            .join("tasks");

        Self {
            cache_dir,
            allow_download: true,
            task_sources: vec![TaskSource::AzureDevOps],
        }
    }
}

/// Task source for downloading tasks
#[derive(Debug, Clone)]
pub enum TaskSource {
    /// Azure DevOps built-in tasks from GitHub
    AzureDevOps,
    /// Local directory containing tasks
    LocalDir(PathBuf),
    /// Custom URL pattern (task name and version substituted)
    CustomUrl(String),
}

/// A cached task
#[derive(Debug, Clone)]
pub struct CachedTask {
    /// Task name
    pub name: String,

    /// Task version
    pub version: String,

    /// Path to the task directory
    pub path: PathBuf,

    /// Parsed task manifest
    pub manifest: TaskManifest,
}

impl CachedTask {
    /// Get the path to the task's execution target
    pub fn execution_target(&self) -> Option<PathBuf> {
        let exec = self.manifest.primary_execution()?;
        Some(self.path.join(&exec.target))
    }
}

/// Task cache for managing Azure DevOps tasks
pub struct TaskCache {
    config: TaskCacheConfig,
    /// In-memory cache of loaded manifests
    cache: Arc<RwLock<HashMap<String, CachedTask>>>,
}

impl TaskCache {
    /// Create a new task cache with default configuration
    pub fn new() -> Self {
        Self::with_config(TaskCacheConfig::default())
    }

    /// Create a task cache with custom configuration
    pub fn with_config(config: TaskCacheConfig) -> Self {
        Self {
            config,
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a task cache with a specific cache directory
    pub fn with_cache_dir(cache_dir: impl AsRef<Path>) -> Self {
        let config = TaskCacheConfig {
            cache_dir: cache_dir.as_ref().to_path_buf(),
            ..Default::default()
        };
        Self::with_config(config)
    }

    /// Get the cache directory
    pub fn cache_dir(&self) -> &Path {
        &self.config.cache_dir
    }

    /// Parse a task reference (e.g., "Bash@3" or "DotNetCoreCLI@2.123.4")
    pub fn parse_task_reference(task_ref: &str) -> Result<(String, String), TaskCacheError> {
        let parts: Vec<&str> = task_ref.split('@').collect();
        if parts.len() != 2 {
            return Err(TaskCacheError::InvalidTaskReference(task_ref.to_string()));
        }

        let name = parts[0].to_string();
        let version = parts[1].to_string();

        // Version can be just major (e.g., "3") or full (e.g., "3.231.0")
        if version.is_empty() || name.is_empty() {
            return Err(TaskCacheError::InvalidTaskReference(task_ref.to_string()));
        }

        Ok((name, version))
    }

    /// Get a task from the cache, downloading if necessary
    pub async fn get_task(&self, task_ref: &str) -> Result<CachedTask, TaskCacheError> {
        let (name, version) = Self::parse_task_reference(task_ref)?;
        self.get_task_by_name_version(&name, &version).await
    }

    /// Get a task by name and version
    pub async fn get_task_by_name_version(
        &self,
        name: &str,
        version: &str,
    ) -> Result<CachedTask, TaskCacheError> {
        let cache_key = format!("{}@{}", name, version);

        // Check in-memory cache first
        {
            let cache = self.cache.read().await;
            if let Some(task) = cache.get(&cache_key) {
                return Ok(task.clone());
            }
        }

        // Check disk cache
        let task_path = self.task_path(name, version);
        if task_path.exists() {
            let task = self.load_cached_task(name, version, &task_path)?;

            // Store in memory cache
            let mut cache = self.cache.write().await;
            cache.insert(cache_key, task.clone());

            return Ok(task);
        }

        // Download if allowed
        if self.config.allow_download {
            let task = self.download_task(name, version).await?;

            // Store in memory cache
            let mut cache = self.cache.write().await;
            cache.insert(cache_key, task.clone());

            return Ok(task);
        }

        Err(TaskCacheError::TaskNotFound(
            name.to_string(),
            version.to_string(),
        ))
    }

    /// Get the path where a task would be cached
    fn task_path(&self, name: &str, version: &str) -> PathBuf {
        self.config.cache_dir.join(name).join(version)
    }

    /// Load a cached task from disk
    fn load_cached_task(
        &self,
        name: &str,
        version: &str,
        path: &Path,
    ) -> Result<CachedTask, TaskCacheError> {
        let manifest_path = path.join("task.json");
        let manifest = TaskManifest::from_file(&manifest_path)?;

        Ok(CachedTask {
            name: name.to_string(),
            version: version.to_string(),
            path: path.to_path_buf(),
            manifest,
        })
    }

    /// Download a task from available sources
    async fn download_task(&self, name: &str, version: &str) -> Result<CachedTask, TaskCacheError> {
        for source in &self.config.task_sources {
            match self.download_from_source(source, name, version).await {
                Ok(task) => return Ok(task),
                Err(_) => continue,
            }
        }

        Err(TaskCacheError::TaskNotFound(
            name.to_string(),
            version.to_string(),
        ))
    }

    /// Download a task from a specific source
    async fn download_from_source(
        &self,
        source: &TaskSource,
        name: &str,
        version: &str,
    ) -> Result<CachedTask, TaskCacheError> {
        match source {
            TaskSource::AzureDevOps => self.download_from_azure_devops(name, version).await,
            TaskSource::LocalDir(dir) => self.load_from_local_dir(dir, name, version).await,
            TaskSource::CustomUrl(pattern) => {
                self.download_from_custom_url(pattern, name, version).await
            }
        }
    }

    /// Download from Azure DevOps built-in tasks (GitHub)
    async fn download_from_azure_devops(
        &self,
        name: &str,
        version: &str,
    ) -> Result<CachedTask, TaskCacheError> {
        // Azure DevOps built-in tasks are on GitHub:
        // https://github.com/microsoft/azure-pipelines-tasks
        //
        // For now, we'll create a placeholder since downloading from GitHub
        // requires more complex handling (finding the right version, extracting, etc.)

        // Create task directory
        let task_path = self.task_path(name, version);
        fs::create_dir_all(&task_path)?;

        // For built-in tasks, we can use common patterns
        // This is a simplified implementation - real implementation would
        // download from GitHub releases or npm packages

        // Check if this is a known built-in task and create a stub
        if let Some(manifest) = create_builtin_task_stub(name, version) {
            let manifest_path = task_path.join("task.json");
            let manifest_json = serde_json::to_string_pretty(&manifest)
                .map_err(|e| TaskCacheError::DownloadError(e.to_string()))?;
            fs::write(&manifest_path, manifest_json)?;

            return Ok(CachedTask {
                name: name.to_string(),
                version: version.to_string(),
                path: task_path,
                manifest,
            });
        }

        Err(TaskCacheError::TaskNotFound(
            name.to_string(),
            version.to_string(),
        ))
    }

    /// Load from a local directory
    async fn load_from_local_dir(
        &self,
        dir: &Path,
        name: &str,
        version: &str,
    ) -> Result<CachedTask, TaskCacheError> {
        // Look for task in local directory
        let task_path = dir.join(name).join(version);
        if task_path.exists() {
            return self.load_cached_task(name, version, &task_path);
        }

        // Try just the task name (for single-version tasks)
        let task_path = dir.join(name);
        if task_path.exists() {
            return self.load_cached_task(name, version, &task_path);
        }

        Err(TaskCacheError::TaskNotFound(
            name.to_string(),
            version.to_string(),
        ))
    }

    /// Download from a custom URL
    async fn download_from_custom_url(
        &self,
        pattern: &str,
        name: &str,
        version: &str,
    ) -> Result<CachedTask, TaskCacheError> {
        let url = pattern
            .replace("{name}", name)
            .replace("{version}", version);

        // For now, this is a stub - real implementation would use reqwest to download
        Err(TaskCacheError::DownloadError(format!(
            "Custom URL download not yet implemented: {}",
            url
        )))
    }

    /// List all cached tasks
    pub fn list_cached_tasks(&self) -> io::Result<Vec<(String, String)>> {
        let mut tasks = Vec::new();

        if !self.config.cache_dir.exists() {
            return Ok(tasks);
        }

        for entry in fs::read_dir(&self.config.cache_dir)? {
            let entry = entry?;
            let task_name = entry.file_name().to_string_lossy().to_string();

            if entry.file_type()?.is_dir() {
                for version_entry in fs::read_dir(entry.path())? {
                    let version_entry = version_entry?;
                    if version_entry.file_type()?.is_dir() {
                        let version = version_entry.file_name().to_string_lossy().to_string();
                        tasks.push((task_name.clone(), version));
                    }
                }
            }
        }

        Ok(tasks)
    }

    /// Clear all cached tasks
    pub fn clear_cache(&self) -> io::Result<()> {
        if self.config.cache_dir.exists() {
            fs::remove_dir_all(&self.config.cache_dir)?;
        }
        Ok(())
    }

    /// Clear a specific cached task
    pub fn clear_task(&self, name: &str, version: &str) -> io::Result<()> {
        let task_path = self.task_path(name, version);
        if task_path.exists() {
            fs::remove_dir_all(task_path)?;
        }
        Ok(())
    }
}

impl Default for TaskCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Create a stub manifest for built-in Azure DevOps tasks
fn create_builtin_task_stub(name: &str, version: &str) -> Option<TaskManifest> {
    // Parse major version
    let major: u32 = version.split('.').next()?.parse().ok()?;

    match name {
        "Bash" => Some(TaskManifest {
            id: "6c731c3c-3c68-459a-a5c9-bde6e6595b5b".to_string(),
            name: "Bash".to_string(),
            friendly_name: Some("Bash".to_string()),
            description: Some("Run a Bash script".to_string()),
            help_url: None,
            help_mark_down: None,
            category: Some("Utility".to_string()),
            visibility: Some(vec!["Build".to_string(), "Release".to_string()]),
            runs_on: Some(vec!["Agent".to_string()]),
            author: Some("Microsoft Corporation".to_string()),
            version: crate::tasks::manifest::TaskVersion {
                major,
                minor: 0,
                patch: 0,
            },
            minimum_agent_version: None,
            instance_name_format: Some("Bash Script".to_string()),
            groups: None,
            inputs: vec![
                crate::tasks::manifest::TaskInput {
                    name: "targetType".to_string(),
                    input_type: Some("radio".to_string()),
                    label: Some("Type".to_string()),
                    default_value: Some("inline".to_string()),
                    required: Some(false),
                    help_mark_down: None,
                    group_name: None,
                    visible_rule: None,
                    options: None,
                    properties: None,
                    validation: None,
                    aliases: None,
                },
                crate::tasks::manifest::TaskInput {
                    name: "script".to_string(),
                    input_type: Some("multiLine".to_string()),
                    label: Some("Script".to_string()),
                    default_value: None,
                    required: Some(true),
                    help_mark_down: None,
                    group_name: None,
                    visible_rule: Some("targetType = inline".to_string()),
                    options: None,
                    properties: None,
                    validation: None,
                    aliases: None,
                },
                crate::tasks::manifest::TaskInput {
                    name: "workingDirectory".to_string(),
                    input_type: Some("filePath".to_string()),
                    label: Some("Working Directory".to_string()),
                    default_value: None,
                    required: Some(false),
                    help_mark_down: None,
                    group_name: None,
                    visible_rule: None,
                    options: None,
                    properties: None,
                    validation: None,
                    aliases: None,
                },
            ],
            output_variables: None,
            execution: None, // Will be handled specially by the runner
            pre_job_execution: None,
            post_job_execution: None,
            data_source_bindings: None,
            messages: None,
            restrictions: None,
            demands: None,
        }),

        "PowerShell" => Some(TaskManifest {
            id: "e213ff0f-5d5c-4791-802d-52ea3e7be1f1".to_string(),
            name: "PowerShell".to_string(),
            friendly_name: Some("PowerShell".to_string()),
            description: Some("Run a PowerShell script".to_string()),
            help_url: None,
            help_mark_down: None,
            category: Some("Utility".to_string()),
            visibility: Some(vec!["Build".to_string(), "Release".to_string()]),
            runs_on: Some(vec!["Agent".to_string()]),
            author: Some("Microsoft Corporation".to_string()),
            version: crate::tasks::manifest::TaskVersion {
                major,
                minor: 0,
                patch: 0,
            },
            minimum_agent_version: None,
            instance_name_format: Some("PowerShell Script".to_string()),
            groups: None,
            inputs: vec![
                crate::tasks::manifest::TaskInput {
                    name: "targetType".to_string(),
                    input_type: Some("radio".to_string()),
                    label: Some("Type".to_string()),
                    default_value: Some("inline".to_string()),
                    required: Some(false),
                    help_mark_down: None,
                    group_name: None,
                    visible_rule: None,
                    options: None,
                    properties: None,
                    validation: None,
                    aliases: None,
                },
                crate::tasks::manifest::TaskInput {
                    name: "script".to_string(),
                    input_type: Some("multiLine".to_string()),
                    label: Some("Script".to_string()),
                    default_value: None,
                    required: Some(true),
                    help_mark_down: None,
                    group_name: None,
                    visible_rule: Some("targetType = inline".to_string()),
                    options: None,
                    properties: None,
                    validation: None,
                    aliases: None,
                },
                crate::tasks::manifest::TaskInput {
                    name: "workingDirectory".to_string(),
                    input_type: Some("filePath".to_string()),
                    label: Some("Working Directory".to_string()),
                    default_value: None,
                    required: Some(false),
                    help_mark_down: None,
                    group_name: None,
                    visible_rule: None,
                    options: None,
                    properties: None,
                    validation: None,
                    aliases: None,
                },
                crate::tasks::manifest::TaskInput {
                    name: "pwsh".to_string(),
                    input_type: Some("boolean".to_string()),
                    label: Some("Use PowerShell Core".to_string()),
                    default_value: Some("false".to_string()),
                    required: Some(false),
                    help_mark_down: None,
                    group_name: None,
                    visible_rule: None,
                    options: None,
                    properties: None,
                    validation: None,
                    aliases: None,
                },
            ],
            output_variables: None,
            execution: None,
            pre_job_execution: None,
            post_job_execution: None,
            data_source_bindings: None,
            messages: None,
            restrictions: None,
            demands: None,
        }),

        "CmdLine" => Some(TaskManifest {
            id: "d9bafed4-0b18-4f58-968d-86655b4d2ce9".to_string(),
            name: "CmdLine".to_string(),
            friendly_name: Some("Command line".to_string()),
            description: Some("Run a command line script".to_string()),
            help_url: None,
            help_mark_down: None,
            category: Some("Utility".to_string()),
            visibility: Some(vec!["Build".to_string(), "Release".to_string()]),
            runs_on: Some(vec!["Agent".to_string()]),
            author: Some("Microsoft Corporation".to_string()),
            version: crate::tasks::manifest::TaskVersion {
                major,
                minor: 0,
                patch: 0,
            },
            minimum_agent_version: None,
            instance_name_format: Some("Command Line Script".to_string()),
            groups: None,
            inputs: vec![
                crate::tasks::manifest::TaskInput {
                    name: "script".to_string(),
                    input_type: Some("multiLine".to_string()),
                    label: Some("Script".to_string()),
                    default_value: None,
                    required: Some(true),
                    help_mark_down: None,
                    group_name: None,
                    visible_rule: None,
                    options: None,
                    properties: None,
                    validation: None,
                    aliases: None,
                },
                crate::tasks::manifest::TaskInput {
                    name: "workingDirectory".to_string(),
                    input_type: Some("filePath".to_string()),
                    label: Some("Working Directory".to_string()),
                    default_value: None,
                    required: Some(false),
                    help_mark_down: None,
                    group_name: None,
                    visible_rule: None,
                    options: None,
                    properties: None,
                    validation: None,
                    aliases: None,
                },
            ],
            output_variables: None,
            execution: None,
            pre_job_execution: None,
            post_job_execution: None,
            data_source_bindings: None,
            messages: None,
            restrictions: None,
            demands: None,
        }),

        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_task_reference() {
        let (name, version) = TaskCache::parse_task_reference("Bash@3").unwrap();
        assert_eq!(name, "Bash");
        assert_eq!(version, "3");

        let (name, version) = TaskCache::parse_task_reference("DotNetCoreCLI@2.123.4").unwrap();
        assert_eq!(name, "DotNetCoreCLI");
        assert_eq!(version, "2.123.4");
    }

    #[test]
    fn test_parse_invalid_task_reference() {
        assert!(TaskCache::parse_task_reference("Bash").is_err());
        assert!(TaskCache::parse_task_reference("Bash@").is_err());
        assert!(TaskCache::parse_task_reference("@3").is_err());
    }

    #[test]
    fn test_builtin_task_stub_bash() {
        let manifest = create_builtin_task_stub("Bash", "3").unwrap();
        assert_eq!(manifest.name, "Bash");
        assert_eq!(manifest.version.major, 3);
    }

    #[test]
    fn test_builtin_task_stub_powershell() {
        let manifest = create_builtin_task_stub("PowerShell", "2").unwrap();
        assert_eq!(manifest.name, "PowerShell");
        assert_eq!(manifest.version.major, 2);
    }

    #[test]
    fn test_builtin_task_stub_unknown() {
        let manifest = create_builtin_task_stub("UnknownTask", "1");
        assert!(manifest.is_none());
    }

    #[tokio::test]
    async fn test_task_cache_config() {
        let cache = TaskCache::new();
        assert!(cache.config.allow_download);
        assert!(!cache.config.task_sources.is_empty());
    }
}
