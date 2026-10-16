<!-- Generated from tmtroot/readme.tmt. Edit that, then `tomet export .`. -->

# Tomet Corpus

Corpus conversion, benchmarking, and stress-testing toolchains for the Tomet ecosystem.

## Overview

`tomet-corpus` provides conversion toolchains that transform large-scale external text sources into valid Tomet (`.tmt`) documents. It serves as a real-world testing ground for syntax verification, parser performance benchmarking, and vocabulary validation.

Supported corpus sources include:

- **Wikipedia**: MediaWiki / wikitext dumps conversion.
- **Wiktionary**: Dictionary and lexical data extraction.
- **Aozora Bunko**: Japanese literary works with ruby annotations and typesetting markers.

## Workspace Layout

- `app` -- CLI toolchain for sync, build, check, and stats.
- `crates/tomet-convert-wikitext` -- Converts MediaWiki / wikitext to Tomet.
- `crates/tomet-convert-aozora` -- Converts Aozora Bunko formatting to Tomet.
- `crates/downloader` -- Fetches and caches remote corpora datasets.
- `crates/tomet-synth` -- Deterministic synthetic corpus generator for benchmarks and stress tests (dependency-free; consumers use it as a dev-dependency).
- `data` -- Corpus data. `data/handwritten` is tracked; `data/wikipedia`, `data/aozora` (generated) and `data/examples` (fetched with `tomet-corpus examples sync`, or `just examples`) are local and git-ignored.

## Building and Running

### Build workspace

```bash
cargo build --workspace
```

### Run tests

```bash
cargo test --workspace
```

### Convert corpus

```bash
cargo run -p tomet-corpus-cli -- sync
```

## Synthetic Corpora

`tomet-synth` generates documents of any size or count without committing them: the same `(seed, index, config)` always yields byte-identical text. It is meant to be used as a dev-dependency by the benchmarks in the consumer repositories.

```rust
use tomet_synth::{Config, Corpus};

let corpus = Corpus::new(42, Config::wiki().with_universe(10_000));
let doc = corpus.doc(7);               // one document
corpus.write_to(dir, 10_000)?;         // or a sharded directory tree
```

To inspect a corpus by hand:

```bash
cargo run -p tomet-synth --example gen -- /tmp/synth 1000 wiki
```

