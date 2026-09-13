use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};
use clap::Args;
use indicatif::{ProgressBar, ProgressStyle};
use tokio::sync::Semaphore;
use tokio::task::JoinSet;
use wiki::{sanitize_filename, WikiPage, WikiToTometConverter};

#[derive(Args, Debug, Clone)]
pub struct ConvertArgs {
    /// Directory containing raw downloaded JSON files
    #[arg(short, long, default_value = "wikipedia/raw")]
    pub input_dir: PathBuf,

    /// Output directory for .tmt files
    #[arg(short, long, default_value = "wikipedia/pages")]
    pub output_dir: PathBuf,

    /// Specific article title(s) to convert
    #[arg(short, long)]
    pub title: Vec<String>,

    /// Overwrite existing converted files
    #[arg(short, long, default_value_t = false)]
    pub force: bool,

    /// Validate converted files using tomet check
    #[arg(long, default_value_t = true)]
    pub validate: bool,

    /// Max concurrent conversion tasks (defaults to CPU thread count)
    #[arg(short = 'j', long)]
    pub concurrency: Option<usize>,
}

enum ConvertOutcome {
    Converted(String),
    Skipped(String),
    Failed(PathBuf, String),
}

pub async fn run_convert(args: ConvertArgs) -> Result<()> {
    tokio::fs::create_dir_all(&args.output_dir)
        .await
        .with_context(|| format!("failed to create output dir: {:?}", args.output_dir))?;

    let mut files_to_convert: Vec<PathBuf> = Vec::new();

    if !args.title.is_empty() {
        for t in &args.title {
            let filename = sanitize_filename(t) + ".json";
            let path = args.input_dir.join(filename);
            if path.exists() {
                files_to_convert.push(path);
            } else {
                eprintln!("Warning: input file not found: {:?}", path);
            }
        }
    } else {
        let mut entries = tokio::fs::read_dir(&args.input_dir).await.with_context(|| {
            format!("failed to read input directory {:?}", args.input_dir)
        })?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("json") {
                files_to_convert.push(path);
            }
        }
    }

    if files_to_convert.is_empty() {
        println!("No JSON files found in {:?}", args.input_dir);
        return Ok(());
    }

    let total = files_to_convert.len();
    let concurrency = args
        .concurrency
        .unwrap_or_else(|| std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4))
        .max(1);

    println!(
        "Converting {} article(s) from {:?} to {:?} (concurrency: {})",
        total, args.input_dir, args.output_dir, concurrency
    );

    let progress = ProgressBar::new(total as u64);
    progress.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:30.cyan/blue}] {pos}/{len} ({percent}%) {msg}")
            .unwrap()
            .progress_chars("#>-"),
    );

    let converter = Arc::new(WikiToTometConverter::new());
    let semaphore = Arc::new(Semaphore::new(concurrency));
    let mut join_set = JoinSet::new();

    for input_path in files_to_convert {
        let filename = input_path.file_stem().unwrap().to_string_lossy().to_string();
        let target_path = args.output_dir.join(format!("{}.tmt", filename));
        let force = args.force;
        let conv = Arc::clone(&converter);
        let sem = Arc::clone(&semaphore);

        join_set.spawn(async move {
            if target_path.exists() && !force {
                return ConvertOutcome::Skipped(filename);
            }

            let _permit = sem.acquire_owned().await.expect("semaphore closed");
            match process_file(&conv, &input_path, &target_path).await {
                Ok(title) => ConvertOutcome::Converted(title),
                Err(e) => ConvertOutcome::Failed(input_path, e.to_string()),
            }
        });
    }

    let mut success_count = 0;
    let mut skipped_count = 0;
    let mut failed_articles: Vec<(PathBuf, String)> = Vec::new();

    while let Some(res) = join_set.join_next().await {
        match res {
            Ok(ConvertOutcome::Converted(title)) => {
                progress.println(format!("  [converted] {}", title));
                success_count += 1;
            }
            Ok(ConvertOutcome::Skipped(title)) => {
                progress.println(format!("  [skipped] {}", title));
                skipped_count += 1;
            }
            Ok(ConvertOutcome::Failed(path, err)) => {
                progress.println(format!("  [error] {:?}: {}", path.file_name().unwrap_or_default(), err));
                failed_articles.push((path, err));
            }
            Err(join_err) => {
                eprintln!("Task join error: {}", join_err);
            }
        }
        progress.inc(1);
    }

    progress.finish_with_message("Conversion complete");

    let failed_count = failed_articles.len();
    println!();
    println!(
        "Conversion finished: {} succeeded, {} skipped, {} failed (Total: {})",
        success_count, skipped_count, failed_count, total
    );

    if !failed_articles.is_empty() {
        eprintln!("\nFailed conversions:");
        for (path, err) in &failed_articles {
            eprintln!("  - {:?}: {}", path, err);
        }
    }

    if args.validate && (success_count > 0 || (skipped_count > 0 && failed_count == 0)) {
        println!();
        println!("Checking tomet syntax for: {:?}", args.output_dir);
        wiki::validate_path(&args.output_dir)?;
        println!("Validation passed successfully!");
    }

    Ok(())
}

async fn process_file(
    converter: &WikiToTometConverter,
    input_path: &Path,
    target_path: &Path,
) -> Result<String> {
    let content = tokio::fs::read_to_string(input_path)
        .await
        .with_context(|| format!("failed to read {:?}", input_path))?;

    let page: WikiPage = serde_json::from_str(&content)
        .with_context(|| format!("failed to parse JSON from {:?}", input_path))?;

    let tmt_output = converter.convert(&page);

    tokio::fs::write(target_path, tmt_output)
        .await
        .with_context(|| format!("failed to write {:?}", target_path))?;

    Ok(page.title)
}
