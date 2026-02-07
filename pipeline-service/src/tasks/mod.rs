// Tasks Module
// Provides Azure DevOps task caching and manifest parsing

pub mod cache;
pub mod manifest;

// Re-export key types
pub use cache::{TaskCache, TaskCacheConfig, TaskCacheError};
pub use manifest::{TaskExecution, TaskInput, TaskManifest, TaskManifestError};
