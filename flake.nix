{
  description = "Valence coprocessor app";

  nixConfig.extra-experimental-features = "nix-command flakes";
  nixConfig.extra-substituters = "https://coffeetables.cachix.org";
  nixConfig.extra-trusted-public-keys = ''
    coffeetables.cachix.org-1:BCQXDtLGFVo/rTG/J4omlyP/jbtNtsZIKHBMTjAWt8g=
  '';

  inputs = {
    nixpkgs.url = "nixpkgs/nixos-24.11";

    flake-parts.url = "github:hercules-ci/flake-parts";

    devshell.url = "github:numtide/devshell";
  };

  outputs = {
    self,
    flake-parts,
    ...
  } @ inputs:
    flake-parts.lib.mkFlake {inherit inputs;} ({moduleWithSystem, ...}: {
      imports = [
        inputs.devshell.flakeModule
      ];

      systems = ["x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin"];

      perSystem = {
        config,
        self',
        inputs',
        pkgs,
        system,
        ...
      }: {
        devshells.default = {
          packages = [
            config.packages.sp1-rust
            config.packages.sp1
            pkgs.llvmPackages.clang
            pkgs.llvmPackages.llvm
          ];
          
          # Define SP1 commands for the menu
          commands = [
            {
              name = "sp1";
              help = "Run cargo-prove";
              command = "cargo prove $@";
            }
            {
              name = "sp1-new";
              help = "Create a new SP1 project";
              command = "cargo prove new $@";
            }
            {
              name = "sp1-build";
              help = "Build an SP1 program";
              command = "cargo prove build $@";
            }
            {
              name = "sp1-vkey";
              help = "View verification key hash";
              command = "cargo prove vkey $@";
            }
          ];
          
          # Add SP1 command aliases via bash.extra
          bash.extra = ''
            # Automatically set up wasm32 target
            # Check if wasm-bindgen-cli needs to be installed
            if ! cargo install --list | grep -q wasm-bindgen-cli; then
              echo "Installing wasm-bindgen-cli..."
              cargo install wasm-bindgen-cli &>/dev/null && echo "wasm-bindgen-cli installed successfully"
            fi
            
            # Check if rustup is available
            if command -v rustup >/dev/null 2>&1; then
              # Check if target is already installed
              if ! rustup target list --installed | grep -q "wasm32-unknown-unknown"; then
                echo "Adding wasm32-unknown-unknown target..."
                rustup target add wasm32-unknown-unknown &>/dev/null && echo "wasm32 target added successfully"
              fi
            else
              echo "rustup not found. To complete setup, install rustup and run: rustup target add wasm32-unknown-unknown"
            fi
          '';
        };
        packages = {
          sp1-rust = pkgs.callPackage ./nix/sp1-rust.nix {};
          sp1 = pkgs.callPackage ./nix/sp1.nix {
            inherit (config.packages) sp1-rust;
          };
        };
      };
    });
}