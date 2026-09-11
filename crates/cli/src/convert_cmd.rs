use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::Args;
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

    println!(
        "Converting {} article(s) from {:?} to {:?}",
        files_to_convert.len(),
        args.input_dir,
        args.output_dir
    );

    let converter = WikiToTometConverter::new();
    let mut success_count = 0;
    let mut failed_count = 0;

    for input_path in files_to_convert {
        let filename = input_path.file_stem().unwrap().to_string_lossy();
        let target_path = args.output_dir.join(format!("{}.tmt", filename));

        if target_path.exists() && !args.force {
            println!("Skipping existing {:?}", target_path);
            continue;
        }

        match process_file(&converter, &input_path, &target_path).await {
            Ok(title) => {
                println!("Converted '{}' -> {:?}", title, target_path);
                success_count += 1;
            }
            Err(e) => {
                eprintln!("Failed to convert {:?}: {}", input_path, e);
                failed_count += 1;
            }
        }
    }

    println!();
    println!(
        "Conversion finished: {} succeeded, {} failed",
        success_count, failed_count
    );

    if args.validate && success_count > 0 {
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
