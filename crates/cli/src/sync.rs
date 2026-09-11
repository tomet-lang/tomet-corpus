use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Args;

#[derive(Args, Debug, Clone)]
pub struct SyncArgs {
    /// Title(s) of specific articles to sync
    #[arg(short, long)]
    pub title: Vec<String>,

    /// Number of random articles to sync
    #[arg(short, long)]
    pub random: Option<usize>,

    /// Category name to sync articles from
    #[arg(short, long)]
    pub category: Option<String>,

    /// Maximum articles when specifying a category
    #[arg(short, long, default_value_t = 10)]
    pub limit: usize,

    /// Language code (e.g., "ja", "en")
    #[arg(long, default_value = "ja")]
    pub lang: String,

    /// Target host ("wikipedia", "wiktionary", or custom URL)
    #[arg(long, default_value = "wikipedia")]
    pub host: String,

    /// Directory for raw downloaded JSON files
    #[arg(long, default_value = "wikipedia/raw")]
    pub raw_dir: PathBuf,

    /// Directory for converted .tmt files
    #[arg(long, default_value = "wikipedia/pages")]
    pub pages_dir: PathBuf,

    /// Overwrite existing downloaded/converted files
    #[arg(short, long, default_value_t = false)]
    pub force: bool,

    /// Validate syntax of converted .tmt files
    #[arg(long, default_value_t = true)]
    pub validate: bool,

    /// Automatically trigger tmtbook build after conversion
    #[arg(long, default_value_t = false)]
    pub build: bool,
}

pub async fn run_sync(args: SyncArgs) -> Result<()> {
    println!("=== Step 1: Downloading from {} ===", args.host);
    let dl_args = downloader::DownloadArgs {
        title: args.title.clone(),
        random: args.random,
        category: args.category.clone(),
        limit: args.limit,
        lang: args.lang.clone(),
        host: args.host.clone(),
        output_dir: args.raw_dir.clone(),
        force: args.force,
        delay_ms: 300,
    };

    let downloaded_files = downloader::run_download(dl_args).await?;
    if downloaded_files.is_empty() {
        println!("No new or existing files to convert.");
        return Ok(());
    }

    println!();
    println!("=== Step 2: Converting to tomet (.tmt) ===");
    let convert_args = crate::convert_cmd::ConvertArgs {
        input_dir: args.raw_dir.clone(),
        output_dir: args.pages_dir.clone(),
        title: args.title.clone(),
        force: args.force,
        validate: args.validate,
    };

    crate::convert_cmd::run_convert(convert_args).await?;

    if args.build {
        println!();
        println!("=== Step 3: Building book with tmtbook ===");
        let src_dir = std::env::current_dir()?;
        let config = tmtbook::BookConfig::load_from_dir(&src_dir)
            .context("Failed to load tmtbook.toml configuration")?;
        let out_dir = src_dir.join(&config.book.dest);

        let report = tmtbook::build_book(&src_dir, &out_dir, &config)?;
        if !report.failures.is_empty() {
            anyhow::bail!("{} document(s) failed during tmtbook build", report.failures.len());
        }
        println!("tmtbook build completed successfully!");
    }

    println!();
    println!("Pipeline completed successfully!");
    Ok(())
}
