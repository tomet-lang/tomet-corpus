set shell := ["bash", "-cu"]

shelf := env_var_or_default("TOMET_SHELF", "tomet-shelf")

tree_out:
    tree -d -L 3

wiki-build:
    {{ shelf }} build

wiki-dev:
    {{ shelf }} serve

wiki-check:
    cargo run -q -- wiki check

wiki-check-strict:
    cargo run -q -- wiki check && {{ shelf }} build --strict

check target="data/wikipedia/pages":
    cargo run -q -- check -p {{ target }}

stats target="data/wikipedia/pages":
    cargo run -q -- stats -p {{ target }}

# Fetch the example vaults (pkm, app-dev) into data/examples. Pin with: just examples --rev <commit>
examples *args:
    cargo run -q -- examples sync {{ args }}

sync-random n="5":
    cargo run -q -- wiki sync --random {{ n }} --build

build:
    cargo run -q -- build --strict

serve:
    cargo run -q -- serve

test:
    cargo test

# Generate and update all derived documentation across the workspace.
docs:
    tomet export .
    tomet format -i .

# Check that all documents parse, format cleanly, and match export targets.
docs-check:
    tomet check .
    tomet format --check .
    tomet export --check .
    @if [ -f .writ.tmt ] && command -v twrit >/dev/null 2>&1; then \
        twrit check .; \
    fi
