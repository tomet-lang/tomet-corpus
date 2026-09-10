use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "cli",
    version,
    about = "tomet-sandbox: Transform dataset collections into tomet (.tmt)"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Wikipedia dataset operations
    Wiki(WikiArgs),
}

#[derive(clap::Args, Debug)]
struct WikiArgs {
    #[command(subcommand)]
    command: WikiCommands,
}

#[derive(Subcommand, Debug)]
enum WikiCommands {
    /// Download articles from Wikipedia API
    Download(wiki::DownloadArgs),

    /// Convert downloaded raw Wikipedia data to tomet (.tmt)
    Convert(wiki::ConvertArgs),
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Wiki(wiki_args) => match wiki_args.command {
            WikiCommands::Download(args) => wiki::run_download(args).await?,
            WikiCommands::Convert(args) => wiki::run_convert(args).await?,
        },
    }

    Ok(())
}
