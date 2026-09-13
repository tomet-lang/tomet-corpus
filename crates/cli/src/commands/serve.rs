use std::net::IpAddr;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Args;

#[derive(Args, Debug, Clone)]
pub struct ServeArgs {
    /// Source directory containing Tomet documents
    #[arg(default_value = ".")]
    pub dir: PathBuf,

    /// Output directory for static site
    #[arg(short, long)]
    pub dest: Option<PathBuf>,

    /// Host IP to bind the dev server to
    #[arg(long, default_value = "127.0.0.1")]
    pub host: IpAddr,

    /// Port to listen on
    #[arg(short, long, default_value_t = 3000)]
    pub port: u16,
}

pub async fn run_serve(args: ServeArgs) -> Result<()> {
    let config = tmtbook::BookConfig::load_from_dir(&args.dir)
        .with_context(|| format!("failed to load config from {:?}", args.dir))?;
    let src_dir = args.dir.canonicalize().context("failed to resolve source dir")?;
    let out_dir = args.dest.unwrap_or_else(|| src_dir.join(&config.book.dest));

    tmtbook::run_dev_server(src_dir, out_dir, config, args.host, args.port).await?;

    Ok(())
}
