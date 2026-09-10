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
        tomet = inputs.tomet.packages.${pkgs.stdenv.hostPlatform.system}.default;
        tmtbook = inputs.tmtbook.packages.${pkgs.stdenv.hostPlatform.system}.default;
      };

      treefmt = import ./formatter.nix {
        tomet = inputs.tomet.packages.${pkgs.stdenv.hostPlatform.system}.default;
      };
    };
}
