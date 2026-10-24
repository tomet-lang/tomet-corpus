mod commands;

use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};
use sisho_sdk::{Capabilities, ExtensionInfo};

#[derive(Parser, Debug)]
#[command(
    name = "sisho-corpus",
    version,
    about = "Corpus conversion, benchmarking, and stress-testing toolchains for the Tomet ecosystem"
)]
struct Cli {
    /// Disclose extension metadata for sisho.
    #[arg(long = "sisho-extension-info", hide = true)]
    sisho_extension_info: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Wikipedia dataset operations
    Wiki(commands::wiki::WikiArgs),

    /// Aozora Bunko dataset operations
    Aozora(commands::aozora::AozoraArgs),

    /// Example vaults (pkm, app-dev) from the tomet-lang/examples repository
    Examples(commands::examples::ExamplesArgs),

    /// Display dataset statistics (word count, categories, metadata distribution)
    Stats(commands::stats::StatsArgs),

    /// Check tomet (.tmt) syntax for a file or directory
    Check(commands::check::CheckArgs),

    /// Build the book into static HTML with tomet-shelf
    Build(commands::build::BuildArgs),

    /// Run local development preview server with tomet-shelf
    Serve(commands::serve::ServeArgs),
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    if cli.sisho_extension_info {
        let info = ExtensionInfo::new(
            "corpus",
            env!("CARGO_PKG_VERSION"),
            "Corpus conversion, benchmarking, and stress-testing toolchains",
        )
        .with_capabilities(Capabilities::default());
        if let Err(error) = info.print() {
            eprintln!("sisho-corpus: failed to write extension info: {error}");
            std::process::exit(1);
        }
        return Ok(());
    }

    let Some(command) = cli.command else {
        Cli::command().print_help()?;
        println!();
        return Ok(());
    };

    match command {
        Commands::Wiki(args) => commands::wiki::run_wiki(args).await?,
        Commands::Aozora(args) => commands::aozora::run_aozora(args).await?,
        Commands::Examples(args) => commands::examples::run_examples(args)?,
        Commands::Stats(args) => commands::stats::run_stats(&args)?,
        Commands::Check(args) => commands::check::run_check(&args)?,
        Commands::Build(args) => commands::build::run_build(args)?,
        Commands::Serve(args) => commands::serve::run_serve(args).await?,
    }

    Ok(())
}
