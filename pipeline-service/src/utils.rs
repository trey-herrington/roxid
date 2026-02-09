// Utility Functions
// Common helpers for path resolution and project root detection

use std::path::{Path, PathBuf};

/// Find the root of a git repository by walking up from the given starting path.
///
/// Traverses ancestor directories looking for a `.git` directory, which indicates
/// the repository root. Returns `None` if no `.git` directory is found (e.g., the
/// path is not inside a git repository).
///
/// # Arguments
/// * `start` - The starting directory to search from
///
/// # Returns
/// The path to the repository root, or `None` if not found
pub fn find_repo_root(start: &Path) -> Option<PathBuf> {
    // Canonicalize to resolve symlinks and get an absolute path
    let start = start.canonicalize().ok()?;
    for ancestor in start.ancestors() {
        if ancestor.join(".git").exists() {
            return Some(ancestor.to_path_buf());
        }
    }
    None
}

/// Resolve the working directory for pipeline execution.
///
/// Attempts to find the git repository root from the current directory.
/// Falls back to the current directory if no repository root is found.
pub fn resolve_working_dir() -> PathBuf {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    find_repo_root(&cwd).unwrap_or(cwd)
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::fs;

    #[test]
    fn test_find_repo_root_with_git_dir() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();

        // Create a .git directory at the root
        fs::create_dir(root.join(".git")).unwrap();

        // Create a nested subdirectory
        let sub = root.join("a").join("b").join("c");
        fs::create_dir_all(&sub).unwrap();

        // Should find the root from the nested subdirectory
        let found = find_repo_root(&sub);
        assert!(found.is_some());
        assert_eq!(
            found.unwrap().canonicalize().unwrap(),
            root.canonicalize().unwrap()
        );
    }

    #[test]
    fn test_find_repo_root_from_root_itself() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        fs::create_dir(root.join(".git")).unwrap();

        let found = find_repo_root(root);
        assert!(found.is_some());
        assert_eq!(
            found.unwrap().canonicalize().unwrap(),
            root.canonicalize().unwrap()
        );
    }

    #[test]
    fn test_find_repo_root_no_git_dir() {
        let temp = tempfile::tempdir().unwrap();
        let sub = temp.path().join("a").join("b");
        fs::create_dir_all(&sub).unwrap();

        // No .git directory anywhere in the temp tree
        // Note: This test may find the real repo root if temp is inside a git repo.
        // We check that it at least doesn't return the sub directory itself as a root
        // unless it happens to have a .git in an ancestor.
        let result = find_repo_root(&sub);
        if let Some(found) = result {
            // If it found something, it must actually have a .git directory
            assert!(found.join(".git").exists());
        }
        // If it found nothing, that's also valid
    }

    #[test]
    fn test_find_repo_root_nonexistent_path() {
        let result = find_repo_root(Path::new("/nonexistent/path/that/does/not/exist"));
        assert!(result.is_none());
    }

    #[test]
    fn test_resolve_working_dir_returns_path() {
        // resolve_working_dir should always return a valid path
        let dir = resolve_working_dir();
        // It should either be the repo root (if we're in a git repo) or cwd
        assert!(dir.exists() || dir == PathBuf::from("."));
    }
}
