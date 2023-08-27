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
      in {
        packages.ggelo = craneLib.buildPackage {
          pname = "ggelo";
          version = "0.1.0";
          src = ./.;
          cargoBuildFlags = "-p cli";

          nativeBuildInputs = [ pkgs.pkg-config ];
          PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig";

          doCheck = true;
        };

        packages.default = self.packages.${system}.ggelo;
        checks.default = self.packages.${system}.ggelo;

        devShells.default = pkgs.mkShell {
          packages = with pkgs; [ rustc cargo pkg-config rust-analyzer ];
          PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig";
        };
    });
}
