{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";

    crane.url = "github:ipetkov/crane";
    crane.inputs = {
      nixpkgs.follows = "nixpkgs";
      flake-utils.follows = "flake-utils";
    };
  };

  outputs = { self, nixpkgs, crane, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let pkgs = import nixpkgs { inherit system; };
          craneLib = crane.lib.${system};

          commonArgs = {
            version = "0.1.0";
            src = craneLib.path ./.;
            cargoBuildFlags = "-p cli";
            buildInputs = [ pkgs.openssl ];
            nativeBuildInputs = [ pkgs.pkg-config ];
          };

          # Cargo build dependencies/artifacts only
          # This derivation exists so that multiple builds can reuse
          # the same build artifacts
          cargoArtifacts = craneLib.buildDepsOnly (commonArgs // {
            pname = "ggelo-deps";
          });

          # Run clippy (and deny all warnings) on the crate source
          runClippy = craneLib.cargoClippy (commonArgs // {
            pname = "ggelo-clippy-check";
            cargoClippyExtraArgs = "--all-targets -- --deny warnings";
            inherit cargoArtifacts;
          });

          ggelo = craneLib.buildPackage (commonArgs // {
            pname = "ggelo";
            inherit cargoArtifacts;
          });
      in {
        packages.ggelo = ggelo;
        packages.default = ggelo;

        checks.build = ggelo;
        checks.runClippy = runClippy;

        devShells.default = pkgs.mkShell {
          packages = with pkgs; [ rustc cargo pkg-config rust-analyzer ];
          PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig";
        };
    });
}
