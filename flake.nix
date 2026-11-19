{
  description = "Wordle — a terminal word game in Rust";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-26.05";

  outputs = { self, nixpkgs }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];
      forAllSystems = f:
        nixpkgs.lib.genAttrs systems
          (system: f nixpkgs.legacyPackages.${system});
    in
    {
      packages = forAllSystems (pkgs: rec {
        # Keep `version` in sync with Cargo.toml (calendar versioning).
        wordle = pkgs.rustPlatform.buildRustPackage {
          pname = "wordle";
          version = "2026.11.18";
          src = pkgs.lib.cleanSource self;

          # Word lists are include_str!-embedded at compile time, so the only
          # build inputs are the vendored crates plus a C compiler for the
          # bundled SQLite in rusqlite.
          cargoLock.lockFile = ./Cargo.lock;

          meta = with pkgs.lib; {
            description = "Terminal Wordle clone (3–8 letters) with persistent SQLite stats";
            homepage = "https://github.com/Jheickson/Wordle";
            license = licenses.mit;
            mainProgram = "wordle";
          };
        };
        default = wordle;
      });

      apps = forAllSystems (pkgs: {
        default = {
          type = "app";
          program = "${self.packages.${pkgs.system}.wordle}/bin/wordle";
        };
      });

      devShells = forAllSystems (pkgs: {
        default = pkgs.mkShell {
          packages = with pkgs; [
            cargo
            clippy
            rustfmt
            rust-analyzer
          ];
        };
      });
    };
}
