use color_eyre::Result;
use service::pipeline::{
    ExecutionContext, ExecutionEvent, PipelineExecutor, PipelineParser,
};
use std::env;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: {} run <pipeline.yaml>", args[0]);
        std::process::exit(1);
    }

    let command = &args[1];
    if command != "run" {
        eprintln!("Unknown command: {}", command);
        eprintln!("Usage: {} run <pipeline.yaml>", args[0]);
        std::process::exit(1);
    }

    let pipeline_path = &args[2];
    println!("Loading pipeline from: {}", pipeline_path);

    let pipeline = PipelineParser::from_file(pipeline_path)?;
    println!("Pipeline: {}", pipeline.name);
    if let Some(desc) = &pipeline.description {
        println!("Description: {}", desc);
    }
    println!("Steps: {}", pipeline.steps.len());
    println!();

    let working_dir = env::current_dir()?.to_string_lossy().to_string();
    let context = ExecutionContext::new(pipeline.name.clone(), working_dir);

    let (tx, mut rx) = mpsc::unbounded_channel();

    let executor = PipelineExecutor::new(context);
    let executor_handle = tokio::spawn(async move {
        executor.execute(pipeline, Some(tx)).await
    });

    while let Some(event) = rx.recv().await {
        match event {
            ExecutionEvent::PipelineStarted { name } => {
                println!("==> Pipeline started: {}\n", name);
            }
            ExecutionEvent::StepStarted { step_name, step_index } => {
                println!("[Step {}/...] Running: {}", step_index + 1, step_name);
            }
            ExecutionEvent::StepOutput { output, .. } => {
                println!("  | {}", output);
            }
            ExecutionEvent::StepCompleted { result, step_index } => {
                println!(
                    "[Step {}/...] {} - {:?} ({}ms, exit code: {:?})",
                    step_index + 1,
                    result.step_name,
                    result.status,
                    result.duration.as_millis(),
                    result.exit_code
                );
                if let Some(error) = &result.error {
                    println!("  Error: {}", error);
                }
                println!();
            }
            ExecutionEvent::PipelineCompleted {
                success,
                total_steps,
                failed_steps,
            } => {
                println!("==> Pipeline completed!");
                println!("Total steps: {}", total_steps);
                println!("Failed steps: {}", failed_steps);
                println!("Status: {}", if success { "✓ SUCCESS" } else { "✗ FAILED" });
            }
        }
    }

    let _results = executor_handle.await?;

    Ok(())
}
