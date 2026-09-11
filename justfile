set shell := ["bash", "-cu"]

tmtbook := env_var_or_default("TMTBOOK", "tmtbook")

tree_out:
    tree -d -L 3

wiki-build:
    {{ tmtbook }} build

wiki-dev:
    {{ tmtbook }} serve

wiki-check:
    cargo run -q -- wiki check

wiki-check-strict:
    cargo run -q -- wiki check && {{ tmtbook }} build --strict

check target="wikipedia/pages":
    cargo run -q -- check -p {{ target }}

stats target="wikipedia/pages":
    cargo run -q -- stats -p {{ target }}

sync-random n="5":
    cargo run -q -- wiki sync --random {{ n }} --build

build:
    cargo run -q -- build --strict

serve:
    cargo run -q -- serve

test:
    cargo test
