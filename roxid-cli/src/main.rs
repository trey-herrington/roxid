use color_eyre::Result;
use pipeline_rpc::{ExecutionEvent, PipelineHandler};
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let args: Vec<String> = env::args().collect();
    
    // If no arguments, launch the TUI
    if args.len() == 1 {
        return roxid_tui::run().await;
    }
    
    if args.len() < 3 {
        eprintln!("Usage: {} run <pipeline.yaml>", args[0]);
        eprintln!("   or: {} (to launch TUI)", args[0]);
        std::process::exit(1);
    }

    let command = &args[1];
    if command != "run" {
        eprintln!("Unknown command: {}", command);
        eprintln!("Usage: {} run <pipeline.yaml>", args[0]);
        eprintln!("   or: {} (to launch TUI)", args[0]);
        std::process::exit(1);
    }

    let pipeline_path = &args[2];
    println!("Loading pipeline from: {}", pipeline_path);

    let handler = PipelineHandler::new();
    let pipeline = handler.parse_from_file(pipeline_path)?;
    
    println!("Pipeline: {}", pipeline.name);
    if let Some(desc) = &pipeline.description {
        println!("Description: {}", desc);
    }
    println!("Steps: {}", pipeline.steps.len());
    println!();

    let working_dir = env::current_dir()?.to_string_lossy().to_string();

    let (tx, mut rx) = PipelineHandler::create_event_channel();

    let handler_clone = PipelineHandler::new();
    let pipeline_clone = pipeline.clone();
    let executor_handle = tokio::spawn(async move {
        handler_clone.execute_pipeline(pipeline_clone, working_dir, Some(tx)).await
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

    let _result = executor_handle.await?;

    Ok(())
}
