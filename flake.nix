{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    naersk = {
      url = "github:nix-community/naersk";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    utils = { url = "github:numtide/flake-utils"; };
  };

  outputs = { self, nixpkgs, naersk, fenix, utils, }:
    utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
        toolchain = fenix.packages.${system}.complete.withComponents [
          "cargo"
          "clippy"
          "rust-analyzer"
          "rust-docs"
          "rust-std"
          "rustc"
          "rustfmt"
        ];
        naersk' = (pkgs.callPackage naersk {}).override {
          cargo = toolchain;
          rustc = toolchain;
        };
        required-pkgs = with pkgs; [
          pkg-config fontconfig
        ];

        power-graphing = naersk'.buildPackage {
          src = ./.;
          nativeBuildInputs = required-pkgs;
        };
      in {
        packages = {
          inherit power-graphing;
          default = power-graphing;
        };

        devShell = pkgs.mkShell {
          packages = required-pkgs ++ [ toolchain ];
        };
      });
}
