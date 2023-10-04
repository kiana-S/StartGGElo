{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";

    rust-overlay.url = "github:oxalica/rust-overlay";
    rust-overlay.inputs = {
      nixpkgs.follows = "nixpkgs";
      flake-utils.follows = "flake-utils";
    };

    crane.url = "github:ipetkov/crane";
    crane.inputs = {
      nixpkgs.follows = "nixpkgs";
      flake-utils.follows = "flake-utils";
      rust-overlay.follows = "rust-overlay";
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay, crane, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let pkgs = import nixpkgs {
            inherit system;
            overlays = [ rust-overlay.overlays.default ];
          };
          rustToolchain = pkgs.rust-bin.selectLatestNightlyWith (toolchain: toolchain.default);
          craneLib = crane.lib.${system}.overrideToolchain rustToolchain;

          commonArgs = {
            pname = "startrnr";
            version = "0.1.0";
            src = craneLib.path ./.;
            buildInputs = [ pkgs.openssl pkgs.sqlite ];
            nativeBuildInputs = [ pkgs.pkg-config ];
          };

          # Cargo build dependencies/artifacts only
          # This derivation exists so that multiple builds can reuse
          # the same build artifacts
          cargoArtifacts = craneLib.buildDepsOnly commonArgs;

          # Run clippy (and deny all warnings) on the crate source
          runClippy = craneLib.cargoClippy (commonArgs // {
            cargoClippyExtraArgs = "--all-targets -- --deny warnings";
            inherit cargoArtifacts;
          });

          startrnr = craneLib.buildPackage (commonArgs // {
            inherit cargoArtifacts;
          });
      in {
        packages.startrnr = startrnr;
        packages.default = startrnr;

        checks.build = startrnr;
        checks.runClippy = runClippy;

        devShells.default = pkgs.mkShell {
          packages = with pkgs; [ rustToolchain pkg-config rust-analyzer ];
          PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig";
        };
    });
}
