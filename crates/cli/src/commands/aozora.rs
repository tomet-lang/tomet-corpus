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
    Download(tomet_aozora::DownloadArgs),

    /// Convert downloaded raw Aozora Bunko JSON data to tomet (.tmt)
    Convert(tomet_aozora::ConvertArgs),
}

pub async fn run_aozora(args: AozoraArgs) -> Result<()> {
    match args.command {
        AozoraCommands::Download(a) => {
            tomet_aozora::run_download(a).await?;
        }
        AozoraCommands::Convert(a) => {
            tomet_aozora::run_convert(a).await?;
        }
    }
    Ok(())
}
