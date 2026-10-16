//! `cargo run -p tomet-synth --example gen -- <dir> <count> [wiki|prose] [seed] [target_bytes] [depth]`
//!
//! Writes a corpus to disk, e.g. to point `tomet-corpus check -p <dir>` or an
//! editor at it. `depth` sets both the heading and the list depth. Benchmarks
//! should call the library directly instead.

use std::path::PathBuf;
use std::process::exit;

use tomet_synth::{Config, Corpus};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (Some(dir), Some(count)) = (args.first(), args.get(1).and_then(|s| s.parse().ok())) else {
        eprintln!("usage: gen <dir> <count> [wiki|prose] [seed] [target_bytes] [depth]");
        exit(2);
    };
    let mut config = match args.get(2).map(String::as_str) {
        None | Some("wiki") => Config::wiki(),
        Some("prose") => Config::prose(),
        Some(other) => {
            eprintln!("unknown style: {other}");
            exit(2);
        }
    };
    let seed = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(0);
    if let Some(bytes) = args.get(4).and_then(|s| s.parse().ok()) {
        config = config.with_target_bytes(bytes);
    }
    if let Some(depth) = args.get(5).and_then(|s| s.parse().ok()) {
        config = config.with_heading_depth(depth).with_list_depth(depth);
    }
    let corpus = Corpus::new(seed, config.with_universe(count));
    match corpus.write_to(&PathBuf::from(dir), count) {
        Ok(bytes) => println!("wrote {count} documents ({bytes} bytes) to {dir}"),
        Err(e) => {
            eprintln!("error: {e}");
            exit(1);
        }
    }
}
