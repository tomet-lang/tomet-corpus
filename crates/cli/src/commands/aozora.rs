use anyhow::Result;
use clap::{Args, Subcommand};

#[derive(Args, Debug)]
pub struct AozoraArgs {
    #[command(subcommand)]
    pub command: AozoraCommands,
}

#[derive(Subcommand, Debug)]
pub enum AozoraCommands {
    /// Download works from Aozora Bunko
    Download(aozora::DownloadArgs),

    /// Convert downloaded raw Aozora Bunko JSON data to tomet (.tmt)
    Convert(aozora::ConvertArgs),
}

pub async fn run_aozora(args: AozoraArgs) -> Result<()> {
    match args.command {
        AozoraCommands::Download(a) => {
            aozora::run_download(a).await?;
        }
        AozoraCommands::Convert(a) => {
            aozora::run_convert(a).await?;
        }
    }
    Ok(())
}
