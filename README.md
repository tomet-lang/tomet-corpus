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

