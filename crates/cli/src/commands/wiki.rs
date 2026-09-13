use anyhow::Result;
use clap::{Args, Subcommand};

#[derive(Args, Debug)]
pub struct WikiArgs {
    #[command(subcommand)]
    pub command: WikiCommands,
}

#[derive(Subcommand, Debug)]
pub enum WikiCommands {
    /// Download articles from Wikipedia API
    Download(super::download::DownloadArgs),

    /// Convert downloaded raw Wikipedia data to tomet (.tmt)
    Convert(super::convert::ConvertArgs),

    /// One-stop pipeline: Download -> Convert -> Validate (-> Build)
    Sync(super::sync::SyncArgs),

    /// Check tomet (.tmt) syntax for converted articles
    Check(super::check::CheckArgs),

    /// Display statistics and categories for converted Wikipedia pages
    Stats(super::stats::StatsArgs),
}

pub async fn run_wiki(args: WikiArgs) -> Result<()> {
    match args.command {
        WikiCommands::Download(a) => {
            super::download::run_download(a).await?;
        }
        WikiCommands::Convert(a) => {
            super::convert::run_convert(a).await?;
        }
        WikiCommands::Sync(a) => {
            super::sync::run_sync(a).await?;
        }
        WikiCommands::Check(a) => {
            super::check::run_check(&a)?;
        }
        WikiCommands::Stats(a) => {
            super::stats::run_stats(&a)?;
        }
    }
    Ok(())
}
