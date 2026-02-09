use crate::output;

use std::path::PathBuf;

use clap::Args;
use color_eyre::Result;

use pipeline_service::utils::find_repo_root;
use pipeline_service::{normalize_pipeline, AzureParser, PipelineValidator, TemplateEngine};

/// Validate a pipeline YAML file
#[derive(Args, Debug)]
pub struct ValidateArgs {
    /// Path to the pipeline YAML file
    pub pipeline: PathBuf,

    /// Also validate template resolution
    #[arg(long)]
    pub templates: bool,

    /// Repository root for template resolution (default: current directory)
    #[arg(long, value_name = "DIR")]
    pub repo_root: Option<PathBuf>,
}

pub fn execute(args: ValidateArgs) -> Result<()> {
    let pipeline_path = &args.pipeline;

    if !pipeline_path.exists() {
        color_eyre::eyre::bail!("Pipeline file not found: {}", pipeline_path.display());
    }

    // Step 1: Parse YAML syntax
    output::status("Validating", &format!("{}", pipeline_path.display()));

    let raw_pipeline = match AzureParser::parse_file(pipeline_path) {
        Ok(p) => p,
        Err(e) => {
            output::error(&format!("Parse error: {}", e.message));
            if let Some(suggestion) = &e.suggestion {
                output::info(&format!("  Suggestion: {}", suggestion));
            }
            std::process::exit(1);
        }
    };

    output::check("YAML syntax valid");

    // Step 2: Normalize pipeline
    let pipeline = normalize_pipeline(raw_pipeline);

    let stages_count = pipeline.stages.len();
    let jobs_count: usize = pipeline.stages.iter().map(|s| s.jobs.len()).sum();
    let steps_count: usize = pipeline
        .stages
        .iter()
        .flat_map(|s| &s.jobs)
        .map(|j| j.steps.len())
        .sum();

    output::check(&format!(
        "Structure: {} stages, {} jobs, {} steps",
        stages_count, jobs_count, steps_count
    ));

    // Step 3: Semantic validation
    match PipelineValidator::validate(&pipeline) {
        Ok(()) => {
            output::check("Semantic validation passed");
        }
        Err(errors) => {
            output::error(&format!("{} validation error(s):", errors.len()));
            for error in &errors {
                output::error(&format!("  - [{}] {}", error.path, error.message));
            }
            std::process::exit(1);
        }
    }

    // Step 4: Template validation (optional)
    if args.templates {
        let repo_root = args.repo_root.clone().unwrap_or_else(|| {
            let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            find_repo_root(&cwd).unwrap_or(cwd)
        });

        output::status("Resolving", "templates...");

        let mut engine = TemplateEngine::new(repo_root);
        match engine.resolve_pipeline(pipeline) {
            Ok(resolved) => {
                let resolved_stages = resolved.stages.len();
                let resolved_jobs: usize = resolved.stages.iter().map(|s| s.jobs.len()).sum();
                let resolved_steps: usize = resolved
                    .stages
                    .iter()
                    .flat_map(|s| &s.jobs)
                    .map(|j| j.steps.len())
                    .sum();

                output::check(&format!(
                    "Templates resolved: {} stages, {} jobs, {} steps",
                    resolved_stages, resolved_jobs, resolved_steps
                ));
            }
            Err(e) => {
                output::error(&format!("Template error: {}", e));
                std::process::exit(1);
            }
        }
    }

    println!();
    output::success("Pipeline is valid");

    Ok(())
}
