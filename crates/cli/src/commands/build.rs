use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Args;

#[derive(Args, Debug, Clone)]
pub struct BuildArgs {
    /// Source directory containing Tomet documents
    #[arg(default_value = ".")]
    pub dir: PathBuf,

    /// Output directory for static site
    #[arg(short, long)]
    pub dest: Option<PathBuf>,

    /// Fail the build if any document could not be rendered
    #[arg(long, default_value_t = false)]
    pub strict: bool,
}

pub fn run_build(args: BuildArgs) -> Result<()> {
    let config = tmtbook::BookConfig::load_from_dir(&args.dir)
        .with_context(|| format!("failed to load config from {:?}", args.dir))?;
    let src_dir = args.dir.canonicalize().context("failed to resolve source dir")?;
    let out_dir = args.dest.unwrap_or_else(|| src_dir.join(&config.book.dest));

    let report = tmtbook::build_book(&src_dir, &out_dir, &config, false)?;
    if args.strict && !report.failures.is_empty() {
        for failure in &report.failures {
            eprintln!("  {}: {}", failure.rel_path, failure.error);
        }
        anyhow::bail!("{} document(s) failed to build (--strict)", report.failures.len());
    }

    Ok(())
}
