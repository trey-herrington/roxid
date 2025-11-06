use color_eyre::Result;
use std::collections::HashMap;
use std::env;
use std::process::{Command, Stdio};
use std::time::Duration;

pub mod proto {
    tonic::include_proto!("pipeline");
}

use proto::{
    pipeline_service_client::PipelineServiceClient, parse_pipeline_request, ExecutePipelineRequest,
    ParsePipelineRequest,
};

// Check if the gRPC service is running
fn is_service_running() -> bool {
    std::net::TcpStream::connect("127.0.0.1:50051")
        .or_else(|_| std::net::TcpStream::connect("[::1]:50051"))
        .is_ok()
}

// Stop the gRPC service
fn stop_service() {
    // Try to find and kill the pipeline-service process
    #[cfg(unix)]
    {
        use std::process::Command;
        let _ = Command::new("pkill")
            .arg("-f")
            .arg("pipeline-service")
            .output();
    }
    
    #[cfg(windows)]
    {
        use std::process::Command;
        let _ = Command::new("taskkill")
            .args(&["/F", "/IM", "pipeline-service.exe"])
            .output();
    }
}

// Start the gRPC service in the background
fn start_service() -> Result<bool> {
    let exe_path = env::current_exe()?;
    let exe_dir = exe_path.parent().ok_or_else(|| {
        color_eyre::eyre::eyre!("Failed to get executable directory")
    })?;
    
    let service_path = exe_dir.join("pipeline-service");
    
    if !service_path.exists() {
        return Err(color_eyre::eyre::eyre!(
            "pipeline-service binary not found at: {}. Make sure both binaries are installed.",
            service_path.display()
        ));
    }
    
    println!("Starting pipeline service...");
    
    Command::new(&service_path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    
    // Wait for service to be ready
    for i in 0..30 {
        if is_service_running() {
            println!("Service ready!");
            return Ok(true); // We started it
        }
        std::thread::sleep(Duration::from_millis(500));
        if i == 29 {
            return Err(color_eyre::eyre::eyre!("Service failed to start after 15 seconds"));
        }
    }
    
    Ok(true)
}

// Ensure service is running, start if needed, returns whether we started it
async fn ensure_service_running() -> Result<bool> {
    if !is_service_running() {
        start_service()
    } else {
        Ok(false) // Service was already running
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let args: Vec<String> = env::args().collect();

    // If no arguments, launch the TUI
    if args.len() == 1 {
        let we_started_service = ensure_service_running().await?;
        let result = roxid_tui::run().await;
        
        // Stop service if we started it
        if we_started_service {
            println!("Stopping service...");
            stop_service();
        }
        
        return result;
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

    // Resolve to absolute path
    let absolute_path = if std::path::Path::new(pipeline_path).is_absolute() {
        pipeline_path.to_string()
    } else {
        env::current_dir()?
            .join(pipeline_path)
            .to_string_lossy()
            .to_string()
    };

    // Ensure service is running
    let we_started_service = ensure_service_running().await?;

    // Connect to gRPC server
    let mut client = PipelineServiceClient::connect("http://[::1]:50051").await?;

    // Parse the pipeline
    let parse_request = ParsePipelineRequest {
        source: Some(parse_pipeline_request::Source::FilePath(absolute_path)),
    };

    let response = client.parse_pipeline(parse_request).await?;
    let pipeline = response
        .into_inner()
        .pipeline
        .ok_or_else(|| color_eyre::eyre::eyre!("No pipeline returned"))?;

    println!("Pipeline: {}", pipeline.name);
    if let Some(desc) = &pipeline.description {
        println!("Description: {}", desc);
    }
    
    // Count total steps from both formats
    let _total_steps = if !pipeline.stages.is_empty() {
        let step_count: usize = pipeline.stages.iter()
            .flat_map(|stage| &stage.jobs)
            .flat_map(|job| &job.steps)
            .count();
        println!("Stages: {}", pipeline.stages.len());
        println!("Total Steps: {}", step_count);
        step_count
    } else {
        println!("Steps: {}", pipeline.steps.len());
        pipeline.steps.len()
    };
    println!();

    let working_dir = env::current_dir()?.to_string_lossy().to_string();

    // Execute the pipeline
    let execute_request = ExecutePipelineRequest {
        pipeline: Some(pipeline.clone()),
        working_dir,
    };

    let mut stream = client
        .execute_pipeline(execute_request)
        .await?
        .into_inner();

    // Track current job for step counting - use a map for parallel jobs
    let mut job_step_counters: HashMap<String, usize> = HashMap::new();
    let mut job_total_steps: HashMap<String, usize> = HashMap::new();
    let mut step_to_job: HashMap<String, String> = HashMap::new();

    // Process events from the stream
    while let Some(event) = stream.message().await? {
        if let Some(e) = event.event {
            use proto::execution_event::Event;
            match e {
                Event::PipelineStarted(started) => {
                    println!("==> Pipeline started: {}\n", started.name);
                }
                Event::StageStarted(started) => {
                    println!(
                        "\n🎭 Stage #{}: {} started",
                        started.stage_index + 1,
                        started.stage_name
                    );
                }
                Event::StageCompleted(completed) => {
                    if let Some(result) = completed.result {
                        let status = proto::StageStatus::try_from(result.status)
                            .unwrap_or(proto::StageStatus::Pending);
                        println!(
                            "   Stage '{}' - {:?} ({}ms, {} jobs)",
                            result.stage_name,
                            status,
                            result.duration_ms,
                            result.job_results.len()
                        );
                    }
                }
                Event::JobStarted(started) => {
                    // Initialize step counter for this job
                    job_step_counters.insert(started.job_name.clone(), 0);
                    
                    // Find the job in the pipeline to get its step count and map steps to this job
                    if let Some(job) = pipeline
                        .stages
                        .iter()
                        .flat_map(|stage| &stage.jobs)
                        .find(|job| job.job == started.job_name)
                    {
                        job_total_steps.insert(started.job_name.clone(), job.steps.len());
                        
                        // Map each step name to this job
                        for step in &job.steps {
                            step_to_job.insert(step.name.clone(), started.job_name.clone());
                        }
                    }
                    
                    println!(
                        "  🔧 Job #{}: {} started",
                        started.job_index + 1,
                        started.job_name
                    );
                }
                Event::JobCompleted(completed) => {
                    if let Some(result) = completed.result {
                        let status = proto::JobStatus::try_from(result.status)
                            .unwrap_or(proto::JobStatus::Pending);
                        println!(
                            "     Job '{}' - {:?} ({}ms, {} steps)",
                            result.job_name,
                            status,
                            result.duration_ms,
                            result.step_results.len()
                        );
                    }
                }
                Event::StepStarted(started) => {
                    // Look up which job this step belongs to
                    if let Some(job_name) = step_to_job.get(&started.step_name) {
                        let counter = job_step_counters.entry(job_name.clone()).or_insert(0);
                        *counter += 1;
                        let total = job_total_steps.get(job_name).copied().unwrap_or(0);
                        
                        println!(
                            "      [Step {}/{}] Running: {}",
                            counter,
                            total,
                            started.step_name
                        );
                    } else {
                        println!("      [Step ?/?] Running: {}", started.step_name);
                    }
                }
                Event::StepOutput(output) => {
                    println!("        | {}", output.output);
                }
                Event::StepCompleted(completed) => {
                    if let Some(result) = completed.result {
                        let status = proto::StepStatus::try_from(result.status)
                            .unwrap_or(proto::StepStatus::Pending);
                        
                        // Look up which job this step belongs to
                        if let Some(job_name) = step_to_job.get(&result.step_name) {
                            let counter = job_step_counters.get(job_name).copied().unwrap_or(0);
                            let total = job_total_steps.get(job_name).copied().unwrap_or(0);
                            
                            println!(
                                "      [Step {}/{}] {} - {:?} ({}ms, exit code: {:?})",
                                counter,
                                total,
                                result.step_name,
                                status,
                                result.duration_ms,
                                result.exit_code
                            );
                        } else {
                            println!(
                                "      [Step ?/?] {} - {:?} ({}ms, exit code: {:?})",
                                result.step_name,
                                status,
                                result.duration_ms,
                                result.exit_code
                            );
                        }
                        
                        if let Some(error) = &result.error {
                            println!("        Error: {}", error);
                        }
                    }
                }
                Event::PipelineCompleted(completed) => {
                    println!("\n==> Pipeline completed!");
                    println!("Total steps: {}", completed.total_steps);
                    println!("Failed steps: {}", completed.failed_steps);
                    println!(
                        "Status: {}",
                        if completed.success {
                            "✓ SUCCESS"
                        } else {
                            "✗ FAILED"
                        }
                    );
                }
            }
        }
    }

    // Stop service if we started it
    if we_started_service {
        println!("\nStopping service...");
        stop_service();
    }

    Ok(())
}
