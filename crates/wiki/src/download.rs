use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, Result};
use clap::Args;
use tokio::time::sleep;

use super::client::WikiClient;

#[derive(Args, Debug, Clone)]
pub struct DownloadArgs {
    /// Title(s) of specific Wikipedia articles to download
    #[arg(short, long)]
    pub title: Vec<String>,

    /// Number of random articles to download
    #[arg(short, long)]
    pub random: Option<usize>,

    /// Category name to download articles from
    #[arg(short, long)]
    pub category: Option<String>,

    /// Maximum articles to download when specifying a category
    #[arg(short, long, default_value_t = 10)]
    pub limit: usize,

    /// Wikipedia language code (e.g., "ja", "en")
    #[arg(long, default_value = "ja")]
    pub lang: String,

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

pub async fn run_download(args: DownloadArgs) -> Result<()> {
    tokio::fs::create_dir_all(&args.output_dir)
        .await
        .with_context(|| format!("failed to create output dir: {:?}", args.output_dir))?;

    let client = WikiClient::new(&args.lang)?;
    let mut titles_to_fetch: Vec<String> = args.title.clone();

    // 1. Fetch category titles if requested
    if let Some(category) = &args.category {
        println!("Fetching category members for '{}'...", category);
        let cat_titles = client.fetch_category_titles(category, args.limit).await?;
        println!("Found {} articles in category.", cat_titles.len());
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
        println!("Example: cli wiki download --title '人工知能'");
        return Ok(());
    }

    // Deduplicate titles
    titles_to_fetch.sort();
    titles_to_fetch.dedup();

    println!(
        "Downloading {} article(s) to {:?}",
        titles_to_fetch.len(),
        args.output_dir
    );

    let total = titles_to_fetch.len();
    let mut success_count = 0;
    let mut skipped_count = 0;
    let mut failed_count = 0;

    for (i, title) in titles_to_fetch.iter().enumerate() {
        let filename = sanitize_filename(title) + ".json";
        let target_path = args.output_dir.join(&filename);

        if target_path.exists() && !args.force {
            println!(
                "[{}/{}] Skipping '{}' (already exists at {:?})",
                i + 1,
                total,
                title,
                target_path
            );
            skipped_count += 1;
            continue;
        }

        println!("[{}/{}] Fetching '{}'...", i + 1, total, title);
        match client.fetch_page(title).await {
            Ok(page) => {
                let json_data = serde_json::to_string_pretty(&page)?;
                if let Err(e) = tokio::fs::write(&target_path, json_data).await {
                    eprintln!("  Failed to save {:?}: {}", target_path, e);
                    failed_count += 1;
                } else {
                    println!("  Saved to {:?}", target_path);
                    success_count += 1;
                }
            }
            Err(e) => {
                eprintln!("  Failed to fetch '{}': {}", title, e);
                failed_count += 1;
            }
        }

        if args.delay_ms > 0 && i + 1 < total {
            sleep(Duration::from_millis(args.delay_ms)).await;
        }
    }

    println!();
    println!(
        "Download complete: {} succeeded, {} skipped, {} failed (Total: {})",
        success_count, skipped_count, failed_count, total
    );

    Ok(())
}

/// Sanitize filename for safe storage on all OS filesystems
pub fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            ' ' => '_',
            other => other,
        })
        .collect()
}
