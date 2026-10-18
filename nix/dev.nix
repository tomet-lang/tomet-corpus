{
  pkgs,
  mkShell,
  fenix,
  tools,
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
    tools
    just
    deno
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
