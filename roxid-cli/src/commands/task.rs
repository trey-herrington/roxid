use crate::output;

use clap::{Args, Subcommand};
use color_eyre::Result;

use pipeline_service::TaskCache;

/// Manage the Azure DevOps task cache
#[derive(Args, Debug)]
pub struct TaskArgs {
    #[command(subcommand)]
    pub command: TaskCommand,
}

#[derive(Subcommand, Debug)]
pub enum TaskCommand {
    /// List cached tasks
    List,

    /// Pre-download a task (e.g., Bash@3)
    Fetch {
        /// Task reference (e.g., Bash@3, PowerShell@2)
        task_ref: String,
    },

    /// Clear the task cache
    Clear {
        /// Clear a specific task (e.g., Bash@3) instead of all
        task_ref: Option<String>,
    },

    /// Show task cache directory path
    Path,
}

pub async fn execute(args: TaskArgs) -> Result<()> {
    let cache = TaskCache::new();

    match args.command {
        TaskCommand::List => {
            output::status(
                "Tasks",
                &format!("cached in {}", cache.cache_dir().display()),
            );

            match cache.list_cached_tasks() {
                Ok(tasks) => {
                    if tasks.is_empty() {
                        output::dim("  No tasks cached");
                    } else {
                        for (name, version) in &tasks {
                            println!("  {}@{}", name, version);
                        }
                        println!();
                        output::dim(&format!("  {} task(s) total", tasks.len()));
                    }
                }
                Err(e) => {
                    output::error(&format!("Failed to list tasks: {}", e));
                    std::process::exit(1);
                }
            }
        }

        TaskCommand::Fetch { task_ref } => {
            output::status("Fetching", &task_ref);

            match cache.get_task(&task_ref).await {
                Ok(task) => {
                    output::success(&format!(
                        "Cached {}@{} at {}",
                        task.name,
                        task.version,
                        task.path.display()
                    ));
                }
                Err(e) => {
                    output::error(&format!("Failed to fetch '{}': {}", task_ref, e));
                    std::process::exit(1);
                }
            }
        }

        TaskCommand::Clear { task_ref } => {
            if let Some(task_ref) = task_ref {
                let (name, version) = TaskCache::parse_task_reference(&task_ref).map_err(|e| {
                    color_eyre::eyre::eyre!("Invalid task reference '{}': {}", task_ref, e)
                })?;

                output::status("Clearing", &format!("{}@{}", name, version));
                cache
                    .clear_task(&name, &version)
                    .map_err(|e| color_eyre::eyre::eyre!("Failed to clear task: {}", e))?;
                output::success("Task removed from cache");
            } else {
                output::status("Clearing", "all cached tasks");
                cache
                    .clear_cache()
                    .map_err(|e| color_eyre::eyre::eyre!("Failed to clear cache: {}", e))?;
                output::success("Task cache cleared");
            }
        }

        TaskCommand::Path => {
            println!("{}", cache.cache_dir().display());
        }
    }

    Ok(())
}
