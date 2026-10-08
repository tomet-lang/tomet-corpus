mod commands;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "cli",
    version,
    about = "tomet-sandbox: Transform dataset collections into tomet (.tmt) books"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Wikipedia dataset operations
    Wiki(commands::wiki::WikiArgs),

    /// Aozora Bunko dataset operations
    Aozora(commands::aozora::AozoraArgs),

    /// Display dataset statistics (word count, categories, metadata distribution)
    Stats(commands::stats::StatsArgs),

    /// Check tomet (.tmt) syntax for a file or directory
    Check(commands::check::CheckArgs),

    /// Build the book into static HTML with tmtbook
    Build(commands::build::BuildArgs),

    /// Run local development preview server with tmtbook
    Serve(commands::serve::ServeArgs),
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Wiki(args) => commands::wiki::run_wiki(args).await?,
        Commands::Aozora(args) => commands::aozora::run_aozora(args).await?,
        Commands::Stats(args) => commands::stats::run_stats(&args)?,
        Commands::Check(args) => commands::check::run_check(&args)?,
        Commands::Build(args) => commands::build::run_build(args)?,
        Commands::Serve(args) => commands::serve::run_serve(args).await?,
    }

    Ok(())
}
