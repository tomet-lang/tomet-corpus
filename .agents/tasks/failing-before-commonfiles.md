# Checks that fail on `main` (found while taking commonfiles, 2026-10-24)

Both fail the same way on `main` before and after the common files; neither is from them.

- [ ] `tomet check .` (tomet 0.5.0): 8 of 8 documents fail. `tmtroot/readme.tmt` and the others write `@config` / `@meta` where the
  heads are now declarations: write `::config` / `::meta`. `${macro.generated_by(self.path)}` is also unknown (`macro.` is not a function
  any more). `sisho check .` stops at this step.
- [ ] `nix build .#checks.x86_64-linux.treefmt`: statix wants `inherit (sisho) tomet;` in `nix/default.nix` (the `treefmt = import ./formatter.nix`
  call) instead of `tomet = sisho.tomet;`. Run `nix fmt`.
