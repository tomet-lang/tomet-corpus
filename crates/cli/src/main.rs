mod convert_cmd;
mod stats;
mod sync;

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "cli",
    version,
    about = "tomet-sandbox: Transform dataset collections into tomet (.tmt) books"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Wikipedia dataset operations
    Wiki(WikiArgs),

    /// Display dataset statistics (word count, categories, metadata distribution)
    Stats(stats::StatsArgs),

    /// Check tomet (.tmt) syntax for a file or directory
    Check(CheckArgs),

    /// Build the book into static HTML with tmtbook
    Build(BuildArgs),

    /// Run local development preview server with tmtbook
    Serve(ServeArgs),
}

#[derive(Args, Debug)]
struct WikiArgs {
    #[command(subcommand)]
    command: WikiCommands,
}

#[derive(Subcommand, Debug)]
enum WikiCommands {
    /// Download articles from Wikipedia API
    Download(downloader::DownloadArgs),

    /// Convert downloaded raw Wikipedia data to tomet (.tmt)
    Convert(convert_cmd::ConvertArgs),

    /// One-stop pipeline: Download -> Convert -> Validate (-> Build)
    Sync(sync::SyncArgs),

    /// Check tomet (.tmt) syntax for converted articles
    Check(CheckArgs),

    /// Display statistics and categories for converted Wikipedia pages
    Stats(stats::StatsArgs),
}

#[derive(Args, Debug, Clone)]
pub struct CheckArgs {
    /// Directory or file to check
    #[arg(short, long, default_value = "wikipedia/pages")]
    pub path: PathBuf,

    /// Suppress progress output
    #[arg(short, long, default_value_t = false)]
    pub quiet: bool,
}

#[derive(Args, Debug, Clone)]
struct BuildArgs {
    /// Source directory containing Tomet documents
    #[arg(default_value = ".")]
    dir: PathBuf,

    /// Output directory for static site
    #[arg(short, long)]
    dest: Option<PathBuf>,

    /// Fail the build if any document could not be rendered
    #[arg(long, default_value_t = false)]
    strict: bool,
}

#[derive(Args, Debug, Clone)]
struct ServeArgs {
    /// Source directory containing Tomet documents
    #[arg(default_value = ".")]
    dir: PathBuf,

    /// Output directory for static site
    #[arg(short, long)]
    dest: Option<PathBuf>,

    /// Host IP to bind the dev server to
    #[arg(long, default_value = "127.0.0.1")]
    host: std::net::IpAddr,

    /// Port to listen on
    #[arg(short, long, default_value_t = 3000)]
    port: u16,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Wiki(wiki_args) => match wiki_args.command {
            WikiCommands::Download(args) => {
                downloader::run_download(args).await?;
            }
            WikiCommands::Convert(args) => {
                convert_cmd::run_convert(args).await?;
            }
            WikiCommands::Sync(args) => {
                sync::run_sync(args).await?;
            }
            WikiCommands::Check(args) => {
                run_check_cmd(&args)?;
            }
            WikiCommands::Stats(args) => {
                stats::run_stats(&args)?;
            }
        },
        Commands::Stats(args) => {
            stats::run_stats(&args)?;
        }
        Commands::Check(args) => {
            run_check_cmd(&args)?;
        }
        Commands::Build(args) => {
            let config = tmtbook::BookConfig::load_from_dir(&args.dir)
                .with_context(|| format!("failed to load config from {:?}", args.dir))?;
            let src_dir = args.dir.canonicalize().context("failed to resolve source dir")?;
            let out_dir = args.dest.unwrap_or_else(|| src_dir.join(&config.book.dest));

            let report = tmtbook::build_book(&src_dir, &out_dir, &config)?;
            if args.strict && !report.failures.is_empty() {
                for failure in &report.failures {
                    eprintln!("  {}: {}", failure.rel_path, failure.error);
                }
                anyhow::bail!("{} document(s) failed to build (--strict)", report.failures.len());
            }
        }
        Commands::Serve(args) => {
            let config = tmtbook::BookConfig::load_from_dir(&args.dir)
                .with_context(|| format!("failed to load config from {:?}", args.dir))?;
            let src_dir = args.dir.canonicalize().context("failed to resolve source dir")?;
            let out_dir = args.dest.unwrap_or_else(|| src_dir.join(&config.book.dest));

            tmtbook::run_dev_server(src_dir, out_dir, config, args.host, args.port).await?;
        }
    }

    Ok(())
}

fn run_check_cmd(args: &CheckArgs) -> Result<()> {
    if !args.quiet {
        println!("Checking tomet syntax for: {:?}", args.path);
    }
    wiki::validate_path(&args.path)?;
    if !args.quiet {
        println!("Validation passed successfully!");
    }
    Ok(())
}
