set shell := ["bash", "-cu"]

tmtbook := env_var_or_default("TMTBOOK", "tmtbook")

tree_out:
    tree -d -L 3

wiki-build:
    {{ tmtbook }} build

wiki-dev:
    {{ tmtbook }} serve
