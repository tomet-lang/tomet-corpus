{
  description = "Tomet Ecosystem";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    treefmt-nix.url = "github:numtide/treefmt-nix";

    #[ Tool ]
    tomet.url = "github:tomet-lang/tomet";
    tmtbook = {
      url = "github:tomet-lang/tomet-book";
      inputs.tomet.follows = "tomet";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-parts.follows = "flake-parts";
      inputs.treefmt-nix.follows = "treefmt-nix";
    };
  };

  outputs = inputs: import ./nix inputs;
}
