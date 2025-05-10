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

        # Inline implementation of sp1-rust.nix
        sp1-rust = pkgs.stdenv.mkDerivation rec {
          name = "sp1-rust";
          version = "1.82.0";

          dontStrip = true;

          nativeBuildInputs = [
            pkgs.stdenv.cc.cc.lib
            pkgs.zlib
          ] ++ (if pkgs.stdenv.isDarwin then [ pkgs.fixDarwinDylibNames ] else [ pkgs.autoPatchelfHook ]);

          installPhase = ''
            runHook preInstall
            mkdir -p $out
            cp -r ./* $out/
            runHook postInstall
          '';

          src = let
            fetchGitHubReleaseAsset =
              {
                owner,
                repo,
                tag,
                asset,
                hash,
              }:
              pkgs.fetchzip {
                url = "https://github.com/${owner}/${repo}/releases/download/${tag}/${asset}";
                inherit hash;
                stripRoot = false;
              };
          in fetchGitHubReleaseAsset ({
            owner = "succinctlabs";
            repo = "rust";
            tag = "succinct-${version}";
          } // ({
            "x86_64-linux" = {
              asset = "rust-toolchain-x86_64-unknown-linux-gnu.tar.gz";
              hash = "sha256-wXI2zVwfrVk28CR8PLq4xyepdlu65uamzt/+jER2M2k=";
            };
            "aarch64-linux" = {
              asset = "rust-toolchain-aarch64-unknown-linux-gnu.tar.gz";
              hash = "sha256-92P392Afp8wEhiLOo+l9KJtwMAcKtK0GxZchXGg3U54=";
            };
            "x86_64-darwin" = {
              asset = "rust-toolchain-x86_64-apple-darwin.tar.gz";
              hash = "sha256-sPQW8eo+qItsmgK1uxRh1r73DBLUXUtmtVUvjacGzp0=";
            };
            "aarch64-darwin" = {
              asset = "rust-toolchain-aarch64-apple-darwin.tar.gz";
              hash = "sha256-R4D7hj2DcZ3vfCejvXwJ68YDOlgWGDPcb08GZNXz1Cg=";
            };
          }.${pkgs.stdenv.system}));
        };

        # Inline implementation of sp1.nix
        sp1 = pkgs.rustPlatform.buildRustPackage {
          pname = "sp1";
          version = "unstable-2025-03-06";

          nativeBuildInputs = [
            sp1-rust
            pkgs.pkg-config
            pkgs.openssl
          ];
          
          # Only build the sp1-cli package
          cargoBuildFlags = [ "--package sp1-cli" ];
          cargoHash = "sha256-gI/N381IfIWnF4tfXM1eKLI93eCjEELg/a5gWQn/3EA=";

          src = pkgs.fetchFromGitHub {
            owner = "succinctlabs";
            repo = "sp1";
            rev = "9f202bf603b3cab5b7c9db0e8cf5524a3428fbee";
            hash = "sha256-RpllsIlrGyYw6dInN0tTs7K1y4FiFmrxFSyt3/Xelkg=";
            fetchSubmodules = true;
          };
          
          doCheck = false;
        };
      in {
        # Create packages for WASM building
        packages = {
          # Use the inlined sp1-rust and sp1 instead of callPackage
          inherit sp1-rust sp1;
          
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

          # Script to install cargo-prove
          install-cargo-prove = pkgs.writeShellScriptBin "install-cargo-prove" ''
            #!/usr/bin/env bash
            # This script downloads the cargo-prove binary for the current platform

            set -e

            PRJ_ROOT=$(pwd)
            PLATFORM="$(uname -s)"
            ARCH="$(uname -m)"

            # Create the bin directory if it doesn't exist
            mkdir -p "$PRJ_ROOT/bin"

            # Define the version to use
            SP1_VERSION="v4.2.0"

            # Determine the correct archive name based on platform and architecture
            if [ "$PLATFORM" = "Darwin" ]; then
              if [ "$ARCH" = "arm64" ]; then
                PLATFORM_TARGET="darwin_arm64"
              else
                PLATFORM_TARGET="darwin_amd64"
              fi
            elif [ "$PLATFORM" = "Linux" ]; then
              if [ "$ARCH" = "aarch64" ]; then
                PLATFORM_TARGET="linux_arm64"
              else
                PLATFORM_TARGET="linux_amd64"
              fi
            else
              echo "Unsupported platform: $PLATFORM"
              exit 1
            fi

            ARCHIVE_NAME="cargo_prove_$SP1_VERSION""_$PLATFORM_TARGET.tar.gz"
            DOWNLOAD_URL="https://github.com/succinctlabs/sp1/releases/download/$SP1_VERSION/$ARCHIVE_NAME"

            echo "Installing cargo-prove for $PLATFORM_TARGET"
            echo "Downloading from: $DOWNLOAD_URL"

            # Create a temporary directory for extraction
            TMP_DIR=$(mktemp -d)
            trap 'rm -rf "$TMP_DIR"' EXIT

            # Download the archive
            curl -L "$DOWNLOAD_URL" -o "$TMP_DIR/$ARCHIVE_NAME" --progress-bar

            # Extract the archive
            tar -xzf "$TMP_DIR/$ARCHIVE_NAME" -C "$TMP_DIR"

            # Copy the binary to our bin directory
            cp "$TMP_DIR/cargo-prove" "$PRJ_ROOT/bin/"

            # Make it executable
            chmod +x "$PRJ_ROOT/bin/cargo-prove"

            # Verify that it works
            echo "Testing cargo-prove:"
            "$PRJ_ROOT/bin/cargo-prove" prove --version || "$PRJ_ROOT/bin/cargo-prove"

            echo "cargo-prove has been successfully installed to $PRJ_ROOT/bin/cargo-prove"
          '';

          # Build WASM script
          build-wasm = pkgs.writeShellScriptBin "build-wasm" ''
            #!/usr/bin/env bash
            # Script to build the WASM binary for Valence coprocessor

            set -e

            PRJ_ROOT=$(pwd)

            # Ensure proper directory structure
            mkdir -p "$PRJ_ROOT/bin"
            mkdir -p "$PRJ_ROOT/target/wasm32-unknown-unknown/release" 
            mkdir -p "$PRJ_ROOT/target/wasm32-unknown-unknown/optimized"

            # Step 1: Install cargo-prove if needed
            if [ ! -f "$PRJ_ROOT/bin/cargo-prove" ]; then
              echo "Installing cargo-prove..."
              ${config.packages.install-cargo-prove}/bin/install-cargo-prove
            fi

            # Step 2: Build the WASM binary using the nix wasm-shell
            echo "Building WASM with nightly Rust toolchain..."
            nix develop .#wasm-shell -c bash -c 'export RUSTFLAGS="--cfg=web_sys_unstable_apis"; cargo build --target wasm32-unknown-unknown --release -p valence-coprocessor-app-lib'

            # Copy the WASM to the expected location if it was built
            if [ -f "$PRJ_ROOT/target/wasm32-unknown-unknown/release/valence_coprocessor_app_lib.wasm" ]; then
              echo "Copying WASM binary to optimized directory..."
              cp "$PRJ_ROOT/target/wasm32-unknown-unknown/release/valence_coprocessor_app_lib.wasm" "$PRJ_ROOT/target/wasm32-unknown-unknown/optimized/"
            else
              echo "WASM binary not found! Build failed."
              exit 1
            fi

            echo "WASM build completed successfully!"
            echo ""
            echo "Note: SP1 circuit building is currently disabled due to toolchain issues."
            echo "The WASM binary is available at: $PRJ_ROOT/target/wasm32-unknown-unknown/optimized/valence_coprocessor_app_lib.wasm"
          '';

          # Deploy to service script
          deploy-to-service = pkgs.writeShellScriptBin "deploy-to-service" ''
            #!/usr/bin/env bash
            # Deploy WASM binary directly to the co-processor service using curl

            set -e

            PRJ_ROOT=$(pwd)
            WASM_PATH="$PRJ_ROOT/target/wasm32-unknown-unknown/optimized/valence_coprocessor_app_lib.wasm"
            SERVICE_URL="http://localhost:37281/api/registry/program"

            # Ensure the WASM binary exists
            if [ ! -f "$WASM_PATH" ]; then
              echo "Error: WASM binary not found at $WASM_PATH"
              echo "Please run 'nix run .#build-wasm' first to build the WASM binary"
              exit 1
            fi

            echo "Deploying WASM binary to co-processor service..."
            echo "WASM binary: $WASM_PATH"
            echo "Service URL: $SERVICE_URL"

            # Create a dummy circuit for now (we'll use the same WASM)
            CIRCUIT_PATH="$WASM_PATH"

            # Base64 encode the WASM binary - handle different OS formats
            OSTYPE=$(uname)
            if [[ "$OSTYPE" == "Darwin" ]]; then
              # macOS
              WASM_BASE64=$(base64 < "$WASM_PATH")
              CIRCUIT_BASE64=$(base64 < "$CIRCUIT_PATH")
            else
              # Linux
              WASM_BASE64=$(base64 "$WASM_PATH")
              CIRCUIT_BASE64=$(base64 "$CIRCUIT_PATH")
            fi

            # Deploy to the co-processor service
            RESPONSE=$(curl -s -X POST "$SERVICE_URL" \
              -H "Content-Type: application/json" \
              -d "{\"lib\": \"$WASM_BASE64\", \"circuit\": \"$CIRCUIT_BASE64\"}")

            # Extract the program ID
            PROGRAM_ID=$(echo "$RESPONSE" | grep -o '"program":"[^"]*"' | cut -d'"' -f4)

            if [ -n "$PROGRAM_ID" ]; then
              echo "Deployment successful!"
              echo "Program ID: $PROGRAM_ID"
              echo ""
              echo "To generate a proof, run:"
              echo "echo '{\"name\": \"Valence\"}' | curl -s -X POST \"http://localhost:37281/api/registry/program/$PROGRAM_ID/prove\" -H \"Content-Type: application/json\" -d '{\"args\":{\"name\":\"Valence\"}}'"
            else
              echo "Deployment failed. Response:"
              echo "$RESPONSE"
            fi
          '';

          # Full pipeline script
          full-pipeline = pkgs.writeShellScriptBin "full-pipeline" ''
            #!/usr/bin/env bash
            # Complete pipeline script for Valence coprocessor app
            # Builds WASM, deploys to service, and attempts to generate/verify proof

            set -e

            PRJ_ROOT=$(pwd)

            echo "=========================================="
            echo "Valence Coprocessor App - Complete Pipeline"
            echo "=========================================="

            # Step 1: Build the WASM binary
            echo ""
            echo "Step 1: Building WASM binary..."
            ${config.packages.build-wasm}/bin/build-wasm

            # Step 2: Deploy to the co-processor service
            echo ""
            echo "Step 2: Deploying to co-processor service..."
            DEPLOY_OUTPUT=$(${config.packages.deploy-to-service}/bin/deploy-to-service)
            echo "$DEPLOY_OUTPUT"

            # Extract the program ID
            PROGRAM_ID=$(echo "$DEPLOY_OUTPUT" | grep "Program ID:" | cut -d' ' -f3)

            if [ -z "$PROGRAM_ID" ]; then
              echo "Failed to extract Program ID. Deployment may have failed."
              exit 1
            fi

            # Step 3: Try to generate a proof
            echo ""
            echo "Step 3: Attempting to generate a proof..."
            PROOF_OUTPUT=$(echo '{"name": "Valence"}' | curl -s -X POST "http://localhost:37281/api/registry/program/$PROGRAM_ID/prove" -H "Content-Type: application/json" -d '{"args":{"name":"Valence"}}')

            echo "Proof generation output:"
            echo "$PROOF_OUTPUT" | jq . 2>/dev/null || echo "$PROOF_OUTPUT"

            # Check if we have errors
            if echo "$PROOF_OUTPUT" | grep -q "Error"; then
              echo ""
              echo "Note: The proof generation encountered errors."
              echo "This could be due to the following reasons:"
              echo "1. Missing imports in the WASM binary"
              echo "2. Incompatibility between the application and the service"
              echo "3. Configuration issues with the co-processor service"
              echo ""
              echo "The WASM binary was successfully built and deployed, but proof generation requires additional configuration."
            else
              # Step 4: Verify the proof (if we got one)
              echo ""
              echo "Step 4: Verifying the proof..."
              echo "$PROOF_OUTPUT" | curl -s -X POST "http://localhost:37281/api/registry/program/$PROGRAM_ID/verify" -H "Content-Type: application/json" -d @-
            fi

            echo ""
            echo "Pipeline completed!"
          '';
        };

        # WASM development shell - including our new scripts
        devshells.wasm-shell = {
          name = "wasm-shell";
          packages = [
            rustWithWasmTarget
            pkgs.wasm-bindgen-cli
            pkgs.curl
            pkgs.jq
            config.packages.install-cargo-prove
            config.packages.build-wasm
            config.packages.deploy-to-service
            config.packages.full-pipeline
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
        
        # SP1 development shell
        devshells.sp1-shell = {
          name = "sp1-shell";
          packages = [
            sp1-rust
            sp1
            pkgs.llvmPackages.clang
            pkgs.llvmPackages.llvm
            pkgs.curl
            pkgs.jq
            config.packages.install-cargo-prove
            config.packages.build-wasm
            config.packages.deploy-to-service
            config.packages.full-pipeline
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
              help = "Deploy the app using the deployment script";
              command = "full-pipeline";
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
        
        # Default development shell with access to all scripts
        devshells.default = {
          packages = [
            pkgs.curl
            pkgs.jq
            config.packages.install-cargo-prove
            config.packages.build-wasm
            config.packages.deploy-to-service
            config.packages.full-pipeline
          ];
          
          commands = [
            {
              category = "build";
              name = "build-wasm-cmd";
              help = "Build the WASM binary";
              command = "build-wasm";
            }
            {
              category = "deploy";
              name = "deploy-to-service-cmd";
              help = "Deploy WASM binary to the coprocessor service";
              command = "deploy-to-service";
            }
            {
              category = "pipeline";
              name = "full-pipeline-cmd";
              help = "Run the complete pipeline (build, deploy, proof)";
              command = "full-pipeline";
            }
            {
              category = "shells";
              name = "sp1";
              help = "Enter the SP1 development shell";
              command = "nix develop .#sp1-shell";
            }
            {
              category = "shells";
              name = "wasm";
              help = "Enter the WASM development shell";
              command = "nix develop .#wasm-shell";
            }
          ];
          
          bash.extra = ''
            echo "Valence Coprocessor App Development Environment"
            echo "View available commands with: menu"
            echo ""
            echo "Quick commands:"
            echo "  build-wasm          - Build the WASM binary"
            echo "  deploy-to-service   - Deploy to the coprocessor service"
            echo "  full-pipeline       - Run complete pipeline (build, deploy, proof)"
          '';
        };

        # Add apps to make everything runnable with 'nix run'
        apps = {
          build-wasm = {
            type = "app";
            program = "${config.packages.build-wasm}/bin/build-wasm";
          };
          deploy-to-service = {
            type = "app";
            program = "${config.packages.deploy-to-service}/bin/deploy-to-service";
          };
          full-pipeline = {
            type = "app";
            program = "${config.packages.full-pipeline}/bin/full-pipeline";
          };
          install-cargo-prove = {
            type = "app";
            program = "${config.packages.install-cargo-prove}/bin/install-cargo-prove";
          };
          # Default to running the full pipeline
          default = self'.apps.full-pipeline;
        };
      };
    });
}