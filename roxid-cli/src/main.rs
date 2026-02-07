use color_eyre::Result;

mod commands;
mod output;

use clap::{Parser, Subcommand};

/// Roxid - Azure DevOps Pipeline Emulator
///
/// Run, test, and validate Azure DevOps pipelines locally.
/// Launch the interactive TUI with no arguments or the 'tui' subcommand.
#[derive(Parser, Debug)]
#[command(name = "roxid", version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Run an Azure DevOps pipeline locally
    Run(commands::run::RunArgs),

    /// Run pipeline tests
    Test(commands::test::TestArgs),

    /// Validate a pipeline YAML file
    Validate(commands::validate::ValidateArgs),

    /// Launch the interactive TUI
    Tui,

    /// Manage the Azure DevOps task cache
    Task(commands::task::TaskArgs),
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let cli = Cli::parse();

    match cli.command {
        // No subcommand = launch TUI (same as `roxid tui`)
        None | Some(Commands::Tui) => roxid_tui::run().await,

        Some(Commands::Run(args)) => commands::run::execute(args).await,

        Some(Commands::Test(args)) => commands::test::execute(args).await,

        Some(Commands::Validate(args)) => commands::validate::execute(args),

        Some(Commands::Task(args)) => commands::task::execute(args).await,
    }
}
