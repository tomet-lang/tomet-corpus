use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use clap::{Args, Subcommand};

const DEFAULT_URL: &str = "https://github.com/tomet-lang/examples.git";

#[derive(Args, Debug)]
pub struct ExamplesArgs {
    #[command(subcommand)]
    pub command: ExamplesCommands,
}

#[derive(Subcommand, Debug)]
pub enum ExamplesCommands {
    /// Fetch (or update) the example vaults into a local, git-ignored directory
    Sync(SyncArgs),
}

#[derive(Args, Debug, Clone)]
pub struct SyncArgs {
    /// Git repository holding the example vaults
    #[arg(long, default_value = DEFAULT_URL)]
    pub url: String,

    /// Directory to clone into
    #[arg(short, long, default_value = "data/examples")]
    pub dest: PathBuf,

    /// Pin to a commit, tag or branch instead of following the default branch
    #[arg(long)]
    pub rev: Option<String>,
}

pub fn run_examples(args: ExamplesArgs) -> Result<()> {
    match args.command {
        ExamplesCommands::Sync(a) => run_sync(&a),
    }
}

fn run_sync(args: &SyncArgs) -> Result<()> {
    let dest = &args.dest;
    if dest.join(".git").exists() {
        match &args.rev {
            None => git(Some(dest), &["pull", "--ff-only", "--quiet"])?,
            Some(rev) => fetch_rev(dest, rev)?,
        }
    } else {
        if dest.exists() && dest.read_dir()?.next().is_some() {
            bail!("{dest:?} exists, is not empty and is not a git checkout");
        }
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let dest_str = dest.to_string_lossy();
        git(
            None,
            &["clone", "--quiet", "--depth", "1", &args.url, &dest_str],
        )?;
        if let Some(rev) = &args.rev {
            fetch_rev(dest, rev)?;
        }
    }

    let head = git_output(dest, &["rev-parse", "--short", "HEAD"])?;
    let files = count_tmt(dest)?;
    println!("examples at {head}: {files} .tmt file(s) in {dest:?}");
    Ok(())
}

/// Shallow-fetches `rev` and checks it out detached.
fn fetch_rev(dest: &Path, rev: &str) -> Result<()> {
    git(
        Some(dest),
        &["fetch", "--quiet", "--depth", "1", "origin", rev],
    )?;
    git(Some(dest), &["checkout", "--quiet", "FETCH_HEAD"])
}

fn git(dir: Option<&Path>, args: &[&str]) -> Result<()> {
    let mut cmd = Command::new("git");
    if let Some(dir) = dir {
        cmd.arg("-C").arg(dir);
    }
    let status = cmd
        .args(args)
        .status()
        .context("failed to run `git`; is it installed and on PATH?")?;
    if !status.success() {
        bail!("`git {}` failed", args.join(" "));
    }
    Ok(())
}

fn git_output(dir: &Path, args: &[&str]) -> Result<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .context("failed to run `git`")?;
    if !out.status.success() {
        bail!("`git {}` failed", args.join(" "));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn count_tmt(dir: &Path) -> Result<usize> {
    let mut n = 0;
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.file_name().is_some_and(|f| f == ".git") {
            continue;
        }
        if path.is_dir() {
            n += count_tmt(&path)?;
        } else if path.extension().is_some_and(|e| e == "tmt") {
            n += 1;
        }
    }
    Ok(n)
}
