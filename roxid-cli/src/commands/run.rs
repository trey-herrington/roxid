use crate::output;

use std::collections::HashMap;
use std::path::PathBuf;

use clap::Args;
use color_eyre::Result;

use pipeline_service::execution::events::progress_channel;
use pipeline_service::parser::models::{ExecutionContext, JobStatus, StageStatus, StepStatus};
use pipeline_service::{normalize_pipeline, AzureParser, ExecutionEvent, PipelineExecutor};

/// Run an Azure DevOps pipeline locally
#[derive(Args, Debug)]
pub struct RunArgs {
    /// Path to the pipeline YAML file
    pub pipeline: PathBuf,

    /// Set a variable (can be repeated, format: name=value)
    #[arg(long = "var", short = 'v', value_name = "NAME=VALUE")]
    pub variables: Vec<String>,

    /// Run only a specific stage
    #[arg(long, value_name = "STAGE")]
    pub stage: Option<String>,

    /// Run only a specific job
    #[arg(long, value_name = "JOB")]
    pub job: Option<String>,

    /// Working directory for execution
    #[arg(long, short = 'w', value_name = "DIR")]
    pub working_dir: Option<PathBuf>,

    /// Enable task runner with cache directory
    #[arg(long, value_name = "DIR")]
    pub task_cache: Option<PathBuf>,
}

