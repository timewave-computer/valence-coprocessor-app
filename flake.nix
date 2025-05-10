{
  description = "Valence coprocessor app";

  nixConfig.extra-experimental-features = "nix-command flakes";
  nixConfig.extra-substituters = "https://coffeetables.cachix.org";
  nixConfig.extra-trusted-public-keys = ''
    coffeetables.cachix.org-1:BCQXDtLGFVo/rTG/J4omlyP/jbtNtsZIKHBMTjAWt8g=
  '';

  inputs = {
    nixpkgs.url = "nixpkgs/nixos-24.11";
    
    # Add Rust overlay for better Rust support
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    flake-parts.url = "github:hercules-ci/flake-parts";

    devshell.url = "github:numtide/devshell";
  };

  outputs = {
    self,
    nixpkgs,
    rust-overlay,
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
      }: let
        # Add rust-overlay
        overlays = [ (import rust-overlay) ];
        pkgsWithOverlays = import nixpkgs {
          inherit system overlays;
        };
        
        # Create a Rust with WASM target (nightly)
        rustWithWasmTarget = pkgsWithOverlays.rust-bin.nightly.latest.default.override {
          targets = [ "wasm32-unknown-unknown" ];
          extensions = [ "rust-src" ];
        };
        
        # Create a stable Rust with WASM target (fallback)
        rustStableWithWasmTarget = pkgsWithOverlays.rust-bin.stable.latest.default.override {
          targets = [ "wasm32-unknown-unknown" ];
        };
      in {
        # Create packages for WASM building
        packages = {
          sp1-rust = pkgs.callPackage ./nix/sp1-rust.nix {};
          sp1 = pkgs.callPackage ./nix/sp1.nix {
            inherit (config.packages) sp1-rust;
          };
          
          # Build the WASM binary
          wasm-binary = pkgs.stdenv.mkDerivation {
            name = "valence-coprocessor-app-wasm";
            version = "0.1.0";
            
            src = ./.;
            
            buildInputs = [
              rustWithWasmTarget
              pkgs.wasm-bindgen-cli
            ];
            
            buildPhase = ''
              # Use the Rust with WASM target to build the WASM binary
              export HOME=$TMPDIR
              export RUSTFLAGS="--cfg=web_sys_unstable_apis"
              ${rustWithWasmTarget}/bin/cargo build --target wasm32-unknown-unknown --release -p valence-coprocessor-app-lib
            '';
            
            installPhase = ''
              mkdir -p $out
              cp target/wasm32-unknown-unknown/release/valence_coprocessor_app_lib.wasm $out/
            '';
          };
        };

        # Create a deployment script
        packages.deploy-script = pkgs.writeShellScriptBin "deploy-app" ''
          set -ex
          
          # Clean any previous build artifacts to avoid conflicts
          rm -rf target/wasm32-unknown-unknown/release/valence_coprocessor_app_lib.wasm || true
          
          # First build the WASM binary using the wasm-shell
          echo "Building WASM with nightly Rust toolchain..."
          nix develop .#wasm-shell -c bash -c 'export RUSTFLAGS="--cfg=web_sys_unstable_apis"; cargo build --target wasm32-unknown-unknown --release -p valence-coprocessor-app-lib'
          
          # Copy the WASM to the expected location
          mkdir -p target/wasm32-unknown-unknown/optimized
          cp target/wasm32-unknown-unknown/release/valence_coprocessor_app_lib.wasm target/wasm32-unknown-unknown/optimized/
          
          # Create a temp directory for deployment
          DEPLOY_DIR=$(mktemp -d)
          
          # Copy prebuilt SP1 binary to this directory
          if [ ! -d "$PRJ_ROOT/bin" ]; then
            mkdir -p "$PRJ_ROOT/bin"
          fi
          
          # Detect platform for the correct binary
          PLATFORM="$(uname -s)"
          ARCH="$(uname -m)"
          
          if [ "$PLATFORM" = "Darwin" ]; then
            if [ "$ARCH" = "arm64" ]; then
              PLATFORM_TARGET="aarch64-apple-darwin"
            else
              PLATFORM_TARGET="x86_64-apple-darwin"
            fi
          elif [ "$PLATFORM" = "Linux" ]; then
            if [ "$ARCH" = "aarch64" ] || [ "$ARCH" = "arm64" ]; then
              PLATFORM_TARGET="aarch64-unknown-linux-gnu"
            else
              PLATFORM_TARGET="x86_64-unknown-linux-gnu"
            fi
          else
            echo "Unsupported platform: $PLATFORM"
            exit 1
          fi
          
          # Install cargo-prove if needed
          if [ ! -f "$PRJ_ROOT/bin/cargo-prove" ]; then
            echo "Installing cargo-prove for $PLATFORM_TARGET"
            CARGO_PROVE_URL="https://github.com/succinctlabs/sp1/releases/latest/download/cargo-prove-$PLATFORM_TARGET"
            curl -L "$CARGO_PROVE_URL" -o "$PRJ_ROOT/bin/cargo-prove"
            chmod +x "$PRJ_ROOT/bin/cargo-prove"
          fi
          
          # Run direct deployment command
          echo "Deploying with cargo-prove..."
          "$PRJ_ROOT/bin/cargo-prove" prove deploy
        '';
        
        # SP1 development shell
        devshells.sp1-shell = {
          name = "sp1-shell";
          packages = [
            config.packages.sp1-rust
            config.packages.sp1
            pkgs.llvmPackages.clang
            pkgs.llvmPackages.llvm
            pkgs.curl
          ] ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
            pkgs.libiconv
            pkgs.darwin.apple_sdk.frameworks.Security
            pkgs.darwin.apple_sdk.frameworks.CoreFoundation
          ];
          
          # Add environment variables
          env = pkgs.lib.optionals pkgs.stdenv.isDarwin [
            {
              name = "LIBRARY_PATH";
              value = "${pkgs.libiconv}/lib";
            }
            {
              name = "RUSTFLAGS";
              value = "-L ${pkgs.libiconv}/lib";
            }
          ] ++ [
            {
              name = "PATH";
              prefix = "$PRJ_ROOT/bin";
            }
          ];
          
          # Define SP1 commands
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
            {
              name = "deploy";
              help = "Deploy the app (using the WASM binary)";
              command = "cargo run -- deploy";
            }
          ];
          
          # Add SP1 command aliases via bash.extra
          bash.extra = ''
            # Install SP1 tools locally
            if [ ! -d "$PRJ_ROOT/bin" ]; then
              mkdir -p "$PRJ_ROOT/bin"
            fi

            # Detect platform
            PLATFORM="$(uname -s)"
            ARCH="$(uname -m)"

            if [ "$PLATFORM" = "Darwin" ]; then
              if [ "$ARCH" = "arm64" ]; then
                PLATFORM_TARGET="aarch64-apple-darwin"
              else
                PLATFORM_TARGET="x86_64-apple-darwin"
              fi
            elif [ "$PLATFORM" = "Linux" ]; then
              if [ "$ARCH" = "aarch64" ] || [ "$ARCH" = "arm64" ]; then
                PLATFORM_TARGET="aarch64-unknown-linux-gnu"
              else
                PLATFORM_TARGET="x86_64-unknown-linux-gnu"
              fi
            else
              echo "Unsupported platform: $PLATFORM"
            fi

            # Install cargo-prove if not already installed
            if [ ! -f "$PRJ_ROOT/bin/cargo-prove" ]; then
              echo "Installing cargo-prove for $PLATFORM_TARGET"
              CARGO_PROVE_URL="https://github.com/succinctlabs/sp1/releases/latest/download/cargo-prove-$PLATFORM_TARGET"
              curl -L "$CARGO_PROVE_URL" -o "$PRJ_ROOT/bin/cargo-prove"
              chmod +x "$PRJ_ROOT/bin/cargo-prove"
            fi

            # Check for Succinct Rust toolchain
            RUST_INSTALLED=$(rustup toolchain list | grep -c succinct || true)
            if [ "$RUST_INSTALLED" -eq "0" ]; then
              echo "Installing Succinct Rust toolchain..."
              PATH="$PRJ_ROOT/bin:$PATH" "$PRJ_ROOT/bin/cargo-prove" prove install-toolchain
            fi
            
            # Set up environment for SP1
            export PATH="$PRJ_ROOT/bin:$PATH"
            export RUSTUP_TOOLCHAIN=succinct
          '';
        };
        
        # WASM development shell
        devshells.wasm-shell = {
          name = "wasm-shell";
          packages = [
            rustWithWasmTarget
            pkgs.wasm-bindgen-cli
          ] ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
            pkgs.libiconv
            pkgs.darwin.apple_sdk.frameworks.Security
            pkgs.darwin.apple_sdk.frameworks.CoreFoundation
          ];
          
          # Add environment variables - don't use RUSTFLAGS here
          env = pkgs.lib.optionals pkgs.stdenv.isDarwin [
            {
              name = "LIBRARY_PATH";
              value = "${pkgs.libiconv}/lib";
            }
          ];
          
          # Define WASM commands
          commands = [
            {
              name = "build-wasm";
              help = "Build WASM with nightly Rust toolchain";
              command = "cargo build --target wasm32-unknown-unknown --release -p valence-coprocessor-app-lib";
            }
          ];
          
          # Add nightly Rust setup
          bash.extra = ''
            # Set RUSTFLAGS properly
            if [ "$(uname -s)" = "Darwin" ]; then
              export RUSTFLAGS="--cfg=web_sys_unstable_apis -L ${pkgs.libiconv}/lib"
            else
              export RUSTFLAGS="--cfg=web_sys_unstable_apis"
            fi
          '';
        };
        
        # Default development shell points to our deployment script
        devshells.default = {
          packages = [
            config.packages.deploy-script
            pkgs.curl
          ];
          
          commands = [
            {
              name = "deploy";
              help = "Build WASM and deploy the app (automated)";
              command = "${config.packages.deploy-script}/bin/deploy-app";
            }
            {
              name = "sp1";
              help = "Enter the SP1 development shell";
              command = "nix develop .#sp1-shell";
            }
            {
              name = "wasm";
              help = "Enter the WASM development shell";
              command = "nix develop .#wasm-shell";
            }
          ];
          
          bash.extra = ''
            echo "Valence Coprocessor App Development Environment"
            echo "Use 'menu' to view available commands"
          '';
        };
      };
    });
}