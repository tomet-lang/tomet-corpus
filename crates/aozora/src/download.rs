use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, Result};
use clap::Args;
use indicatif::{ProgressBar, ProgressStyle};
use tokio::time::sleep;

use crate::catalog::AozoraCatalog;
use crate::client::AozoraClient;
use crate::model::AozoraCatalogEntry;

#[derive(Args, Debug, Clone)]
pub struct DownloadArgs {
    /// Title(s) of specific works to download (exact or partial match)
    #[arg(short, long)]
    pub title: Vec<String>,

    /// Author name(s) to download works from
    #[arg(short, long)]
    pub author: Vec<String>,

    /// Work ID(s) (作品ID) to download
    #[arg(long)]
    pub id: Vec<u64>,

    /// Number of random works to download
    #[arg(short, long)]
    pub random: Option<usize>,

    /// Maximum works to download when matching by author or title search
    #[arg(short, long, default_value_t = 10)]
    pub limit: usize,

    /// Output directory for raw downloaded JSON files
    #[arg(short, long, default_value = "aozora/raw")]
    pub output_dir: PathBuf,

    /// Directory to cache the Aozora Bunko master catalog CSV
    #[arg(long, default_value = "aozora/cache")]
    pub cache_dir: PathBuf,

    /// Force re-downloading the catalog CSV
    #[arg(long, default_value_t = false)]
    pub refresh_catalog: bool,

    /// Overwrite existing downloaded files
    #[arg(short, long, default_value_t = false)]
    pub force: bool,

    /// Delay between downloads in milliseconds
    #[arg(long, default_value_t = 300)]
    pub delay_ms: u64,
}

pub async fn run_download(args: DownloadArgs) -> Result<Vec<PathBuf>> {
    tokio::fs::create_dir_all(&args.output_dir)
        .await
        .with_context(|| format!("failed to create output dir: {:?}", args.output_dir))?;

    let client = AozoraClient::new()?;

    // 1. Load or fetch master catalog
    let catalog = AozoraCatalog::load_or_fetch(
        client.http_client(),
        &args.cache_dir,
        args.refresh_catalog,
    )
    .await?;

    // 2. Resolve entries to download
    let mut targets: Vec<&AozoraCatalogEntry> = Vec::new();

    // Work IDs
    for &id in &args.id {
        if let Some(entry) = catalog.find_by_id(id) {
            targets.push(entry);
        } else {
            eprintln!("Warning: No text found for work ID {}", id);
        }
    }

    // Titles
    for title in &args.title {
        let matches = catalog.find_by_title(title);
        if matches.is_empty() {
            eprintln!("Warning: No works found matching title '{}'", title);
        } else {
            for m in matches.into_iter().take(args.limit) {
                targets.push(m);
            }
        }
    }

    // Authors
    for author in &args.author {
        let matches = catalog.find_by_author(author);
        if matches.is_empty() {
            eprintln!("Warning: No works found for author '{}'", author);
        } else {
            for m in matches.into_iter().take(args.limit) {
                targets.push(m);
            }
        }
    }

    // Random
    if let Some(count) = args.random {
        let random_entries = catalog.find_random(count);
        println!("Picked {} random works.", random_entries.len());
        targets.extend(random_entries);
    }

    if targets.is_empty() {
        println!("No works specified to download. Use --title, --author, --id, or --random.");
        return Ok(Vec::new());
    }

    // Deduplicate by book ID
    targets.sort_by_key(|e| e.book_id.clone());
    targets.dedup_by_key(|e| e.book_id.clone());

    let total = targets.len();
    println!(
        "Downloading {} Aozora Bunko work(s) to {:?}",
        total, args.output_dir
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

    for (i, entry) in targets.iter().enumerate() {
        let filename = sanitize_aozora_filename(&entry.author(), &entry.title) + ".json";
        let target_path = args.output_dir.join(&filename);

        progress.set_message(format!("Fetching '{}' ({})", entry.title, entry.author()));

        if target_path.exists() && !args.force {
            progress.println(format!("  [skip] '{}' already exists", filename));
            skipped_count += 1;
            saved_files.push(target_path);
            progress.inc(1);
            continue;
        }

        match client.fetch_book(entry).await {
            Ok(book) => {
                let json_data = serde_json::to_string_pretty(&book)?;
                if let Err(e) = tokio::fs::write(&target_path, json_data).await {
                    progress.println(format!("  [error] Failed to save {:?}: {}", target_path, e));
                    failed_count += 1;
                } else {
                    progress.println(format!("  [saved] {} - {} -> {:?}", book.author, book.title, target_path));
                    success_count += 1;
                    saved_files.push(target_path);
                }
            }
            Err(e) => {
                progress.println(format!("  [error] Failed to fetch '{}': {}", entry.title, e));
                failed_count += 1;
            }
        }

        progress.inc(1);

        if args.delay_ms > 0 && i + 1 < total {
            sleep(Duration::from_millis(args.delay_ms)).await;
        }
    }

    progress.finish_with_message("Aozora download complete");

    println!();
    println!(
        "Download finished: {} succeeded, {} skipped, {} failed (Total: {})",
        success_count, skipped_count, failed_count, total
    );

    Ok(saved_files)
}

/// Sanitize author and title into a safe filename
pub fn sanitize_aozora_filename(author: &str, title: &str) -> String {
    let combined = if author.is_empty() {
        title.to_string()
    } else {
        format!("{}_{}", author, title)
    };

    combined
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | ' ' | '\t' | '\n' => '_',
            _ => c,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(
            sanitize_aozora_filename("太宰治", "走れメロス"),
            "太宰治_走れメロス"
        );
        assert_eq!(
            sanitize_aozora_filename("Author/Name", "Title: Subtitle?"),
            "Author_Name_Title__Subtitle_"
        );
    }
}

