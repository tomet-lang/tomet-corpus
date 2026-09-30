{
  pkgs,
  mkShell,
  fenix,
  tomet,
  tomet-lsp,
  tmtbook,
  twrit,
  ...
}:
let
  rustToolchain = fenix.combine [
    (fenix.stable.withComponents [
      "cargo"
      "clippy"
      "rustc"
      "rust-src"
    ])
  ];
in
mkShell rec {
  buildInputs = with pkgs; [
    #= Develop
    tomet
    tomet-lsp
    tmtbook
    twrit
    just
    pagefind

    #= Build
    pkg-config
    esbuild
    tailwindcss_4

    #= Rust
    rustToolchain
    cargo-edit
    cargo-outdated
    cargo-nextest

    #= Runtime
    openssl
  ];

  shellHook = ''
    echo "🧪 tomet-corpus"
  '';
}
