//! Deterministic synthetic `.tmt` corpus generator, for benchmarks and stress
//! tests that need inputs at a chosen scale without committing them.
//!
//! - Same `(seed, index, Config)` => byte-identical document, on every
//!   platform and release. Documents are independent: `doc(i)` never replays
//!   `0..i`, so a million-document corpus can be generated lazily or in
//!   parallel.
//! - Dependency-free, and it must stay that way: consumers (tomet, tomet-shelf,
//!   immermemo) depend on this crate, so depending on any of them back would be
//!   a cycle. The output is plain text; whether it parses is checked on the
//!   consumer's side.
//!
//! ```
//! use tomet_synth::{Config, Corpus};
//!
//! let corpus = Corpus::new(42, Config::wiki().with_universe(100));
//! let doc = corpus.doc(7);
//! assert!(doc.text.starts_with("@meta{"));
//! assert_eq!(doc.text, corpus.doc(7).text);
//! ```

mod doc;
mod rng;
mod vocab;

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub use doc::{BlockWeights, Config, Style};

/// A generated document and the file name it should be stored under.
#[derive(Debug, Clone)]
pub struct Doc {
    pub index: usize,
    pub name: String,
    pub text: String,
}

/// Documents per subdirectory in [`Corpus::write_to`]. Keeps a million-file
/// corpus from becoming a single directory.
pub const FILES_PER_DIR: usize = 1000;

#[derive(Debug, Clone)]
pub struct Corpus {
    seed: u64,
    config: Config,
}

impl Corpus {
    pub fn new(seed: u64, config: Config) -> Self {
        Corpus { seed, config }
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    /// The document at `index`.
    pub fn doc(&self, index: usize) -> Doc {
        Doc {
            index,
            name: file_name(index),
            text: doc::render(index, self.seed, &self.config),
        }
    }

    /// Documents `0..count`, generated lazily.
    pub fn iter(&self, count: usize) -> impl Iterator<Item = Doc> + '_ {
        (0..count).map(|i| self.doc(i))
    }

    /// Relative path of document `index` under a corpus directory:
    /// `<index / FILES_PER_DIR>/<name>`.
    pub fn rel_path(index: usize) -> PathBuf {
        Path::new(&format!("{:05}", index / FILES_PER_DIR)).join(file_name(index))
    }

    /// Writes documents `0..count` under `dir` (created if missing) and
    /// returns the total bytes written.
    pub fn write_to(&self, dir: &Path, count: usize) -> io::Result<u64> {
        let mut bytes = 0u64;
        for doc in self.iter(count) {
            let path = dir.join(Self::rel_path(doc.index));
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&path, &doc.text)?;
            bytes += doc.text.len() as u64;
        }
        Ok(bytes)
    }
}

/// Title of document `index`. Independent of the seed, so a link written into
/// one document can name any other document of the same corpus.
pub fn title_of(index: usize) -> String {
    let n = vocab::NOUNS.len();
    format!(
        "{}{}_{index}",
        vocab::NOUNS[index % n],
        vocab::NOUNS[(index / n) % n]
    )
}

fn file_name(index: usize) -> String {
    format!("doc-{index:07}.tmt")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_inputs_same_bytes() {
        let a = Corpus::new(1, Config::wiki()).doc(5);
        let b = Corpus::new(1, Config::wiki()).doc(5);
        assert_eq!(a.text, b.text);
    }

    #[test]
    fn seed_and_index_both_change_output() {
        let c = Corpus::new(1, Config::wiki());
        assert_ne!(c.doc(5).text, c.doc(6).text);
        assert_ne!(c.doc(5).text, Corpus::new(2, Config::wiki()).doc(5).text);
    }

    /// FNV-1a over the whole document, so the RNG stream and the grammar of
    /// the output are both pinned, not just the front matter.
    fn fnv(text: &str) -> u64 {
        text.bytes().fold(0xcbf2_9ce4_8422_2325, |h, b| {
            (h ^ b as u64).wrapping_mul(0x0100_0000_01b3)
        })
    }

    #[test]
    fn output_is_pinned() {
        // The stream must never change silently: benchmark baselines depend
        // on it. If this fails on purpose, every recorded baseline is void.
        let wiki = Corpus::new(0, Config::wiki().with_target_bytes(2_000)).doc(0);
        let prose = Corpus::new(0, Config::prose().with_target_bytes(2_000)).doc(0);
        assert_eq!(
            (fnv(&wiki.text), fnv(&prose.text)),
            (16894126183888804619, 18438898717660632380)
        );
    }

    #[test]
    fn reaches_target_without_wild_overshoot() {
        for style in [Config::wiki(), Config::prose()] {
            for target in [1_000, 30_000, 200_000] {
                let t = Corpus::new(3, style.clone().with_target_bytes(target))
                    .doc(1)
                    .text
                    .len();
                assert!(t >= target, "{t} < {target}");
                assert!(t < target + 6_000, "{t} overshoots {target}");
            }
        }
    }

    #[test]
    fn links_stay_inside_the_universe() {
        let universe = 20;
        let c = Corpus::new(9, Config::wiki().with_universe(universe));
        let titles: Vec<String> = (0..universe).map(title_of).collect();
        let text = c.doc(2).text;
        let mut links = 0;
        for part in text.split("@link(ref:\"").skip(1) {
            let target = part.split('"').next().unwrap();
            assert!(titles.iter().any(|t| t == target), "dangling: {target}");
            links += 1;
        }
        assert!(links > 0, "expected some links");
    }

    #[test]
    fn heading_depth_one_has_no_subheadings() {
        let c = Corpus::new(4, Config::wiki().with_heading_depth(1));
        assert!(!c.doc(0).text.lines().any(|l| l.starts_with("== ")));
    }

    #[test]
    fn write_to_shards_directories() {
        let dir = std::env::temp_dir().join(format!("tomet-synth-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let c = Corpus::new(1, Config::wiki().with_target_bytes(500));
        let bytes = c.write_to(&dir, FILES_PER_DIR + 2).unwrap();
        assert!(bytes > 0);
        assert!(dir.join("00000/doc-0000000.tmt").is_file());
        assert!(dir.join("00001/doc-0001001.tmt").is_file());
        fs::remove_dir_all(&dir).unwrap();
    }
}