pub async fn execute(args: RunArgs) -> Result<()> {
    let pipeline_path = &args.pipeline;

    if !pipeline_path.exists() {
        color_eyre::eyre::bail!("Pipeline file not found: {}", pipeline_path.display());
    }

    // Parse variables from --var flags
    let mut variables = HashMap::new();
    for var_str in &args.variables {
        if let Some((name, value)) = var_str.split_once('=') {
            variables.insert(name.to_string(), value.to_string());
        } else {
            color_eyre::eyre::bail!("Invalid variable format '{}'. Expected name=value", var_str);
        }
    }

    // Resolve working directory
    let working_dir = match &args.working_dir {
        Some(dir) => dir.clone(),
        None => std::env::current_dir()?,
    };

    // Parse the pipeline
    output::status("Parsing", &format!("{}", pipeline_path.display()));
    let raw_pipeline = AzureParser::parse_file(pipeline_path)
        .map_err(|e| color_eyre::eyre::eyre!("Parse error: {}", e.message))?;
    let pipeline = normalize_pipeline(raw_pipeline);

    let pipeline_name = pipeline.name.clone().unwrap_or_else(|| {
        pipeline_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("pipeline")
            .to_string()
    });

    let stages_count = pipeline.stages.len();
    let jobs_count: usize = pipeline.stages.iter().map(|s| s.jobs.len()).sum();
    let steps_count: usize = pipeline
        .stages
        .iter()
        .flat_map(|s| &s.jobs)
        .map(|j| j.steps.len())
        .sum();

    output::info(&format!(
        "Pipeline '{}': {} stages, {} jobs, {} steps",
        pipeline_name, stages_count, jobs_count, steps_count
    ));

    // Build execution context
    let context = ExecutionContext::new(
        pipeline_name.clone(),
        working_dir.to_string_lossy().to_string(),
    )
    .with_variables(variables);

    // Create progress channel and executor
    let (tx, mut rx) = progress_channel();

    let mut executor = PipelineExecutor::from_pipeline(&pipeline)
        .map_err(|e| color_eyre::eyre::eyre!("Failed to build execution graph: {}", e.message))?;
    executor = executor.with_progress(tx);

    if let Some(cache_dir) = args.task_cache {
        executor = executor.with_task_runner(cache_dir);
    }

    // Spawn execution in background
    let exec_handle = tokio::spawn(async move { executor.execute(context).await });

    // Process events in the foreground
    let mut overall_success = true;
    while let Some(event) = rx.recv().await {
        match &event {
            ExecutionEvent::PipelineStarted {
                pipeline_name,
                total_stages,
            } => {
                println!();
                output::header(&format!(
                    "Pipeline '{}' ({} stages)",
                    pipeline_name, total_stages
                ));
            }

            ExecutionEvent::PipelineCompleted {
                success, duration, ..
            } => {
                println!();
                overall_success = *success;
                if *success {
                    output::success(&format!(
                        "Pipeline completed successfully in {:.2}s",
                        duration.as_secs_f64()
                    ));
                } else {
                    output::failure(&format!(
                        "Pipeline failed after {:.2}s",
                        duration.as_secs_f64()
                    ));
                }
            }

            ExecutionEvent::StageStarted {
                stage_name,
                display_name,
                total_jobs,
            } => {
                let label = display_name.as_deref().unwrap_or(stage_name);
                output::stage_header(label, *total_jobs);
            }

            ExecutionEvent::StageCompleted {
                stage_name,
                status,
                duration,
            } => {
                let symbol = match status {
                    StageStatus::Succeeded => "OK",
                    StageStatus::Failed => "FAIL",
                    _ => "DONE",
                };
                let color_fn = if *status == StageStatus::Succeeded {
                    output::dim_success
                } else {
                    output::dim_failure
                };
                color_fn(&format!(
                    "  Stage '{}' {} ({:.2}s)",
                    stage_name,
                    symbol,
                    duration.as_secs_f64()
                ));
            }

            ExecutionEvent::StageSkipped {
                stage_name, reason, ..
            } => {
                output::warning(&format!("  Stage '{}' skipped: {}", stage_name, reason));
            }

            ExecutionEvent::JobStarted {
                job_name,
                display_name,
                total_steps,
                ..
            } => {
                let label = display_name.as_deref().unwrap_or(job_name);
                println!("    Job '{}' ({} steps)", label, total_steps);
            }

            ExecutionEvent::JobCompleted {
                job_name,
                status,
                duration,
                ..
            } => {
                let symbol = match status {
                    JobStatus::Succeeded => "OK",
                    JobStatus::Failed => "FAIL",
                    _ => "DONE",
                };
                if *status == JobStatus::Succeeded {
                    output::dim_success(&format!(
                        "    Job '{}' {} ({:.2}s)",
                        job_name,
                        symbol,
                        duration.as_secs_f64()
                    ));
                } else {
                    output::dim_failure(&format!(
                        "    Job '{}' {} ({:.2}s)",
                        job_name,
                        symbol,
                        duration.as_secs_f64()
                    ));
                }
            }

            ExecutionEvent::JobSkipped {
                job_name, reason, ..
            } => {
                output::warning(&format!("    Job '{}' skipped: {}", job_name, reason));
            }

            ExecutionEvent::StepStarted {
                step_name,
                display_name,
                step_index,
                ..
            } => {
                let label = display_name
                    .as_deref()
                    .or(step_name.as_deref())
                    .unwrap_or("step");
                println!("      [Step {}] {}", step_index + 1, label);
            }

            ExecutionEvent::StepOutput {
                output, is_error, ..
            } => {
                for line in output.lines() {
                    if *is_error {
                        output::step_error(line);
                    } else {
                        output::step_output(line);
                    }
                }
            }

            ExecutionEvent::StepCompleted {
                status,
                duration,
                exit_code,
                ..
            } => {
                let symbol = match status {
                    StepStatus::Succeeded => "OK",
                    StepStatus::Failed => "FAIL",
                    StepStatus::Skipped => "SKIP",
                    _ => "DONE",
                };
                let exit_info = match exit_code {
                    Some(code) if *code != 0 => format!(" (exit code: {})", code),
                    _ => String::new(),
                };
                if *status == StepStatus::Succeeded {
                    output::dim_success(&format!(
                        "        {} ({:.2}s){}",
                        symbol,
                        duration.as_secs_f64(),
                        exit_info,
                    ));
                } else if *status == StepStatus::Failed {
                    output::dim_failure(&format!(
                        "        {} ({:.2}s){}",
                        symbol,
                        duration.as_secs_f64(),
                        exit_info,
                    ));
                } else {
                    println!(
                        "        {} ({:.2}s){}",
                        symbol,
                        duration.as_secs_f64(),
                        exit_info,
                    );
                }
            }

            ExecutionEvent::StepSkipped {
                step_name, reason, ..
            } => {
                let label = step_name.as_deref().unwrap_or("step");
                output::warning(&format!("        {} skipped: {}", label, reason));
            }

            ExecutionEvent::VariableSet {
                name,
                value,
                is_secret,
                ..
            } => {
                let display_value = if *is_secret { "***" } else { value.as_str() };
                output::dim(&format!("        [var] {} = {}", name, display_value));
            }

            ExecutionEvent::Log { level, message, .. } => {
                use pipeline_service::execution::events::LogLevel;
                match level {
                    LogLevel::Error => output::error(message),
                    LogLevel::Warning => output::warning(message),
                    _ => output::dim(message),
                }
            }

            ExecutionEvent::Error { message, .. } => {
                output::error(&format!("ERROR: {}", message));
            }
        }
    }

    // Wait for executor to finish
    let _result = exec_handle.await?;

    if !overall_success {
        std::process::exit(1);
    }

    Ok(())
}
