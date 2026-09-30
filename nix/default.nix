{
  flake-parts,
  ...
}@inputs:
flake-parts.lib.mkFlake { inherit inputs; } {
  systems = [
    "x86_64-linux"
    "aarch64-linux"
    "aarch64-darwin"
  ];
  imports = [
    inputs.treefmt-nix.flakeModule
  ];

  perSystem =
    { pkgs, ... }:
    {
      devShells.default = pkgs.callPackage ./dev.nix {
        inherit inputs;
        fenix = inputs.fenix.packages.${pkgs.stdenv.hostPlatform.system};

        tomet = inputs.tomet.packages.${pkgs.stdenv.hostPlatform.system}.tomet;
        tomet-lsp = inputs.tomet.packages.${pkgs.stdenv.hostPlatform.system}.tomet-lsp;
        tmtbook = inputs.tomet-book.packages.${pkgs.stdenv.hostPlatform.system}.tmtbook;
        twrit = inputs.twrit.packages.${pkgs.stdenv.hostPlatform.system}.twrit;
      };

      treefmt = import ./formatter.nix {
        tomet = inputs.tomet.packages.${pkgs.stdenv.hostPlatform.system}.tomet;
      };
    };
}
