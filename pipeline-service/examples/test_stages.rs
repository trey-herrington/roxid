use pipeline_service::pipeline::{ExecutionContext, PipelineExecutor, PipelineParser};

#[tokio::main]
async fn main() {
    let yaml = r#"
name: test-stages
stages:
  - stage: build
    jobs:
      - job: build_job
        steps:
          - name: Build step
            command: echo "Building..."
  - stage: test
    depends_on: [build]
    jobs:
      - job: test_job1
        steps:
          - name: Test 1
            command: echo "Testing 1..."
      - job: test_job2
        steps:
          - name: Test 2
            command: echo "Testing 2..."
"#;

    let pipeline = PipelineParser::parse(yaml).expect("Failed to parse");
    println!("✓ Parsed pipeline: {}", pipeline.name);
    println!("✓ Number of stages: {}", pipeline.stages.len());
    println!("✓ Is legacy format: {}", pipeline.is_legacy());

    if !pipeline.stages.is_empty() {
        for stage in &pipeline.stages {
            println!("\n  Stage: {}", stage.stage);
            if !stage.depends_on.is_empty() {
                println!("    Depends on: {:?}", stage.depends_on);
            }
            println!("    Jobs: {}", stage.jobs.len());
            for job in &stage.jobs {
                println!("      - {} ({} steps)", job.job, job.steps.len());
            }
        }
    }

    let context = ExecutionContext::new(
        pipeline.name.clone(),
        std::env::current_dir()
            .unwrap()
            .to_string_lossy()
            .to_string(),
    );

    println!("\n🚀 Executing pipeline...\n");
    let executor = PipelineExecutor::new(context);
    let results = executor.execute(pipeline, None).await;

    println!("\n✅ Execution completed!");
    println!("Total steps executed: {}", results.len());
    for (i, result) in results.iter().enumerate() {
        let status_icon = match result.status {
            pipeline_service::pipeline::StepStatus::Success => "✓",
            pipeline_service::pipeline::StepStatus::Failed => "✗",
            _ => "•",
        };
        println!(
            "  {} Step {}: {} - {:?}",
            status_icon,
            i + 1,
            result.step_name,
            result.status
        );
    }
}
