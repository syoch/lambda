{
  description = "Rust Lambda Calculus Project";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      rust-overlay,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [
            "rust-src"
            "rust-analyzer"
          ];
        };
      in
      {
        packages = {
          vsix = pkgs.stdenv.mkDerivation {
            name = "lambda-lang-support.vsix";
            src = ./vscode-extension;

            nativeBuildInputs = with pkgs; [
              nodejs_25
            ];

            buildPhase = ''
              npm install
              npm run compile
            '';

            installPhase = ''
              mkdir -p $out
              npx @vscode/vsce package --out $out/lambda-lang-support.vsix
            '';

            phases = [
              "unpackPhase"
              "buildPhase"
              "installPhase"
            ];
          };

          default = self.packages.${system}.vsix;
        };

        apps.build-vsix = {
          type = "app";
          program = toString (
            pkgs.writeShellScript "build-vsix" ''
              set -e
              cd vscode-extension
              npm install
              npm run compile
              npx @vscode/vsce package --out ./lambda-lang-support.vsix
              echo "✓ VSIX created: ./lambda-lang-support.vsix"
            ''
          );
        };

        apps.default = self.apps.${system}.build-vsix;

        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustToolchain
            cargo
            rustc
            rustfmt
            clippy
            perf
            clang-tools
            clang

            nodejs_25
            esbuild
          ];

          shellHook = ''
            echo "Rust Lambda Calculus Development Environment"
            echo "Rust version: $(rustc --version)"
          '';
        };
      }
    );
}
