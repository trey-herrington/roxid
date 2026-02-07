use crate::output;

use std::path::PathBuf;

use clap::Args;
use color_eyre::Result;

use pipeline_service::{ReportFormat, TestFileParser, TestReporter, TestRunner};

/// Run pipeline tests
#[derive(Args, Debug)]
pub struct TestArgs {
    /// Path to test file (default: discover roxid-test.yml files)
    pub file: Option<PathBuf>,

    /// Filter tests by name pattern (supports * wildcards)
    #[arg(long, short = 'f', value_name = "PATTERN")]
    pub filter: Option<String>,

    /// Output format: terminal, junit, tap
    #[arg(long, short = 'o', default_value = "terminal")]
    pub output: String,

    /// Stop on first failure
    #[arg(long)]
    pub fail_fast: bool,

    /// Working directory for test execution
    #[arg(long, short = 'w', value_name = "DIR")]
    pub working_dir: Option<PathBuf>,
}

pub async fn execute(args: TestArgs) -> Result<()> {
    let format: ReportFormat = args
        .output
        .parse()
        .map_err(|e: String| color_eyre::eyre::eyre!("{}", e))?;

    // Build the test runner
    let mut runner = TestRunner::new();

    if let Some(dir) = &args.working_dir {
        runner = runner.with_working_dir(dir.to_string_lossy().to_string());
    }

    if let Some(filter) = &args.filter {
        runner = runner.with_filter(filter.clone());
    }

    runner = runner.with_fail_fast(args.fail_fast);

    // Discover or use provided test files
    let test_files = if let Some(file) = &args.file {
        if !file.exists() {
            color_eyre::eyre::bail!("Test file not found: {}", file.display());
        }
        vec![file.clone()]
    } else {
        let current_dir = args
            .working_dir
            .clone()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
        TestFileParser::discover(&current_dir)
    };

    if test_files.is_empty() {
        output::warning("No test files found. Create a roxid-test.yml file to define tests.");
        return Ok(());
    }

    output::status("Running", &format!("{} test file(s)", test_files.len()));

    let mut all_passed = true;
    let mut total_passed = 0usize;
    let mut total_failed = 0usize;
    let mut total_skipped = 0usize;

    for file in &test_files {
        output::dim(&format!("  {}", file.display()));

        match runner.run_file(file).await {
            Ok(suite_result) => {
                total_passed += suite_result.passed;
                total_failed += suite_result.failed;
                total_skipped += suite_result.skipped;

                if !suite_result.all_passed() {
                    all_passed = false;
                }

                let report = TestReporter::report(&suite_result, format);
                print!("{}", report);
            }
            Err(e) => {
                all_passed = false;
                total_failed += 1;
                output::error(&format!(
                    "Failed to run test file '{}': {:?}",
                    file.display(),
                    e
                ));
            }
        }
    }

    // Print summary
    println!();
    let total = total_passed + total_failed + total_skipped;
    if all_passed {
        output::success(&format!(
            "All {} tests passed ({} passed, {} skipped)",
            total, total_passed, total_skipped
        ));
    } else {
        output::failure(&format!(
            "{} of {} tests failed ({} passed, {} failed, {} skipped)",
            total_failed, total, total_passed, total_failed, total_skipped
        ));
    }

    if !all_passed {
        std::process::exit(1);
    }

    Ok(())
}
