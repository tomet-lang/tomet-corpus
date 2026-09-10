{
  pkgs,
  mkShell,
  tomet,
  tmtbook,
  ...
}:
mkShell rec {
  buildInputs = with pkgs; [
    tomet
    tmtbook
    pagefind
  ];

  shellHook = ''
    echo "🧪 tomet tmtbook"
  '';
}
