{ tomet, ... }:
{
  projectRootFile = "flake.nix";
  programs = {
    #= Nix
    nixfmt.enable = true;
    statix.enable = true;
    deadnix.enable = true;
    #= Shell
    shfmt.enable = true;
    shellcheck.enable = true;

    #= Main
    rustfmt.enable = true; # Rust
    taplo.enable = true; # Toml
  };
  settings = {
    global.excludes = [ ]; # https://github.com/numtide/treefmt-nix/issues/171

    formatter = {
      tomet = {
        command = "${tomet}/bin/tomet";
        options = [
          "format"
          "-i"
        ];
        includes = [ "*.tmt" ];
      };
    };

    shfmt = {
      includes = [ "*.sh" ];
    };

    rustfmt = {
      includes = [ "*.rs" ];
    };
  };
}
