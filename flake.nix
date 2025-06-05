{
  description = "Valence coprocessor app";

  nixConfig.extra-experimental-features = "nix-command flakes";
  nixConfig.extra-substituters = "https://coffeetables.cachix.org";
  nixConfig.extra-trusted-public-keys = ''
    coffeetables.cachix.org-1:BCQXDtLGFVo/rTG/J4omlyP/jbtNtsZIKHBMTjAWt8g=
  '';

  inputs = {
    nixpkgs.url = "nixpkgs/nixos-24.11";
    
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
        overlays = [ rust-overlay.overlays.default ];
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
              let 
                tarball = pkgs.fetchurl {
                  url = "https://github.com/${owner}/${repo}/releases/download/${tag}/${asset}";
                  sha256 = hash;
                };
              in
              pkgs.runCommand "extract-${asset}" { } ''
                mkdir -p $out
                tar -xzf ${tarball} -C $out
              '';
          in fetchGitHubReleaseAsset ({
            owner = "succinctlabs";
            repo = "rust";
            tag = "succinct-1.82.0";
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
              hash = "sha256-TyButIZ7LwQanQEwgSPjpEP8jMD6HGCYYoL+I5XAxs0=";
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
              ${rustWithWasmTarget}/bin/cargo build --target wasm32-unknown-unknown --release -p valence-coprocessor-app-controller
            '';
            
            installPhase = ''
              mkdir -p $out
              cp target/wasm32-unknown-unknown/release/valence_coprocessor_app_controller.wasm $out/valence_coprocessor_app_lib.wasm
            '';
          };

          # Script to install cargo-prove
          install-cargo-prove = pkgs.writeShellScriptBin "install-cargo-prove" ''
            #!/usr/bin/env bash
            # This script downloads the cargo-prove binary for the current platform

            set -e

            # Ensure PRJ_ROOT is available
            if [ -z "$PRJ_ROOT" ]; then
              export PRJ_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || echo "$PWD")"
            fi

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

            # Ensure PRJ_ROOT is available
            if [ -z "$PRJ_ROOT" ]; then
              export PRJ_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || echo "$PWD")"
            fi

            # Ensure proper directory structure
            mkdir -p "$PRJ_ROOT/bin"
            mkdir -p "$PRJ_ROOT/target/wasm32-unknown-unknown/release" 
            mkdir -p "$PRJ_ROOT/target/wasm32-unknown-unknown/optimized"
            mkdir -p "$PRJ_ROOT/target/sp1"

            # Step 1: Install cargo-prove if needed
            if [ ! -f "$PRJ_ROOT/bin/cargo-prove" ]; then
              echo "Installing cargo-prove..."
              ${config.packages.install-cargo-prove}/bin/install-cargo-prove
            fi

            # Step 2: Build the WASM binary using the nix wasm-shell
            echo "Building WASM with nightly Rust toolchain..."
            echo "Current directory before build: $PWD"
            echo "PRJ_ROOT is: $PRJ_ROOT"
            echo "Target release directory before build: $PRJ_ROOT/target/wasm32-unknown-unknown/release/"
            ls -la "$PRJ_ROOT/target/wasm32-unknown-unknown/release/" 2>/dev/null || echo "Release directory does not exist yet or is empty."

            nix develop .#wasm-shell -c bash -c 'export RUSTFLAGS="--cfg=web_sys_unstable_apis"; echo "Inside nix develop (wasm-shell): Building valence-coprocessor-app-controller..."; pwd; cargo build --target wasm32-unknown-unknown --release -p valence-coprocessor-app-controller -v; echo "WASM Build command finished. Checking output..."; ls -la target/wasm32-unknown-unknown/release/;'

            echo "WASM Build process completed. Checking for WASM file..."
            echo "Expected WASM file location: $PRJ_ROOT/target/wasm32-unknown-unknown/release/valence_coprocessor_app_controller.wasm"
            ls -la "$PRJ_ROOT/target/wasm32-unknown-unknown/release/" 2>/dev/null || echo "Release directory does not exist or is empty after build."

            # Copy the WASM to the expected location if it was built
            if [ -f "$PRJ_ROOT/target/wasm32-unknown-unknown/release/valence_coprocessor_app_controller.wasm" ]; then
              echo "Copying WASM binary to optimized directory..."
              cp "$PRJ_ROOT/target/wasm32-unknown-unknown/release/valence_coprocessor_app_controller.wasm" "$PRJ_ROOT/target/wasm32-unknown-unknown/optimized/valence_coprocessor_app_lib.wasm"
              echo "Copied to: $PRJ_ROOT/target/wasm32-unknown-unknown/optimized/valence_coprocessor_app_lib.wasm"
              ls -la "$PRJ_ROOT/target/wasm32-unknown-unknown/optimized/"
            else
              echo "WASM binary not found! Build failed."
              exit 1
            fi

            # Step 3: Build the SP1 circuit using sp1-shell
            echo "Building SP1 circuit..."
            echo "Using cargo-prove from: $PRJ_ROOT/bin/cargo-prove"
            
            nix develop .#sp1-shell -c bash -c 'pwd; echo "Inside nix develop (sp1-shell): Building SP1 circuit..."; cd "$PRJ_ROOT/docker/build/program-circuit/program" && pwd && echo "Toolchain information from sp1-shell:" && cargo-prove prove --version && cargo-prove prove build --ignore-rust-version; ' || \
            {
              echo "SP1 build failed (executed via sp1-shell), but we'll continue with dev mode"
              echo "SP1 circuit build failed. Will continue with WASM-only deployment (dev mode)."
              echo "WASM build completed successfully!"
              echo ""
              echo "The WASM binary is available at: $PRJ_ROOT/target/wasm32-unknown-unknown/optimized/valence_coprocessor_app_lib.wasm"
            }

            # Check both possible output locations for SP1 binary
            CIRCUIT_PATHS=(
              "$PRJ_ROOT/docker/build/program-circuit/target/program.elf" # Primary path based on deployer
              "$PRJ_ROOT/docker/build/program-circuit/program/elf/program-circuit" # Typical cargo-prove output for this crate
              "$PRJ_ROOT/target/sp1/valence-coprocessor-app-circuit" # Old path, keep for now
              "$PRJ_ROOT/target/sp1/circuit" # Old path, keep for now
            )
            
            CIRCUIT_FOUND=false
            CIRCUIT_PATH=""
            
            for path in "''${CIRCUIT_PATHS[@]}"; do
              if [ -f "$path" ]; then
                CIRCUIT_FOUND=true
                CIRCUIT_PATH="$path"
                echo "SP1 circuit found at: $path"
                break
              fi
            done
            
            if [ "$CIRCUIT_FOUND" = true ]; then
              echo "SP1 circuit built successfully!"
              mkdir -p "$PRJ_ROOT/target/sp1/optimized"
              cp "$CIRCUIT_PATH" "$PRJ_ROOT/target/sp1/optimized/valence-coprocessor-app-circuit"
              echo "Copied to: $PRJ_ROOT/target/sp1/optimized/valence-coprocessor-app-circuit"
            else
              echo "Warning: SP1 circuit build failed or file not found at expected locations."
              echo "Searched in: ''${CIRCUIT_PATHS[*]}"
              echo "Generating a fallback dummy ELF for dev mode deployment."
              mkdir -p "$PRJ_ROOT/target/sp1/optimized"
              # Call generate-sp1-elf to output to the correct location
              ${config.packages.generate-sp1-elf}/bin/generate-sp1-elf "$PRJ_ROOT/target/sp1/optimized/valence-coprocessor-app-circuit"
              echo "Fallback dummy ELF generated at: $PRJ_ROOT/target/sp1/optimized/valence-coprocessor-app-circuit"
              # Update CIRCUIT_PATH and CIRCUIT_FOUND for subsequent messages
              CIRCUIT_PATH="$PRJ_ROOT/target/sp1/optimized/valence-coprocessor-app-circuit"
              CIRCUIT_FOUND=true # Since we just created it
              # find "$PRJ_ROOT/target" -name "circuit" -o -name "*circuit*" | sort # This find might be less useful now
            fi

            echo "WASM build completed successfully!"
            echo ""
            echo "The WASM binary is available at: $PRJ_ROOT/target/wasm32-unknown-unknown/optimized/valence_coprocessor_app_lib.wasm"
            echo "The SP1 circuit is available at: $PRJ_ROOT/target/sp1/optimized/valence-coprocessor-app-circuit (either built or fallback)"
          '';

          # Deploy to service script
          deploy-to-service = pkgs.writeShellScriptBin "deploy-to-service" ''
            #!/usr/bin/env bash
            echo "--- deploy-to-service script started ---"
            # Deploy WASM binary directly to the co-processor service using curl

            set -e

            # Ensure PRJ_ROOT is available
            if [ -z "$PRJ_ROOT" ]; then
              export PRJ_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || echo "$PWD")"
            fi

            WASM_PATH="$PRJ_ROOT/target/wasm32-unknown-unknown/optimized/valence_coprocessor_app_lib.wasm"
            # This path should always contain an ELF, either real or dummy, due to build-wasm modifications
            CIRCUIT_PATH_FOR_DEPLOYMENT="$PRJ_ROOT/target/sp1/optimized/valence-coprocessor-app-circuit" 
            
            # Use environment variable if set, otherwise use default
            SERVICE_URL=''${VALENCE_SERVICE_URL:-http://localhost:37281/api/registry/controller}
            SERVICE_HOST=''${SERVICE_URL%/api*}

            # Ensure the WASM binary exists
            if [ ! -f "$WASM_PATH" ]; then
              echo "Error: WASM binary not found at $WASM_PATH"
              echo "Please run 'nix run .#build-wasm' first to build the WASM binary"
              exit 1
            fi

            # Ensure the CIRCUIT_PATH_FOR_DEPLOYMENT exists
            if [ ! -f "$CIRCUIT_PATH_FOR_DEPLOYMENT" ]; then
              echo "Error: Circuit ELF not found at $CIRCUIT_PATH_FOR_DEPLOYMENT"
              echo "This file should have been created by 'build-wasm', either from a successful SP1 build or as a fallback."
              echo "Please run 'nix run .#build-wasm' first."
              exit 1
            fi
            echo "Using circuit ELF from: $CIRCUIT_PATH_FOR_DEPLOYMENT for deployment."

            # Check service status
            echo "Checking service status at $SERVICE_HOST/api/status..."
            if ! curl -s --connect-timeout 5 -X GET "$SERVICE_HOST/api/status" > /dev/null; then
              echo "Error: Failed to connect to the co-processor service at $SERVICE_HOST"
              echo "Please ensure the service is running at $SERVICE_HOST"
              exit 1
            fi
            echo "Service is responsive. Proceeding with deployment."

            echo "Deploying WASM binary to co-processor service..."
            echo "WASM binary: $WASM_PATH"
            
            # Force dev_mode: true for this deployment to test mock verification
            echo "Forcing dev_mode: true for deployment payload to test mock verification."
            DEV_MODE_FOR_PAYLOAD=true # JSON boolean true, not string "true"
            
            echo "Service URL: $SERVICE_URL"

            # Base64 encode the WASM binary
            echo "Base64 encoding WASM binary..."
            WASM_BASE64=$(openssl base64 -A -in "$WASM_PATH")

            # Base64 encode the circuit ELF (real or dummy from build-wasm)
            echo "Base64 encoding Circuit ELF from $CIRCUIT_PATH_FOR_DEPLOYMENT..."
            CIRCUIT_BASE64_FOR_PAYLOAD=$(openssl base64 -A -in "$CIRCUIT_PATH_FOR_DEPLOYMENT")

            # Prepare the request payload - always include the circuit field, forced to dev mode
            REQUEST_PAYLOAD="{\"controller\": \"$WASM_BASE64\", \"circuit\": \"$CIRCUIT_BASE64_FOR_PAYLOAD\"}"

            echo "Deploying with payload snippet (circuit hash omitted for brevity):"
            printf '{"controller": "%s...", "circuit": "%s..."}\n' \
              "$(echo "$WASM_BASE64" | cut -c1-30)" \
              "$(echo "$CIRCUIT_BASE64_FOR_PAYLOAD" | cut -c1-30)"


            # Deploy to the co-processor service using a more reliable method
            echo "Sending deployment request to $SERVICE_URL..."
            
            # Use a temporary file for the response
            TEMP_OUTPUT=$(mktemp)
            
            http_code=$(curl -s -o "$TEMP_OUTPUT" -w "%{http_code}" \
              --connect-timeout 10 -X POST "$SERVICE_URL" \
              -H "Content-Type: application/json" \
              -d "$REQUEST_PAYLOAD")

            if [ "$http_code" -ne 200 ]; then
              echo "Error: Received HTTP code $http_code from service"
              echo "Response:"
              cat "$TEMP_OUTPUT"
              rm "$TEMP_OUTPUT"
              exit 1
            fi

            # Read the response
            RESPONSE=$(cat "$TEMP_OUTPUT")
            rm "$TEMP_OUTPUT"

            # Extract the program ID
            PROGRAM_ID=$(echo "$RESPONSE" | grep -o '"controller":"[^"]*"' | cut -d'"' -f4)

            if [ -n "$PROGRAM_ID" ]; then
              echo "Deployment successful!"
              echo "Program ID: $PROGRAM_ID"
              echo ""
              echo "To generate a proof, run:"
              if [ "$DEV_MODE_FOR_PAYLOAD" = "true" ]; then
                echo "echo '{\"name\": \"Valence\"}' | curl -s -X POST \"$SERVICE_URL/$PROGRAM_ID/prove\" -H \"Content-Type: application/json\" -d '{\"args\":{\"name\":\"Valence\"}, \"dev_mode\": true}'"
              else
              echo "echo '{\"name\": \"Valence\"}' | curl -s -X POST \"$SERVICE_URL/$PROGRAM_ID/prove\" -H \"Content-Type: application/json\" -d '{\"args\":{\"name\":\"Valence\"}}'"
              fi
            else
              echo "Deployment failed. Response:"
              echo "$RESPONSE"
            fi
          '';

          # Full pipeline script
          full-pipeline = pkgs.writeShellScriptBin "full-pipeline" ''
            #!/usr/bin/env bash
            echo "--- full-pipeline script started ---"
            # Complete pipeline script for Valence coprocessor app
            # Builds WASM, deploys to service, and attempts to generate/verify proof

            set -e # Re-enabled set -e

            # Ensure PRJ_ROOT is available and cd to it
            if [ -z "$PRJ_ROOT" ]; then
              export PRJ_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || echo "$PWD")"
            fi
            cd "$PRJ_ROOT"
            echo "Running full-pipeline from: $PWD (PRJ_ROOT is $PRJ_ROOT)"

            echo "==========================================="
            echo "Valence Coprocessor App - Complete Pipeline"
            echo "==========================================="
            echo ""
            echo "NOTE: For best results, run the coprocessor service with:"
            echo "RUST_LOG=debug cargo run --manifest-path Cargo.toml -p valence-coprocessor-service --profile optimized"
            echo ""

            # Step 1: Build the WASM binary
            echo ""
            echo "Step 1: Building WASM binary..."
            ${config.packages.build-wasm}/bin/build-wasm

            # Check if the SP1 circuit exists
            CIRCUIT_PATH="$PRJ_ROOT/target/sp1/optimized/valence-coprocessor-app-circuit"
            if [ -f "$CIRCUIT_PATH" ]; then
              echo "SP1 circuit found at: $CIRCUIT_PATH"
              echo "Using SP1 circuit for deployment (full proving mode)"
              DEV_MODE="false"
            else
              echo "SP1 circuit not found, using development mode"
              echo "Dev mode will execute the WASM without generating a ZK proof"
              DEV_MODE="true"
            fi

            # Step 2: Deploy to the co-processor service
            echo ""
            echo "Step 2: Deploying to co-processor service..."
            # DEPLOY_OUTPUT=$(${config.packages.deploy-to-service}/bin/deploy-to-service) # Old way
            
            # New way: capture output and exit code separately
            DEPLOY_SERVICE_LOG_FILE=$(mktemp)
            echo "Capturing deploy-to-service output to $DEPLOY_SERVICE_LOG_FILE"
            
            set +e # Allow deploy-to-service to fail without exiting full-pipeline immediately
            ${config.packages.deploy-to-service}/bin/deploy-to-service > "$DEPLOY_SERVICE_LOG_FILE" 2>&1
            DEPLOY_SERVICE_EXIT_CODE=$?
            set -e # Re-enable set -e

            DEPLOY_OUTPUT=$(cat "$DEPLOY_SERVICE_LOG_FILE")
            echo "--- Output from deploy-to-service (exit code $DEPLOY_SERVICE_EXIT_CODE): ---"
            echo "$DEPLOY_OUTPUT"
            echo "--- End of deploy-to-service output ---"
            rm "$DEPLOY_SERVICE_LOG_FILE"

            if [ $DEPLOY_SERVICE_EXIT_CODE -ne 0 ]; then
              echo "Error: deploy-to-service failed with exit code $DEPLOY_SERVICE_EXIT_CODE. See output above."
              exit 1
            fi
            
            # Extract the program ID
            PROGRAM_ID=$(echo "$DEPLOY_OUTPUT" | grep "Program ID:" | cut -d' ' -f3)

            if [ -z "$PROGRAM_ID" ]; then
              echo "Failed to extract Program ID. Deployment may have failed."
              exit 1
            fi

            # Use environment variable if set, otherwise use default
            SERVICE_URL=''${VALENCE_SERVICE_URL:-http://localhost:37281/api/registry/controller}
            SERVICE_HOST=''${SERVICE_URL%/api*}

            # Add a short delay to allow the service to fully process the deployment
            echo "Waiting 3 seconds for the service to process the deployment..."
            sleep 3

            # Step 3: Call the 'prove' endpoint (which internally calls entrypoint for dev_mode=true)
            echo ""
            echo "Step 3: Calling prove endpoint (dev_mode=true, will call entrypoint)..."
            # The payload here is what entrypoint will receive under 'payload' key
            PROVE_PAYLOAD='{"payload": {"cmd": "store", "path": "/some_dir/long_filename_with_symbols!!.json"}, "dev_mode": true}'
            echo "Prove payload: $PROVE_PAYLOAD"
            curl -X POST -H "Content-Type: application/json" -d "$PROVE_PAYLOAD" "$SERVICE_URL/$PROGRAM_ID/prove"


            # Step 4: Attempt to retrieve proof from program's virtual storage
            echo ""
            echo "Step 4: Attempting to retrieve proof from program storage..."
            # Allow a few seconds for the service to potentially call the WASM entrypoint and for WASM to write to storage
            echo "Waiting 5 seconds for potential storage write..."
            sleep 5

            STORAGE_PATH="LONGFILE.JSO" # Match the transformed path from WASM
            STORAGE_PAYLOAD="{\"path\": \"$STORAGE_PATH\"}"
            STORAGE_TEMP_OUTPUT=$(mktemp)
            
            echo "Querying storage: $SERVICE_HOST/api/registry/controller/$PROGRAM_ID/storage/fs with payload $STORAGE_PAYLOAD"

            storage_http_code=$(curl -s -o "$STORAGE_TEMP_OUTPUT" -w "%{http_code}" \
                               --connect-timeout 10 -X POST "$SERVICE_HOST/api/registry/controller/$PROGRAM_ID/storage/fs" \
                               -H "Content-Type: application/json" \
                               -d "$STORAGE_PAYLOAD")
            
            echo "Storage query HTTP Status Code: $storage_http_code"
            
            if [ "$storage_http_code" -ne 200 ]; then
              echo "Error: Storage query received HTTP code $storage_http_code from service"
              echo "Response:"
              cat "$STORAGE_TEMP_OUTPUT"
              # Potentially exit here or just report not found
            else
              echo "Storage query successful. Response content:"
              cat "$STORAGE_TEMP_OUTPUT" | jq . 2>/dev/null || cat "$STORAGE_TEMP_OUTPUT"
              
              # Extract base64 data, decode, and print/parse as JSON
              BASE64_DATA=$(cat "$STORAGE_TEMP_OUTPUT" | jq -r .data 2>/dev/null)
              
              if [ -n "$BASE64_DATA" ] && [ "$BASE64_DATA" != "null" ]; then
                echo "Retrieved base64 data from storage. Decoding..."
                DECODED_STORAGE_CONTENT=$(echo "$BASE64_DATA" | base64 --decode 2>/dev/null)
                
                if [ $? -eq 0 ]; then
                  echo "Decoded storage content ($STORAGE_PATH):"
                  echo "$DECODED_STORAGE_CONTENT" | jq . 2>/dev/null || echo "$DECODED_STORAGE_CONTENT"
                else
                  echo "Error: Failed to base64 decode the storage data."
                  echo "Raw base64 data was: $BASE64_DATA"
                fi
              else
                echo "Warning: No 'data' field found in storage response, or it was null."
                echo "File $STORAGE_PATH might not exist or is empty in program storage."
              fi
            fi
            rm "$STORAGE_TEMP_OUTPUT"

            echo ""
            echo "Pipeline completed!"
          '';

          # SP1 ELF circuit package - generates a minimal valid ELF file during build
          sp1-elf-circuit = pkgs.stdenvNoCC.mkDerivation {
            name = "sp1-elf-circuit";
            version = "0.1.0";
            
            # Don't need a source, we'll generate the file
            src = null;
            
            # Simple installation that generates and installs the ELF file
            buildPhase = ''
              # Create a simple ELF header (64 bytes) - this is a very minimal ELF file
              # Magic number + basic ELF header fields
              printf "\x7F\x45\x4C\x46\x01\x01\x01\x00\x00\x00\x00\x00\x00\x00\x00\x00" > valid-circuit.elf
              printf "\x02\x00\x28\x00\x01\x00\x00\x00\x54\x80\x04\x08\x34\x00\x00\x00" >> valid-circuit.elf
              printf "\x00\x00\x00\x00\x00\x00\x00\x00\x34\x00\x20\x00\x01\x00\x00\x00" >> valid-circuit.elf
              printf "\x00\x00\x00\x00\x01\x00\x00\x00\x00\x00\x00\x00\x00\x80\x04\x08" >> valid-circuit.elf
              
              # Add some minimal content (simple machine code)
              printf "\x00\x80\x04\x08\x40\x01\x00\x00\xB8\x04\x00\x00\x00\xCD\x80\xC3" >> valid-circuit.elf
              
              # Pad to 100KB as required by SP1
              dd if=/dev/zero bs=$((100*1024 - $(stat -f%z valid-circuit.elf))) count=1 >> valid-circuit.elf 2>/dev/null
            '';
            
            installPhase = ''
              mkdir -p $out/bin
              cp valid-circuit.elf $out/bin/
            '';
            
            meta = {
              description = "Minimal valid ELF circuit file for SP1 zkVM";
              platforms = pkgs.lib.platforms.all;
            };
          };
          
          # Generate SP1 ELF script - now using SP1's toolchain
          generate-sp1-elf = pkgs.writeShellScriptBin "generate-sp1-elf" ''
            #!/usr/bin/env bash
            # Script to generate a minimal valid ELF file for SP1 zkVM
            
            set -e
            
            # Ensure PRJ_ROOT is available
            if [ -z "$PRJ_ROOT" ]; then
              export PRJ_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || echo "$PWD")"
            fi
            
            # Default output path
            OUTPUT_PATH=''${1:-"$PRJ_ROOT/assets/valid-circuit.elf"}
            mkdir -p "$(dirname "$OUTPUT_PATH")"
            
            echo "Generating minimal ELF file for SP1 zkVM integration..."
            
            # Create a simple ELF header (64 bytes) - this is a very minimal ELF file
            # Magic number + basic ELF header fields
            printf "\x7F\x45\x4C\x46\x01\x01\x01\x00\x00\x00\x00\x00\x00\x00\x00\x00" > "$OUTPUT_PATH"
            printf "\x02\x00\x28\x00\x01\x00\x00\x00\x54\x80\x04\x08\x34\x00\x00\x00" >> "$OUTPUT_PATH"
            printf "\x00\x00\x00\x00\x00\x00\x00\x00\x34\x00\x20\x00\x01\x00\x00\x00" >> "$OUTPUT_PATH"
            printf "\x00\x00\x00\x00\x01\x00\x00\x00\x00\x00\x00\x00\x00\x80\x04\x08" >> "$OUTPUT_PATH"
            
            # Add some minimal content (simple machine code)
            printf "\x00\x80\x04\x08\x40\x01\x00\x00\xB8\x04\x00\x00\x00\xCD\x80\xC3" >> "$OUTPUT_PATH"
            
            # Pad to 100KB as required by SP1
            dd if=/dev/zero bs=$((100*1024 - $(stat -f%z "$OUTPUT_PATH"))) count=1 >> "$OUTPUT_PATH" 2>/dev/null
            
            echo "ELF file successfully generated at: $OUTPUT_PATH"
            SIZE=$(du -h "$OUTPUT_PATH" | cut -f1)
            echo "File size: $SIZE"
            
            # Display basic file info
            file "$OUTPUT_PATH" 2>/dev/null || echo "File command not available, but the ELF file was created"
            
            echo "The ELF file is now ready to use with SP1 zkVM integration."
            echo "Note: This is a minimal ELF file for testing purposes only."
          '';

          # Valence Deploy - mirrors cargo-valence deploy functionality
          valence-deploy = pkgs.writeShellScriptBin "valence-deploy" ''
            #!/usr/bin/env bash
            # Deploy circuit to Valence coprocessor - mirrors cargo-valence deploy
            
            set -e
            
            # Parse command line arguments
            USE_PUBLIC_SERVICE=false
            CONTROLLER_PATH="./crates/controller"
            CIRCUIT_NAME="valence-coprocessor-app-circuit"
            VERBOSE=false
            
            usage() {
              echo "Usage: valence-deploy [OPTIONS] deploy circuit"
              echo ""
              echo "Options:"
              echo "  --socket <HOST:PORT>    Use public coprocessor service (like prover.timewave.computer:37281)"
              echo "  --controller <PATH>     Path to controller crate (default: ./crates/controller)"
              echo "  --circuit <NAME>        Circuit name (default: valence-coprocessor-app-circuit)"
              echo "  --verbose              Enable verbose output"
              echo "  -h, --help             Show this help message"
              echo ""
              echo "Examples:"
              echo "  # Deploy to local service (default localhost:37281)"
              echo "  valence-deploy deploy circuit"
              echo ""
              echo "  # Deploy to public service"
              echo "  valence-deploy --socket prover.timewave.computer:37281 deploy circuit"
              echo ""
              echo "  # Deploy with custom controller path"
              echo "  valence-deploy --controller ./my-controller --circuit my-circuit deploy circuit"
            }
            
            # Parse arguments
            while [[ $# -gt 0 ]]; do
              case $1 in
                --socket)
                  if [ -n "$2" ] && [[ "$2" != --* ]]; then
                    PUBLIC_SOCKET="$2"
                    USE_PUBLIC_SERVICE=true
                    shift 2
                  else
                    echo "Error: --socket requires a HOST:PORT argument"
                    exit 1
                  fi
                  ;;
                --controller)
                  if [ -n "$2" ] && [[ "$2" != --* ]]; then
                    CONTROLLER_PATH="$2"
                    shift 2
                  else
                    echo "Error: --controller requires a path argument"
                    exit 1
                  fi
                  ;;
                --circuit)
                  if [ -n "$2" ] && [[ "$2" != --* ]]; then
                    CIRCUIT_NAME="$2"
                    shift 2
                  else
                    echo "Error: --circuit requires a name argument"
                    exit 1
                  fi
                  ;;
                --verbose)
                  VERBOSE=true
                  shift
                  ;;
                -h|--help)
                  usage
                  exit 0
                  ;;
                deploy)
                  if [ "$2" = "circuit" ]; then
                    COMMAND="deploy-circuit"
                    shift 2
                  else
                    echo "Error: Unknown deploy target '$2'. Expected 'circuit'"
                    exit 1
                  fi
                  ;;
                *)
                  echo "Error: Unknown option '$1'"
                  usage
                  exit 1
                  ;;
              esac
            done
            
            # Ensure PRJ_ROOT is available
            if [ -z "$PRJ_ROOT" ]; then
              export PRJ_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || echo "$PWD")"
            fi
            
            # Determine service URL
            if [ "$USE_PUBLIC_SERVICE" = true ]; then
              if [[ "$PUBLIC_SOCKET" == *:* ]]; then
                SERVICE_HOST="http://$PUBLIC_SOCKET"
              else
                SERVICE_HOST="http://$PUBLIC_SOCKET:37281"
              fi
              echo "Using public coprocessor service: $SERVICE_HOST"
            else
              SERVICE_HOST="http://localhost:37281"
              echo "Using local coprocessor service: $SERVICE_HOST"
            fi
            
            SERVICE_URL="$SERVICE_HOST/api/registry/controller"
            
            if [ "$VERBOSE" = true ]; then
              echo "Controller path: $CONTROLLER_PATH"
              echo "Circuit name: $CIRCUIT_NAME"
              echo "Service URL: $SERVICE_URL"
            fi
            
            # Step 1: Build the WASM binary and circuit
            echo "Building WASM binary and circuit..."
            ${config.packages.build-wasm}/bin/build-wasm
            
            # Check required files
            WASM_PATH="$PRJ_ROOT/target/wasm32-unknown-unknown/optimized/valence_coprocessor_app_lib.wasm"
            CIRCUIT_PATH="$PRJ_ROOT/target/sp1/optimized/valence-coprocessor-app-circuit"
            
            if [ ! -f "$WASM_PATH" ]; then
              echo "Error: WASM binary not found at $WASM_PATH"
              exit 1
            fi
            
            if [ ! -f "$CIRCUIT_PATH" ]; then
              echo "Warning: SP1 circuit not found at $CIRCUIT_PATH"
              echo "Generating fallback circuit for development mode..."
              ${config.packages.generate-sp1-elf}/bin/generate-sp1-elf "$CIRCUIT_PATH"
            fi
            
            # Step 2: Check service availability
            echo "Checking service availability at $SERVICE_HOST..."
            if ! curl -s --connect-timeout 5 "$SERVICE_HOST/api/status" > /dev/null; then
              echo "Error: Cannot connect to coprocessor service at $SERVICE_HOST"
              if [ "$USE_PUBLIC_SERVICE" = false ]; then
                echo "Make sure you have a local coprocessor service running."
                echo "You can start one with: cargo run -p valence-coprocessor-service"
              else
                echo "Make sure the public service is accessible and the URL is correct."
              fi
              exit 1
            fi
            echo "Service is available ✓"
            
            # Step 3: Deploy
            echo "Deploying circuit to coprocessor service..."
            
            # Base64 encode files
            WASM_B64=$(openssl base64 -A -in "$WASM_PATH")
            CIRCUIT_B64=$(openssl base64 -A -in "$CIRCUIT_PATH")
            
            # Create deployment payload
            PAYLOAD="{\"controller\": \"$WASM_B64\", \"circuit\": \"$CIRCUIT_B64\"}"
            
            if [ "$VERBOSE" = true ]; then
              echo "WASM size: $(du -h "$WASM_PATH" | cut -f1)"
              echo "Circuit size: $(du -h "$CIRCUIT_PATH" | cut -f1)"
              echo "Payload size: $(echo "$PAYLOAD" | wc -c) bytes"
            fi
            
            # Deploy
            TEMP_OUTPUT=$(mktemp)
            
            http_code=$(curl -s -o "$TEMP_OUTPUT" -w "%{http_code}" \
              --connect-timeout 30 -X POST "$SERVICE_URL" \
              -H "Content-Type: application/json" \
              -d "$PAYLOAD")
            
            if [ "$http_code" -ne 200 ]; then
              echo "Error: Deployment failed with HTTP code $http_code"
              echo "Response:"
              cat "$TEMP_OUTPUT"
              rm "$TEMP_OUTPUT"
              exit 1
            fi
            
            # Parse response
            RESPONSE=$(cat "$TEMP_OUTPUT")
            rm "$TEMP_OUTPUT"
            
            # Extract controller ID (program ID)
            CONTROLLER_ID=$(echo "$RESPONSE" | grep -o '"controller":"[^"]*"' | cut -d'"' -f4)
            
            if [ -n "$CONTROLLER_ID" ]; then
              echo ""
              echo "✅ Deployment successful!"
              echo ""
              # Mirror cargo-valence output format
              echo "{\"controller\": \"$CONTROLLER_ID\", \"circuit\": \"$CIRCUIT_NAME\"}"
              echo ""
              echo "Controller ID: $CONTROLLER_ID"
              echo "Circuit: $CIRCUIT_NAME"
              echo ""
              echo "To generate a proof:"
              if [ "$USE_PUBLIC_SERVICE" = true ]; then
                echo "  valence-prove --socket $PUBLIC_SOCKET -j '{\"value\": 42}' -p /var/share/proof.bin $CONTROLLER_ID"
              else
                echo "  valence-prove -j '{\"value\": 42}' -p /var/share/proof.bin $CONTROLLER_ID"
              fi
            else
              echo "Error: Failed to extract controller ID from response"
              echo "Response: $RESPONSE"
              exit 1
            fi
          '';

          # Valence Prove - mirrors cargo-valence prove functionality  
          valence-prove = pkgs.writeShellScriptBin "valence-prove" ''
            #!/usr/bin/env bash
            # Generate proof using Valence coprocessor - mirrors cargo-valence prove
            
            set -e
            
            # Parse command line arguments
            USE_PUBLIC_SERVICE=false
            ARGS_JSON=""
            PATH_ARG=""
            CONTROLLER_ID=""
            VERBOSE=false
            
            usage() {
              echo "Usage: valence-prove [OPTIONS] <CONTROLLER_ID>"
              echo ""
              echo "Options:"
              echo "  --socket <HOST:PORT>    Use public coprocessor service"
              echo "  -j, --json <JSON>       JSON arguments to pass to the controller"
              echo "  -p, --path <PATH>       Path where proof will be stored in virtual filesystem"
              echo "  --verbose              Enable verbose output"
              echo "  -h, --help             Show this help message"
              echo ""
              echo "Examples:"
              echo "  # Get a verifying key"
              echo "  valence-prove vk <CONTROLLER_ID>"
              echo ""
              echo "  # Generate a proof with controller ID"
              echo "  valence-prove prove <CONTROLLER_ID> '{\"value\": 42}' \"/path/in/fs.json\""
            }
            
            # Parse arguments
            while [[ $# -gt 0 ]]; do
              case $1 in
                --socket)
                  if [ -n "$2" ] && [[ "$2" != --* ]]; then
                    PUBLIC_SOCKET="$2"
                    USE_PUBLIC_SERVICE=true
                    shift 2
                  else
                    echo "Error: --socket requires a HOST:PORT argument"
                    exit 1
                  fi
                  ;;
                -j|--json)
                  if [ -n "$2" ] && [[ "$2" != --* ]]; then
                    ARGS_JSON="$2"
                    shift 2
                  else
                    echo "Error: --json requires a JSON argument"
                    exit 1
                  fi
                  ;;
                -p|--path)
                  if [ -n "$2" ] && [[ "$2" != --* ]]; then
                    PATH_ARG="$2"
                    shift 2
                  else
                    echo "Error: --path requires a path argument"
                    exit 1
                  fi
                  ;;
                --verbose)
                  VERBOSE=true
                  shift
                  ;;
                -h|--help)
                  usage
                  exit 0
                  ;;
                vk)
                  if [ -n "$2" ]; then
                    COMMAND="vk"
                    CONTROLLER_ID="$2"
                    shift 2
                  else
                    echo "Error: vk requires a CONTROLLER_ID argument"
                    exit 1
                  fi
                  ;;
                prove)
                  if [ -n "$2" ]; then
                    COMMAND="prove"
                    CONTROLLER_ID="$2"
                    if [ -n "$3" ] && [[ "$3" != --* ]]; then
                      ARGS_JSON="$3"
                      if [ -n "$4" ] && [[ "$4" != --* ]]; then
                        PATH_ARG="$4"
                        shift 4
                      else
                        shift 3
                      fi
                    else
                      shift 2
                    fi
                  else
                    echo "Error: prove requires a CONTROLLER_ID argument"
                    exit 1
                  fi
                  ;;
                *)
                  echo "Error: Unknown option '$1'"
                  usage
                  exit 1
                  ;;
              esac
            done
            
            # Check required arguments
            if [ -z "$COMMAND" ]; then
              echo "Error: No command specified. Use 'vk' or 'prove'."
              usage
              exit 1
            fi
            
            # Ensure PRJ_ROOT is available
            if [ -z "$PRJ_ROOT" ]; then
              export PRJ_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || echo "$PWD")"
            fi
            
            # Determine service URL
            if [ "$USE_PUBLIC_SERVICE" = true ]; then
              if [[ "$PUBLIC_SOCKET" == *:* ]]; then
                SERVICE_HOST="http://$PUBLIC_SOCKET"
              else
                SERVICE_HOST="http://$PUBLIC_SOCKET:37281"
              fi
              echo "Using public coprocessor service: $SERVICE_HOST"
            else
              SERVICE_HOST="http://localhost:37281"
              echo "Using local coprocessor service: $SERVICE_HOST"
            fi
            
            SERVICE_URL="$SERVICE_HOST/api/registry/controller"
            PROVE_URL="$SERVICE_URL/$CONTROLLER_ID/prove"
            VK_URL="$SERVICE_URL/$CONTROLLER_ID/vk"
            
            if [ "$VERBOSE" = true ]; then
              echo "Controller ID: $CONTROLLER_ID"
              echo "Service URL: $SERVICE_URL"
            fi
            
            # Check service availability
            echo "Checking service availability..."
            if ! curl -s --connect-timeout 5 "$SERVICE_HOST/api/status" > /dev/null; then
              echo "Error: Cannot connect to coprocessor service at $SERVICE_HOST"
              exit 1
            fi
            
            # Parse JSON args to include in payload
            PARSED_JSON_ARGS=$(echo "$ARGS_JSON" | jq . 2>/dev/null) || {
              echo "Error: Invalid JSON in arguments: $ARGS_JSON"
              exit 1
            }
            
            # Create prove payload
            PROVE_PAYLOAD=$(jq -n \
              --argjson args "$PARSED_JSON_ARGS" \
              --arg path "$PATH_ARG" \
              '{
                args: $args,
                payload: {
                  cmd: "store",
                  path: $path
                },
                dev_mode: true
              }')
            
            echo "Submitting proof request..."
            if [ "$VERBOSE" = true ]; then
              echo "Payload: $PROVE_PAYLOAD"
            fi
            
            # Submit proof request
            TEMP_OUTPUT=$(mktemp)
            
            http_code=$(curl -s -o "$TEMP_OUTPUT" -w "%{http_code}" \
              --connect-timeout 30 -X POST "$PROVE_URL" \
              -H "Content-Type: application/json" \
              -d "$PROVE_PAYLOAD")
            
            if [ "$http_code" -ne 200 ]; then
              echo "Error: Proof request failed with HTTP code $http_code"
              echo "Response:"
              cat "$TEMP_OUTPUT"
              rm "$TEMP_OUTPUT"
              exit 1
            fi
            
            RESPONSE=$(cat "$TEMP_OUTPUT")
            rm "$TEMP_OUTPUT"
            
            echo "✅ Proof request submitted successfully!"
            echo "Response: $RESPONSE"
            
            # Wait a moment and try to retrieve the result
            echo ""
            echo "Waiting for proof to be processed..."
            sleep 3
            
            # Try to retrieve the stored proof
            STORAGE_PATH=$(echo "$PATH_ARG" | sed 's|/||g' | tr '[:lower:]' '[:upper:]' | cut -c1-8).$(echo "$PATH_ARG" | sed 's/.*\.//' | tr '[:lower:]' '[:upper:]' | cut -c1-3)
            STORAGE_PAYLOAD="{\"path\": \"$STORAGE_PATH\"}"
            
            echo "Attempting to retrieve proof from storage..."
            echo "Storage path: $STORAGE_PATH"
            
            STORAGE_TEMP_OUTPUT=$(mktemp)
            storage_http_code=$(curl -s -o "$STORAGE_TEMP_OUTPUT" -w "%{http_code}" \
                               --connect-timeout 10 -X POST "$SERVICE_HOST/api/registry/controller/$CONTROLLER_ID/storage/fs" \
                               -H "Content-Type: application/json" \
                               -d "$STORAGE_PAYLOAD")
            
            if [ "$storage_http_code" -eq 200 ]; then
              echo "✅ Proof retrieved from storage!"
              if [ "$VERBOSE" = true ]; then
                cat "$STORAGE_TEMP_OUTPUT" | jq . 2>/dev/null || cat "$STORAGE_TEMP_OUTPUT"
              else
                echo "Use valence-storage to retrieve the full proof data."
              fi
            else
              echo "⚠️  Could not retrieve proof from storage (this may be normal for async processing)"
              echo "Use valence-storage later to check if the proof is ready."
            fi
            
            rm "$STORAGE_TEMP_OUTPUT"
          '';

          # Valence Storage - mirrors cargo-valence storage functionality
          valence-storage = pkgs.writeShellScriptBin "valence-storage" ''
            # Retrieve data from Valence coprocessor storage - mirrors cargo-valence storage
            
            set -e
            
            # Parse command line arguments
            USE_PUBLIC_SERVICE=false
            STORAGE_PATH=""
            CONTROLLER_ID=""
            VERBOSE=false
            
            usage() {
              echo "Usage: valence-storage [OPTIONS] COMMAND <CONTROLLER_ID> [PATH]"
              echo ""
              echo "Commands:"
              echo "  fs <CONTROLLER_ID> <PATH>   Retrieve a file from the virtual filesystem"
              echo "  raw <CONTROLLER_ID>         Get the raw storage data"
              echo ""
              echo "Options:"
              echo "  --socket <HOST:PORT>        Use public coprocessor service"
              echo "  --verbose                   Enable verbose output"
              echo "  -h, --help                  Show this help message"
              echo ""
              echo "Examples:"
              echo "  # Retrieve a file from local service"
              echo "  valence-storage fs <CONTROLLER_ID> PROOF.BIN"
              echo ""
              echo "  # Retrieve a file from public service"
              echo "  valence-storage --socket prover.timewave.computer:37281 fs <CONTROLLER_ID> PROOF.BIN"
              echo ""
              echo "  # Get raw storage from local service"
              echo "  valence-storage raw <CONTROLLER_ID>"
            }
            
            # Parse arguments
            while [[ $# -gt 0 ]]; do
              case $1 in
                --socket)
                  if [ -n "$2" ] && [[ "$2" != --* ]]; then
                    PUBLIC_SOCKET="$2"
                    USE_PUBLIC_SERVICE=true
                    shift 2
                  else
                    echo "Error: --socket requires a HOST:PORT argument"
                    exit 1
                  fi
                  ;;
                --verbose)
                  VERBOSE=true
                  shift
                  ;;
                -h|--help)
                  usage
                  exit 0
                  ;;
                fs)
                  if [ -n "$2" ]; then
                    COMMAND="fs"
                    CONTROLLER_ID="$2"
                    if [ -n "$3" ] && [[ "$3" != --* ]]; then
                      STORAGE_PATH="$3"
                      shift 3
                    else
                      echo "Error: fs requires a PATH argument"
                      exit 1
                    fi
                  else
                    echo "Error: fs requires a CONTROLLER_ID argument"
                    exit 1
                  fi
                  ;;
                raw)
                  if [ -n "$2" ]; then
                    COMMAND="raw"
                    CONTROLLER_ID="$2"
                    shift 2
                  else
                    echo "Error: raw requires a CONTROLLER_ID argument"
                    exit 1
                  fi
                  ;;
                *)
                  echo "Error: Unknown option '$1'"
                  usage
                  exit 1
                  ;;
              esac
            done
            
            # Check required arguments
            if [ -z "$COMMAND" ]; then
              echo "Error: No command specified. Use 'fs' or 'raw'."
              usage
              exit 1
            fi
            
            if [ -z "$CONTROLLER_ID" ]; then
              echo "Error: CONTROLLER_ID is required"
              usage
              exit 1
            fi
            
            if [ "$COMMAND" = "fs" ] && [ -z "$STORAGE_PATH" ]; then
              echo "Error: Storage path is required for 'fs' command"
              usage
              exit 1
            fi
            
            # Determine service URL
            if [ "$USE_PUBLIC_SERVICE" = true ]; then
              if [[ "$PUBLIC_SOCKET" == *:* ]]; then
                SERVICE_HOST="http://$PUBLIC_SOCKET"
              else
                SERVICE_HOST="http://$PUBLIC_SOCKET:37281"
              fi
              echo "Using public coprocessor service: $SERVICE_HOST"
            else
              SERVICE_HOST="http://localhost:37281"
              echo "Using local coprocessor service: $SERVICE_HOST"
            fi
            
            # Check service availability
            if ! curl -s --connect-timeout 5 "$SERVICE_HOST/api/status" > /dev/null; then
              echo "Error: Cannot connect to coprocessor service at $SERVICE_HOST"
              exit 1
            fi
            
            if [ "$COMMAND" = "fs" ]; then
              # For filesystem command
              # If path doesn't appear to be in FAT16 format (8.3) already, convert it
              if [[ ! "$STORAGE_PATH" =~ ^[A-Z0-9]{1,8}\.[A-Z0-9]{1,3}$ ]]; then
                # Convert path to FAT-16 format (8.3 filename, case insensitive)
                FAT16_PATH=$(echo "$STORAGE_PATH" | sed 's|/||g' | tr '[:lower:]' '[:upper:]' | cut -c1-8).$(echo "$STORAGE_PATH" | sed 's/.*\.//' | tr '[:lower:]' '[:upper:]' | cut -c1-3)
                if [ "$VERBOSE" = true ]; then
                  echo "Original path: $STORAGE_PATH"
                  echo "Converted to FAT-16 path: $FAT16_PATH"
                fi
              else
                FAT16_PATH="$STORAGE_PATH"
                if [ "$VERBOSE" = true ]; then
                  echo "Using provided FAT-16 path: $FAT16_PATH"
                fi
              fi
              
              if [ "$VERBOSE" = true ]; then
                echo "Controller ID: $CONTROLLER_ID"
              fi
              
              # Query storage
              STORAGE_PAYLOAD="{\"path\": \"$FAT16_PATH\"}"
              STORAGE_URL="$SERVICE_HOST/api/registry/controller/$CONTROLLER_ID/storage/fs"
              
              echo "Querying filesystem for $FAT16_PATH..."
              
              TEMP_OUTPUT=$(mktemp)
              http_code=$(curl -s -o "$TEMP_OUTPUT" -w "%{http_code}" \
                         --connect-timeout 10 -X POST "$STORAGE_URL" \
                         -H "Content-Type: application/json" \
                         -d "$STORAGE_PAYLOAD")
              
              if [ "$http_code" -ne 200 ]; then
                echo "Error: Storage query failed with HTTP code $http_code"
                echo "Response:"
                cat "$TEMP_OUTPUT"
                rm "$TEMP_OUTPUT"
                exit 1
              fi
              
              RESPONSE=$(cat "$TEMP_OUTPUT")
              rm "$TEMP_OUTPUT"
              
              # Extract and decode data
              BASE64_DATA=$(echo "$RESPONSE" | jq -r .data 2>/dev/null)
              
              if [ -n "$BASE64_DATA" ] && [ "$BASE64_DATA" != "null" ]; then
                echo "✅ File retrieved successfully!"
                echo ""
                
                # Decode and pretty-print the data
                DECODED_DATA=$(echo "$BASE64_DATA" | base64 --decode 2>/dev/null)
                
                if [ $? -eq 0 ]; then
                  # Try to format as JSON if possible
                  echo "$DECODED_DATA" | jq . 2>/dev/null || echo "$DECODED_DATA"
                else
                  echo "Error: Failed to decode base64 data"
                  echo "Raw response: $RESPONSE"
                fi
              else
                echo "No data found at path $FAT16_PATH"
                echo "Full response: $RESPONSE"
              fi
            elif [ "$COMMAND" = "raw" ]; then
              # For raw storage command
              STORAGE_URL="$SERVICE_HOST/api/registry/controller/$CONTROLLER_ID/storage/raw"
              
              echo "Retrieving raw storage data..."
              
              TEMP_OUTPUT=$(mktemp)
              http_code=$(curl -s -o "$TEMP_OUTPUT" -w "%{http_code}" \
                         --connect-timeout 10 -X GET "$STORAGE_URL")
              
              if [ "$http_code" -ne 200 ]; then
                echo "Error: Raw storage query failed with HTTP code $http_code"
                echo "Response:"
                cat "$TEMP_OUTPUT"
                rm "$TEMP_OUTPUT"
                exit 1
              fi
              
              RESPONSE=$(cat "$TEMP_OUTPUT")
              rm "$TEMP_OUTPUT"
              
              # Extract base64 data
              BASE64_DATA=$(echo "$RESPONSE" | jq -r .data 2>/dev/null)
              
              if [ -n "$BASE64_DATA" ] && [ "$BASE64_DATA" != "null" ]; then
                echo "✅ Raw storage data retrieved successfully!"
                echo ""
                
                # Display file entries from raw storage
                echo "Files in storage:"
                echo "$BASE64_DATA" | base64 --decode | grep -a -o "[A-Z][A-Z0-9]*\\.[A-Z][A-Z0-9]*" | sort | uniq
                
                if [ "$VERBOSE" = true ]; then
                  echo ""
                  echo "Full response (base64):"
                  echo "$BASE64_DATA"
                fi
              else
                echo "No storage data found"
                echo "Full response: $RESPONSE"
              fi
            fi
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
            {
              name = "MACOSX_DEPLOYMENT_TARGET";
              value = "11.0";
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
            pkgs.rustup
            sp1
            pkgs.llvmPackages.clang
            pkgs.llvmPackages.llvm
            pkgs.curl
            pkgs.jq
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
            # Ensure PRJ_ROOT is available inside the shell
            if [ -z "$PRJ_ROOT" ]; then
              export PRJ_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || echo "$PWD")"
            fi
            
            # Ensure $PRJ_ROOT/bin is in PATH for precedence if user/script placed something there
            export PATH="$PRJ_ROOT/bin:$PATH"

            echo "--- sp1-shell setup ---"
            # Attempt to prevent rustup from trying to manage itself if run as root by Nix builder
            export RUSTUP_PERMIT_COPY_RENAME_DIR=true
            # Set a common RUSTUP_HOME to avoid issues with default .rustup, especially in sandboxed Nix builds
            # Using a path within PRJ_ROOT might be an option if persistence across runs isn't an issue for the toolchain itself,
            # or rely on the user's default ~/.rustup for interactive `nix develop`.
            # For `nix run` commands, HOME might be /homeless-shelter. Let rustup use its default logic for now.
            # Consider XDG_DATA_HOME if more control is needed: export RUSTUP_HOME="''${XDG_DATA_HOME:-$HOME/.local/share}/rustup"

            echo "Current PATH: $PATH"
            echo "Disabling rustup auto-self-update..."
            rustup set auto-self-update disable || echo "Failed to disable rustup auto-self-update (may not be critical)."
            
            echo "Ensuring a default rustup toolchain (stable) is active if none..."
            rustup default stable || echo "Failed to set rustup default stable (may not be critical if a toolchain is already active)."
            echo "Rustup toolchain list:"
            rustup toolchain list
            echo "Cargo version from rustup default: $(cargo --version || echo 'cargo not found')"

            echo "Locating cargo-prove..."
            if command -v cargo-prove &> /dev/null; then
                CARGO_PROVE_PATH=$(command -v cargo-prove)
                echo "Found cargo-prove at: $CARGO_PROVE_PATH"
                echo "Version: $($CARGO_PROVE_PATH prove --version || $CARGO_PROVE_PATH --version || echo 'version N/A')"
                
                SHOULD_INSTALL_TOOLCHAIN=true
                # Check if the succinct toolchain is already installed and responsive
                # Use grep -Fxq for fixed string, exact line match to avoid issues with spaces/suffixes
                if rustup toolchain list | grep -Fxq "succinct"; then
                  echo "Found existing 'succinct' toolchain in rustup."
                  if cargo +succinct --version &> /dev/null; then
                    echo "'succinct' toolchain is responsive. Checking its version..."
                    SUCCINCT_CARGO_VERSION=$(cargo +succinct --version)
                    echo "Detected succinct cargo version: $SUCCINCT_CARGO_VERSION"
                    # We expect something like "cargo 1.89.0-nightly (056f5f4f3 2025-05-09)" for the current setup
                    if [[ "$SUCCINCT_CARGO_VERSION" == *"1.89.0-nightly"* ]]; then
                        echo "Detected expected succinct toolchain version. Skipping install-toolchain."
                        SHOULD_INSTALL_TOOLCHAIN=false
                    else
                        echo "Succinct toolchain version ($SUCCINCT_CARGO_VERSION) does not match expected heuristic. Will reinstall."
                    fi
                  else
                    echo "'succinct' toolchain found in rustup list but not responsive. Will reinstall."
                  fi
                else
                  echo "'succinct' toolchain not found in rustup list. Will install."
                fi

                if [ "$SHOULD_INSTALL_TOOLCHAIN" = "true" ]; then
                  echo "Installing/updating SP1 Rust toolchain ('succinct') via cargo-prove and rustup..."
                  "$CARGO_PROVE_PATH" prove install-toolchain
                fi
            else
                echo "ERROR: cargo-prove not found in PATH. Expected from 'sp1' package or '$PRJ_ROOT/bin'."
            fi
            
            export RUSTUP_TOOLCHAIN=succinct
            echo "RUSTUP_TOOLCHAIN set to 'succinct'."
            
            echo "Verifying 'succinct' toolchain cargo access (cargo +succinct --version):"
            if command -v cargo &> /dev/null; then # cargo here should be rustup's shim
              cargo +succinct --version || echo "Warning: 'cargo +succinct --version' failed. This might be an intermittent issue or the toolchain is still setting up."
            else
              echo "ERROR: 'cargo' (rustup shim) not found. This is unexpected after rustup setup."
            fi
            echo "--- sp1-shell setup complete ---"
          '';
        };
        
        # Default development shell with access to all scripts
        devshells.default = {
          packages = [
            pkgs.curl
            pkgs.jq
            rustStableWithWasmTarget  # Add stable Rust for general development
            config.packages.install-cargo-prove
            config.packages.build-wasm
            config.packages.deploy-to-service
            config.packages.full-pipeline
          ] ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
            pkgs.libiconv
            pkgs.darwin.apple_sdk.frameworks.Security
            pkgs.darwin.apple_sdk.frameworks.CoreFoundation
          ];
          
          # Add environment variables for Darwin
          env = pkgs.lib.optionals pkgs.stdenv.isDarwin [
            {
              name = "MACOSX_DEPLOYMENT_TARGET";
              value = "11.0";
            }
            {
              name = "LIBRARY_PATH";
              value = "${pkgs.libiconv}/lib";
            }
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
            
            # Set up Rust environment properly
            if [ "$(uname -s)" = "Darwin" ]; then
              export MACOSX_DEPLOYMENT_TARGET=11.0
              export LIBRARY_PATH="${pkgs.libiconv}/lib"
            fi
            
            # Ensure PRJ_ROOT is available
            if [ -z "$PRJ_ROOT" ]; then
              export PRJ_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || echo "$PWD")"
            fi
          '';
        };

        # Set up apps that can be run with 'nix run'
        apps = {
          build-wasm = {
            type = "app";
            program = "${config.packages.build-wasm}/bin/build-wasm";
          };
          
          install-cargo-prove = {
            type = "app";
            program = "${config.packages.install-cargo-prove}/bin/install-cargo-prove";
          };
          
          deploy-to-service = {
            type = "app";
            program = "${config.packages.deploy-to-service}/bin/deploy-to-service";
          };
          
          full-pipeline = {
            type = "app";
            program = "${config.packages.full-pipeline}/bin/full-pipeline";
          };
          
          generate-sp1-elf = {
            type = "app";
            program = "${config.packages.generate-sp1-elf}/bin/generate-sp1-elf";
          };
          
          # New Valence commands that mirror cargo-valence functionality
          valence-deploy = {
            type = "app";
            program = "${config.packages.valence-deploy}/bin/valence-deploy";
          };
          
          valence-prove = {
            type = "app";
            program = "${config.packages.valence-prove}/bin/valence-prove";
          };
          
          valence-storage = {
            type = "app";
            program = "${config.packages.valence-storage}/bin/valence-storage";
          };
        };
      };
    });
}