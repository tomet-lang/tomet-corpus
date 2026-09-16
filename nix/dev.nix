{
  pkgs,
  mkShell,
  tomet,
  tmtbook,
  twrit,
  ...
}:
mkShell rec {
  buildInputs = with pkgs; [
    tomet
    tmtbook
    twrit
    pagefind
  ];

  shellHook = ''
    echo "🧪 tomet tmtbook"
  '';
}
