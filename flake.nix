{
  description = "Tomet Ecosystem";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    treefmt-nix.url = "github:numtide/treefmt-nix";

    #[ Tool ]
    tomet.url = "github:tomet-lang/tomet";
    tomet-book = {
      url = "github:tomet-lang/tomet-book";
      inputs.tomet.follows = "tomet";
    };
    twrit = {
      url = "github:tomet-lang/tomet-writ";
      inputs.tomet.follows = "tomet";
    };
  };

  outputs = inputs: import ./nix inputs;
}
