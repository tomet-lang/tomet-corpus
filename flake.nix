{
  description = "Tomet Corpus - Conversion and corpus generation tools";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    treefmt-nix = {
      url = "github:numtide/treefmt-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    #= Rust
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane = {
      url = "github:ipetkov/crane";
    };

    #= Tool
    tomet = {
      url = "github:tomet-lang/tomet";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    twrit = {
      url = "github:tomet-lang/tomet-writ";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.tomet.follows = "tomet";
    };
    tomet-book = {
      url = "github:tomet-lang/tomet-book";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.tomet.follows = "tomet";
    };
  };

  outputs = inputs: import ./nix inputs;
}
