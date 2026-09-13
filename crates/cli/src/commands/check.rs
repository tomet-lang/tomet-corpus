use std::path::PathBuf;

use anyhow::Result;
use clap::Args;

#[derive(Args, Debug, Clone)]
pub struct CheckArgs {
    /// Directory or file to check
    #[arg(short, long, default_value = "wikipedia/pages")]
    pub path: PathBuf,

    /// Suppress progress output
    #[arg(short, long, default_value_t = false)]
    pub quiet: bool,
}

pub fn run_check(args: &CheckArgs) -> Result<()> {
    if !args.quiet {
        println!("Checking tomet syntax for: {:?}", args.path);
    }
    wiki::validate_path(&args.path)?;
    if !args.quiet {
        println!("Validation passed successfully!");
    }
    Ok(())
}
