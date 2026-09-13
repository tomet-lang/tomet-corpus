use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::Args;

#[derive(Args, Debug, Clone)]
pub struct StatsArgs {
    /// Directory containing .tmt files to analyze
    #[arg(short, long, default_value = "wikipedia/pages")]
    pub path: PathBuf,
}

pub fn run_stats(args: &StatsArgs) -> Result<()> {
    if !args.path.exists() {
        anyhow::bail!("Path does not exist: {:?}", args.path);
    }

    let mut files = Vec::new();
    collect_tmt_files(&args.path, &mut files)?;

    if files.is_empty() {
        println!("No .tmt files found in {:?}", args.path);
        return Ok(());
    }

    let mut total_lines = 0;
    let mut total_chars = 0;
    let mut total_bytes: u64 = 0;
    let mut category_counts: HashMap<String, usize> = HashMap::new();
    let mut type_counts: HashMap<String, usize> = HashMap::new();

    for file in &files {
        let content = std::fs::read_to_string(file)
            .with_context(|| format!("failed to read {:?}", file))?;

        total_lines += content.lines().count();
        total_chars += content.chars().count();
        total_bytes += content.len() as u64;

        parse_meta_stats(&content, &mut category_counts, &mut type_counts);
    }

    let file_count = files.len();
    let avg_lines = total_lines / file_count;
    let avg_chars = total_chars / file_count;

    println!("==================================================");
    println!("  Dataset Statistics: {:?}", args.path);
    println!("==================================================");
    println!("  Total Documents : {}", file_count);
    println!("  Total Lines     : {} (avg {} lines/doc)", total_lines, avg_lines);
    println!("  Total Characters: {} (avg {} chars/doc)", total_chars, avg_chars);
    println!("  Total Size      : {:.2} KB", total_bytes as f64 / 1024.0);
    println!();

    // Top Categories
    let mut sorted_categories: Vec<(&String, &usize)> = category_counts.iter().collect();
    sorted_categories.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));

    println!("Top Categories ({} unique):", category_counts.len());
    if sorted_categories.is_empty() {
        println!("  (No categories extracted)");
    } else {
        for (cat, count) in sorted_categories.iter().take(15) {
            let bar_len = (*count * 20) / file_count.max(1);
            let bar = "#".repeat(bar_len.max(1));
            println!("  - {:<24} : {:>3} [{}]", cat, count, bar);
        }
    }
    println!();

    // Type distribution
    let mut sorted_types: Vec<(&String, &usize)> = type_counts.iter().collect();
    sorted_types.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));

    println!("Document Types ({} unique):", type_counts.len());
    if sorted_types.is_empty() {
        println!("  (No infobox types found)");
    } else {
        for (typ, count) in sorted_types.iter().take(10) {
            println!("  - {:<24} : {:>3}", typ, count);
        }
    }
    println!("==================================================");

    Ok(())
}

fn collect_tmt_files(dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    if dir.is_file() {
        if dir.extension().and_then(|s| s.to_str()) == Some("tmt") {
            files.push(dir.to_path_buf());
        }
        return Ok(());
    }

    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_tmt_files(&path, files)?;
        } else if path.extension().and_then(|s| s.to_str()) == Some("tmt") {
            files.push(path);
        }
    }
    Ok(())
}

fn parse_meta_stats(
    content: &str,
    category_counts: &mut HashMap<String, usize>,
    type_counts: &mut HashMap<String, usize>,
) {
    let mut in_meta = false;
    let mut in_categories = false;

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("@meta{") {
            in_meta = true;
            continue;
        }

        if in_meta {
            if trimmed.starts_with('}') && !in_categories {
                break;
            }

            if trimmed.starts_with("categories: [") {
                in_categories = true;
                continue;
            }

            if in_categories {
                if trimmed.starts_with(']') {
                    in_categories = false;
                    continue;
                }
                let cat = trimmed.trim_matches('"').trim_end_matches(',').trim_matches('"');
                if !cat.is_empty() {
                    *category_counts.entry(cat.to_string()).or_insert(0) += 1;
                }
                continue;
            }

            if trimmed.starts_with("type:") {
                let typ_val = trimmed
                    .strip_prefix("type:")
                    .unwrap_or("")
                    .trim()
                    .trim_matches('"')
                    .trim_end_matches(',')
                    .trim_matches('"');
                if !typ_val.is_empty() {
                    *type_counts.entry(typ_val.to_string()).or_insert(0) += 1;
                }
            }
        }
    }
}
