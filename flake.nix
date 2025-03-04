{
  description = "A dummy data generator for a key-value store";

  nixConfig.bash-prompt = "[nix]λ ";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    flake-utils = {
      url = "github:numtide/flake-utils";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, flake-utils, nixpkgs, fenix }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };

        rust-toolchain = fenix.packages.${system}.stable.withComponents [
          "cargo"
          "clippy"
          "rust-src"
          "rustc-dev"
          "rustfmt"
        ];

      in rec {
        # `nix build`
        packages.dummy-data-gen = pkgs.buildRustPackage {
          pname = "dummy-data-gen";
          version = "0.1.0";

          src = ./.;

          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          nativeBuildInputs = [ rust-toolchain ];
        };

        defaultPackage = packages.dummy-data-gen;

        # `nix run`
        apps.app = flake-utils.lib.mkApp {
          drv = packages.dummy-data-gen;
          name = "app";
        };

        defaultApp = apps.app;

        # `nix run .#watch`
        apps.watch = flake-utils.lib.mkApp {
          drv = pkgs.writeShellApplication {
            name = "watch";
            runtimeInputs = [
              pkgs.cargo-watch
              pkgs.gcc
              rust-toolchain
              pkgs.cargo-edit
              pkgs.rust-analyzer
              pkgs.openssl
              pkgs.zlib
              pkgs.rdkafka
              pkgs.cyrus_sasl
            ];
            text = ''
              cargo watch -w "./src/" -w "Cargo.lock" -w "Cargo.toml" -x "run"
            '';
          };
        };

        # `nix develop`
        devShell = pkgs.mkShell {
          buildInputs = [
            pkgs.cargo-edit
            pkgs.cargo-watch
            pkgs.rust-analyzer
            rust-toolchain
            pkgs.openssl
            pkgs.zlib
            pkgs.rdkafka
            pkgs.cyrus_sasl
          ];

          shellHook = ''
            export CARGO_INCREMENTAL=1
            export OPENSSL_NO_VENDOR=1
          '';
        };
      }
    );
}
