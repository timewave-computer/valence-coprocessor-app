{
  description = "A development shell for the Valence co-processor.";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      nixpkgs,
      rust-overlay,
      flake-utils,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };

        customRustToolchain = (
          pkgs.rust-bin.selectLatestNightlyWith (
            toolchain:
            toolchain.default.override {
              targets = [ "wasm32-unknown-unknown" ];
            }
          )
        );

      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = [
            customRustToolchain
          ];
        };
      }
    );
}
