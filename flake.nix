{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = inputs@{ self, nixpkgs, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let pkgs = import nixpkgs { inherit system; };
      in {
        packages.ggelo = pkgs.rustPlatform.buildRustPackage {
          pname = "ggelo";
          version = "0.1.0";
          src = ./.;
          cargoBuildFlags = "-p cli";
          cargoLock.lockFile = ./Cargo.lock;
        };

        packages.default = self.packages.${system}.ggelo;

        devShells.default = pkgs.mkShell {
          inputsFrom = [ self.packages.${system}.ggelo ];
          packages = [ pkgs.rust-analyzer ];
        };
    });
}
