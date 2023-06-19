{
  description = "An interaction nets compiler";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, fenix, flake-utils, ... }:
    let
      inherit (nixpkgs.lib) genAttrs importTOML optionals cleanSource;
      inherit ((importTOML ./Cargo.toml).package) version;
    in
    flake-utils.lib.eachDefaultSystem (system:
      let
        fenixPkgs = fenix.packages.${system};
        pkgs = nixpkgs.legacyPackages.${system};

        toolchain = fenixPkgs.minimal.toolchain;
        rustPlatform = pkgs.makeRustPlatform {
          cargo = toolchain;
          rustc = toolchain;
        };
      in
      rec {
        packages.default = rustPlatform.buildRustPackage
          {
            pname = "zamuza";
            inherit version;

            src = cleanSource ./.;

            cargoLock.lockFile = ./Cargo.lock;

            nativeBuildInputs = [
              pkgs.installShellFiles
            ];

            buildInputs = optionals pkgs.stdenv.isDarwin [
              pkgs.darwin.apple_sdk.frameworks.CoreServices
            ];
          };

        checks.default = packages.default;

        devShells.default = pkgs.mkShell {
          packages = [
            (fenixPkgs.default.withComponents [
              "cargo"
              "clippy"
              "rustc"
              "rustfmt"
            ])
            fenixPkgs.rust-analyzer
            pkgs.gdb

            (rustPlatform.buildRustPackage
              rec {
                pname = "pest-ide-tools";
                version = "v0.3.2";

                src = pkgs.fetchFromGitHub {
                  owner = "pest-parser";
                  repo = pname;
                  rev = version;
                  hash = "sha256-hnTXxzp4k6CqSwLijD+hNmag0qDO1S2Pf1GdW0AfbzA=";
                };

                cargoLock.lockFile = src + "/Cargo.lock";

                nativeBuildInputs = [ pkgs.pkg-config pkgs.installShellFiles ];

                buildInputs = [
                  pkgs.openssl
                ] ++ optionals pkgs.stdenv.isDarwin [
                  pkgs.darwin.apple_sdk.frameworks.CoreServices
                ];
              })
          ];

          buildInputs = optionals pkgs.stdenv.isDarwin [
            pkgs.darwin.apple_sdk.frameworks.CoreServices
            pkgs.libiconv
          ];

          RUST_SRC_PATH = pkgs.rustPlatform.rustLibSrc;
        };
      });
}
