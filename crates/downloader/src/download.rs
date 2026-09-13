use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, Result};
use clap::Args;
use indicatif::{ProgressBar, ProgressStyle};
use tokio::time::sleep;

use super::client::{MediaWikiClient, MediaWikiEndpoint};

#[derive(Args, Debug, Clone)]
pub struct DownloadArgs {
    /// Title(s) of specific articles to download
    #[arg(short, long)]
    pub title: Vec<String>,

    /// Number of random articles to download
    #[arg(short, long)]
    pub random: Option<usize>,

    /// Category name(s) to download articles from
    #[arg(short, long)]
    pub category: Vec<String>,

    /// Maximum articles to download when specifying a category
    #[arg(short, long, default_value_t = 50)]
    pub limit: usize,

    /// Language code (e.g., "ja", "en")
    #[arg(long, default_value = "ja")]
    pub lang: String,

    /// Target host ("wikipedia", "wiktionary", or custom URL)
    #[arg(long, default_value = "wikipedia")]
    pub host: String,

    /// Output directory for raw downloaded JSON files
    #[arg(short, long, default_value = "wikipedia/raw")]
    pub output_dir: PathBuf,

    /// Overwrite existing downloaded files
    #[arg(short, long, default_value_t = false)]
    pub force: bool,

    /// Delay between API requests in milliseconds
    #[arg(long, default_value_t = 300)]
    pub delay_ms: u64,
}

pub async fn run_download(args: DownloadArgs) -> Result<Vec<PathBuf>> {
    tokio::fs::create_dir_all(&args.output_dir)
        .await
        .with_context(|| format!("failed to create output dir: {:?}", args.output_dir))?;

    let endpoint = MediaWikiEndpoint::from_host_and_lang(&args.host, &args.lang);
    let client = MediaWikiClient::new(endpoint)?;
    let mut titles_to_fetch: Vec<String> = args.title.clone();

    // 1. Fetch category titles if requested
    for category in &args.category {
        println!("Fetching category members for '{}'...", category);
        let cat_titles = client.fetch_category_titles(category, args.limit).await?;
        println!("Found {} articles in category '{}'.", cat_titles.len(), category);
        titles_to_fetch.extend(cat_titles);
    }

    // 2. Fetch random titles if requested
    if let Some(count) = args.random {
        println!("Fetching {} random article titles...", count);
        let random_titles = client.fetch_random_titles(count).await?;
        println!("Found {} random titles.", random_titles.len());
        titles_to_fetch.extend(random_titles);
    }

    if titles_to_fetch.is_empty() {
        println!("No articles specified to download. Use --title, --random, or --category.");
        return Ok(Vec::new());
    }

    // Deduplicate titles
    titles_to_fetch.sort();
    titles_to_fetch.dedup();

    let total = titles_to_fetch.len();
    println!(
        "Downloading {} article(s) to {:?} [host: {}]",
        total,
        args.output_dir,
        client.endpoint().base_url()
    );

    let progress = ProgressBar::new(total as u64);
    progress.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:30.cyan/blue}] {pos}/{len} ({percent}%) {msg}")
            .unwrap()
            .progress_chars("#>-"),
    );

    let mut saved_files = Vec::new();
    let mut success_count = 0;
    let mut skipped_count = 0;
    let mut failed_count = 0;

    for (i, title) in titles_to_fetch.iter().enumerate() {
        let filename = sanitize_filename(title) + ".json";
        let target_path = args.output_dir.join(&filename);

        progress.set_message(format!("Fetching '{}'", title));

        if target_path.exists() && !args.force {
            progress.println(format!("  [skip] '{}' already exists", title));
            skipped_count += 1;
            saved_files.push(target_path);
            progress.inc(1);
            continue;
        }

        match client.fetch_page(title).await {
            Ok(page) => {
                let json_data = serde_json::to_string_pretty(&page)?;
                if let Err(e) = tokio::fs::write(&target_path, json_data).await {
                    progress.println(format!("  [error] Failed to save {:?}: {}", target_path, e));
                    failed_count += 1;
                } else {
                    progress.println(format!("  [saved] '{}' -> {:?}", title, target_path));
                    success_count += 1;
                    saved_files.push(target_path);
                }
            }
            Err(e) => {
                progress.println(format!("  [error] Failed to fetch '{}': {}", title, e));
                failed_count += 1;
            }
        }

        progress.inc(1);

        if args.delay_ms > 0 && i + 1 < total {
            sleep(Duration::from_millis(args.delay_ms)).await;
        }
    }

    progress.finish_with_message("Download complete");

    println!();
    println!(
        "Download complete: {} succeeded, {} skipped, {} failed (Total: {})",
        success_count, skipped_count, failed_count, total
    );

    Ok(saved_files)
}

pub use wiki::sanitize_filename;
