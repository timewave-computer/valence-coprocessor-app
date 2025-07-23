{
  description = "Valence coprocessor app";

  nixConfig = {
    extra-experimental-features = "nix-command flakes";
    allow-import-from-derivation = true;
    extra-substituters = [
      "https://coffeetables.cachix.org"
      "https://timewave.cachix.org"
    ];
    extra-trusted-public-keys = [
      "coffeetables.cachix.org-1:BCQXDtLGFVo/rTG/J4omlyP/jbtNtsZIKHBMTjAWt8g="
      "timewave.cachix.org-1:nu3Uqsm3sikI9xFK3Mt4AD4Q6z+j6eS9+kND1vtznq4="
    ];
  };

  inputs = {
    nixpkgs.url = "nixpkgs/nixos-24.11";
    

    flake-parts.url = "github:hercules-ci/flake-parts";
    devshell.url = "github:numtide/devshell";
    crate2nix.url = "github:timewave-computer/crate2nix";
    sp1-nix.url = "github:timewave-computer/sp1.nix";
    fp-addons.url = "github:timewave-computer/flake-parts-addons";
  };

  outputs = {
    self,
    nixpkgs,
    flake-parts,
    ...
  } @ inputs:
    flake-parts.lib.mkFlake {inherit inputs;} ({lib, moduleWithSystem, ...}: {
      imports = [
        inputs.devshell.flakeModule
        inputs.crate2nix.flakeModule
        inputs.fp-addons.flakeModules.tools
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
        crate2nix = {
          cargoNix = ./Cargo.nix;
          devshell.name = "default";
          profile = "optimized";
          crateOverrides = inputs'.sp1-nix.tools.crateOverrides // {
            valence-coprocessor-app-circuit = attrs: {
              meta.mainProgram = "valence-coprocessor-app-circuit";
            };
            valence-coprocessor-app-controller = attrs: {
              meta.mainProgram = "valence-coprocessor-app-controller";  
            };
          };
        };
      } // (let
        # Common packages used across shells
        commonShellPackages = [
          pkgs.curl
          pkgs.jq
          config.packages.build-app
          config.packages.full-pipeline
        ];
        
        # Common Darwin-specific packages
        commonDarwinPackages = pkgs.lib.optionals pkgs.stdenv.isDarwin [
          pkgs.libiconv
          pkgs.darwin.apple_sdk.frameworks.Security
          pkgs.darwin.apple_sdk.frameworks.CoreFoundation
        ];

        

        # Use sp1-nix for proper SP1 toolchain management  
        sp1-packages = inputs'.sp1-nix.packages;
      in {
        # Create packages for WASM building
        packages = {
          # crate2nix generated packages
          inherit (config.crate2nix.packages) 
            valence-coprocessor-app-circuit
            valence-coprocessor-app-controller;
          
          # Use sp1-nix packages (inherit what's available)
          inherit (sp1-packages) sp1;
          


          # Build app script
          build-app = pkgs.writeShellScriptBin "build-app" ''
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

            # Step 1: cargo-prove is available from sp1-nix, no installation needed
            echo "Using cargo-prove from sp1-nix..."

            # Step 2: Build the app using the nix wasm-shell
            echo "Building app with nightly Rust toolchain..."
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

            # Step 3: Build the SP1 circuit using circuit-shell
            echo "Building SP1 circuit..."
            echo "Using cargo-prove from: $PRJ_ROOT/bin/cargo-prove"
            
            nix develop .#circuit-shell -c bash -c 'pwd; echo "Inside nix develop (circuit-shell): Building SP1 circuit..."; cd "$PRJ_ROOT/crates/circuit" && pwd && echo "Toolchain information from circuit-shell:" && cargo-prove prove --version && cargo-prove prove build --ignore-rust-version; ' || \
            {
              echo "SP1 build failed (executed via circuit-shell), but we'll continue with dev mode"
              echo "SP1 circuit build failed. Will continue with WASM-only deployment (dev mode)."
              echo "WASM build completed successfully!"
              echo ""
              echo "The WASM binary is available at: $PRJ_ROOT/target/wasm32-unknown-unknown/optimized/valence_coprocessor_app_lib.wasm"
            }

            # Check both possible output locations for SP1 binary
            CIRCUIT_PATHS=(
              "$PRJ_ROOT/crates/circuit/elf/valence-coprocessor-app-circuit" # Standard cargo-prove output
              "$PRJ_ROOT/target/sp1/valence-coprocessor-app-circuit" # Alternative target location
              "$PRJ_ROOT/target/sp1/circuit" # Fallback location
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
              ${config.packages.generate-mock-elf}/bin/generate-mock-elf "$PRJ_ROOT/target/sp1/optimized/valence-coprocessor-app-circuit"
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
            ${config.packages.build-app}/bin/build-app

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
            
            # New way: capture output and exit code separately
            DEPLOY_SERVICE_LOG_FILE=$(mktemp)
            echo "Capturing deploy-app output to $DEPLOY_SERVICE_LOG_FILE"
            
            set +e # Allow deploy-app to fail without exiting full-pipeline immediately
            ${config.packages.deploy-app}/bin/deploy-app circuit > "$DEPLOY_SERVICE_LOG_FILE" 2>&1
            DEPLOY_SERVICE_EXIT_CODE=$?
            set -e # Re-enable set -e

            DEPLOY_OUTPUT=$(cat "$DEPLOY_SERVICE_LOG_FILE")
            echo "--- Output from deploy-app (exit code $DEPLOY_SERVICE_EXIT_CODE): ---"
            echo "$DEPLOY_OUTPUT"
            echo "--- End of deploy-app output ---"
            rm "$DEPLOY_SERVICE_LOG_FILE"

            if [ $DEPLOY_SERVICE_EXIT_CODE -ne 0 ]; then
              echo "Error: deploy-app failed with exit code $DEPLOY_SERVICE_EXIT_CODE. See output above."
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
          generate-mock-elf = pkgs.writeShellScriptBin "generate-mock-elf" ''
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

          # Deploy App - mirrors cargo-valence deploy functionality
          deploy-app = pkgs.writeShellScriptBin "deploy-app" ''
            #!/usr/bin/env bash
            # Deploy circuit to Valence coprocessor - mirrors cargo-valence deploy
            
            set -e
            
            # Parse command line arguments
            USE_PUBLIC_SERVICE=false
            CONTROLLER_PATH="./crates/controller"
            CIRCUIT_NAME="valence-coprocessor-app-circuit"
            VERBOSE=false
            DEPLOY_TYPE="circuit"  # Default to circuit
            
            usage() {
              echo "Usage: deploy-app [OPTIONS] [circuit]"
              echo ""
              echo "Options:"
              echo "  --socket <HOST:PORT>    Use public coprocessor service (like prover.timewave.computer:37281)"
              echo "  --controller <PATH>     Path to controller crate (default: ./crates/controller)"
              echo "  --circuit <NAME>        Circuit name (default: valence-coprocessor-app-circuit)"
              echo "  --verbose              Enable verbose output"
              echo "  -h, --help             Show this help message"
              echo ""
              echo "Arguments:"
              echo "  circuit                 Deploy the circuit (default behavior)"
              echo ""
              echo "Examples:"
              echo "  # Deploy to local service (default localhost:37281)"
              echo "  deploy-app circuit"
              echo "  deploy-app              # circuit is implied"
              echo ""
              echo "  # Deploy to public service"
              echo "  deploy-app --socket prover.timewave.computer:37281 circuit"
              echo ""
              echo "  # Deploy with custom controller path"
              echo "  deploy-app --controller ./my-controller --circuit my-circuit circuit"
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
                circuit)
                  DEPLOY_TYPE="circuit"
                  shift
                  ;;
                *)
                  # If it's not an option and not "circuit", it's an error
                  if [[ "$1" != -* ]]; then
                    echo "Error: Unknown argument '$1'. Expected 'circuit' or no argument."
                    usage
                    exit 1
                  else
                    echo "Error: Unknown option '$1'"
                    usage
                    exit 1
                  fi
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
            ${config.packages.build-app}/bin/build-app
            
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
              ${config.packages.generate-mock-elf}/bin/generate-mock-elf "$CIRCUIT_PATH"
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
            echo "Service is available"
            
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
              echo "Deployment successful!"
              echo ""
              # Mirror cargo-valence output format
              echo "{\"controller\": \"$CONTROLLER_ID\", \"circuit\": \"$CIRCUIT_NAME\"}"
              echo ""
              echo "Controller ID: $CONTROLLER_ID"
              echo "Circuit: $CIRCUIT_NAME"
              echo ""
              echo "To generate a proof:"
              if [ "$USE_PUBLIC_SERVICE" = true ]; then
                echo "  prove-app --socket $PUBLIC_SOCKET $CONTROLLER_ID '{\"value\": 42}' \"/proof.bin\""
              else
                echo "  prove-app $CONTROLLER_ID '{\"value\": 42}' \"/proof.bin\""
              fi
            else
              echo "Error: Failed to extract controller ID from response"
              echo "Response: $RESPONSE"
              exit 1
            fi
          '';

          # Derive VK - get verification key for a controller
          derive-vk = pkgs.writeShellScriptBin "derive-vk" ''
            #!/usr/bin/env bash
            # Get verification key from Valence coprocessor
            
            set -e
            
            # Parse command line arguments
            USE_PUBLIC_SERVICE=false
            CONTROLLER_ID=""
            VERBOSE=false
            
            usage() {
              echo "Usage: derive-vk [OPTIONS] <CONTROLLER_ID>"
              echo ""
              echo "Options:"
              echo "  --socket <HOST:PORT>    Use public coprocessor service"
              echo "  --verbose              Enable verbose output"
              echo "  -h, --help             Show this help message"
              echo ""
              echo "Examples:"
              echo "  # Get verification key from local service"
              echo "  derive-vk <CONTROLLER_ID>"
              echo ""
              echo "  # Get verification key from public service"
              echo "  derive-vk --socket prover.timewave.computer:37281 <CONTROLLER_ID>"
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
                *)
                  if [ -z "$CONTROLLER_ID" ]; then
                    CONTROLLER_ID="$1"
                    shift
                  else
                    echo "Error: Unknown option '$1'"
                    usage
                    exit 1
                  fi
                  ;;
              esac
            done
            
            # Check required arguments
            if [ -z "$CONTROLLER_ID" ]; then
              echo "Error: CONTROLLER_ID is required"
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
            
            VK_URL="$SERVICE_HOST/api/registry/controller/$CONTROLLER_ID/vk"
            
            if [ "$VERBOSE" = true ]; then
              echo "Controller ID: $CONTROLLER_ID"
              echo "VK URL: $VK_URL"
            fi
            
            # Check service availability
            echo "Checking service availability..."
            if ! curl -s --connect-timeout 5 "$SERVICE_HOST/api/status" > /dev/null; then
              echo "Error: Cannot connect to coprocessor service at $SERVICE_HOST"
              exit 1
            fi
            
            # Get verification key
            echo "Retrieving verification key..."
            
            TEMP_OUTPUT=$(mktemp)
            
            http_code=$(curl -s -o "$TEMP_OUTPUT" -w "%{http_code}" \
              --connect-timeout 30 -X GET "$VK_URL")
            
            if [ "$http_code" -ne 200 ]; then
              echo "Error: VK request failed with HTTP code $http_code"
              echo "Response:"
              cat "$TEMP_OUTPUT"
              rm "$TEMP_OUTPUT"
              exit 1
            fi
            
            RESPONSE=$(cat "$TEMP_OUTPUT")
            rm "$TEMP_OUTPUT"
            
            echo "Verification key retrieved successfully!"
            echo "$RESPONSE"
          '';

          # Prove App - generate proof for a controller
          prove-app = pkgs.writeShellScriptBin "prove-app" ''
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
              echo "Usage: prove-app [OPTIONS] <CONTROLLER_ID> <JSON_ARGS> <PATH>"
              echo ""
              echo "Options:"
              echo "  --socket <HOST:PORT>    Use public coprocessor service"
              echo "  --verbose              Enable verbose output"
              echo "  -h, --help             Show this help message"
              echo ""
              echo "Arguments:"
              echo "  CONTROLLER_ID           Controller ID to generate proof for"
              echo "  JSON_ARGS              JSON arguments to pass to the controller"
              echo "  PATH                   Path where proof will be stored in virtual filesystem"
              echo ""
              echo "Examples:"
              echo "  # Generate a proof with controller ID"
              echo "  prove-app <CONTROLLER_ID> '{\"value\": 42}' \"/path/in/fs.json\""
              echo ""
              echo "  # Generate a proof using public service"
              echo "  prove-app --socket prover.timewave.computer:37281 <CONTROLLER_ID> '{\"value\": 42}' \"/proof.bin\""
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
                *)
                  if [ -z "$CONTROLLER_ID" ]; then
                    CONTROLLER_ID="$1"
                    shift
                  elif [ -z "$ARGS_JSON" ]; then
                    ARGS_JSON="$1"
                    shift
                  elif [ -z "$PATH_ARG" ]; then
                    PATH_ARG="$1"
                    shift
                  else
                    echo "Error: Unknown option '$1'"
                    usage
                    exit 1
                  fi
                  ;;
              esac
            done
            
            # Check required arguments
            if [ -z "$CONTROLLER_ID" ]; then
              echo "Error: CONTROLLER_ID is required"
              usage
              exit 1
            fi
            
            if [ -z "$ARGS_JSON" ]; then
              echo "Error: JSON_ARGS is required"
              usage
              exit 1
            fi
            
            if [ -z "$PATH_ARG" ]; then
              echo "Error: PATH is required"
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
            
            echo "Proof request submitted successfully!"
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
              echo "Proof retrieved from storage!"
              if [ "$VERBOSE" = true ]; then
                cat "$STORAGE_TEMP_OUTPUT" | jq . 2>/dev/null || cat "$STORAGE_TEMP_OUTPUT"
              else
                echo "Use get-proof to retrieve the full proof data."
              fi
            else
              echo "Warning: Could not retrieve proof from storage (this may be normal for async processing)"
              echo "Use get-proof later to check if the proof is ready."
            fi
            
            rm "$STORAGE_TEMP_OUTPUT"
          '';

          # Get Proof - retrieve proof data from coprocessor storage
          get-proof = pkgs.writeShellScriptBin "get-proof" ''
            #!/usr/bin/env bash
            # Retrieve data from Valence coprocessor storage - mirrors cargo-valence storage
            
            set -e
            
            # Parse command line arguments
            USE_PUBLIC_SERVICE=false
            STORAGE_PATH=""
            CONTROLLER_ID=""
            VERBOSE=false
            
            usage() {
              echo "Usage: get-proof [OPTIONS] <CONTROLLER_ID> <PATH>"
              echo ""
              echo "Options:"
              echo "  --socket <HOST:PORT>        Use public coprocessor service"
              echo "  --verbose                   Enable verbose output"
              echo "  -h, --help                  Show this help message"
              echo ""
              echo "Arguments:"
              echo "  CONTROLLER_ID               Controller ID to retrieve proof from"
              echo "  PATH                       Path to the proof file in virtual filesystem"
              echo ""
              echo "Examples:"
              echo "  # Retrieve a proof from local service"
              echo "  get-proof <CONTROLLER_ID> PROOF.BIN"
              echo ""
              echo "  # Retrieve a proof from public service"
              echo "  get-proof --socket prover.timewave.computer:37281 <CONTROLLER_ID> PROOF.BIN"
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
                *)
                  if [ -z "$CONTROLLER_ID" ]; then
                    CONTROLLER_ID="$1"
                    shift
                  elif [ -z "$STORAGE_PATH" ]; then
                    STORAGE_PATH="$1"
                    shift
                  else
                    echo "Error: Unknown option '$1'"
                    usage
                    exit 1
                  fi
                  ;;
              esac
            done
            
            # Check required arguments
            if [ -z "$CONTROLLER_ID" ]; then
              echo "Error: CONTROLLER_ID is required"
              usage
              exit 1
            fi
            
            if [ -z "$STORAGE_PATH" ]; then
              echo "Error: PATH is required"
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
            
            # Retrieve file from filesystem
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
                echo "File retrieved successfully!"
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
          '';
        };

        # WASM development shell - including our new scripts
        devshells.wasm-shell = {
          name = "wasm-shell";
          packages = [
            pkgs.rustc
            pkgs.wasm-bindgen-cli  
            sp1-packages.sp1 or pkgs.rustc
          ] ++ commonShellPackages ++ commonDarwinPackages;
          
          # Add environment variables - don't use RUSTFLAGS here
          env = [
            {
              name = "CC";
              value = "${pkgs.clang}/bin/clang";
            }
          ] ++ lib.optionals pkgs.stdenv.isDarwin [
            {
              name = "LIBRARY_PATH";
              prefix = "${pkgs.darwin.libiconv}/lib:${pkgs.libiconv}/lib";
            }
            {
              name = "MACOS_DEPLOYMENT_TARGET";
              value = "10.03";
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
        
        # Circuit development shell
        devshells.circuit-shell = {
          name = "circuit-shell";
          packages = [
            sp1-packages.sp1 or pkgs.rustc
            pkgs.llvmPackages.clang
            pkgs.llvmPackages.llvm
          ] ++ commonShellPackages ++ commonDarwinPackages;
          
          # Add environment variables
          env = [
            {
              name = "CC";
              value = "${pkgs.clang}/bin/clang";
            }
          ] ++ lib.optionals pkgs.stdenv.isDarwin [
            {
              name = "LIBRARY_PATH";
              prefix = "${pkgs.darwin.libiconv}/lib:${pkgs.libiconv}/lib";
            }
            {
              name = "MACOS_DEPLOYMENT_TARGET";
              value = "10.03";
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
              help = "Build an SP1 circuit";
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
          
          # Add SP1 setup
          bash.extra = ''
            # Ensure PRJ_ROOT is available inside the shell
            if [ -z "$PRJ_ROOT" ]; then
              export PRJ_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || echo "$PWD")"
            fi
            
            echo "--- circuit-shell (sp1-nix) ---"
            echo "SP1 tooling available from sp1-nix"
            echo "Ready for SP1 circuit development"
          '';
        };
        
        # Default development shell with access to all scripts
        devshells.default = {
          packages = commonShellPackages ++ [
            sp1-packages.sp1 or pkgs.rustc
          ];
          
          commands = [
            {
              category = "build";
              name = "build-app-cmd";
              help = "Build the app";
              command = "build-app";
            }
            {
              category = "pipeline";
              name = "full-pipeline-cmd";
              help = "Run the complete pipeline (build, deploy, proof)";
              command = "full-pipeline";
            }
            {
              category = "shells";
              name = "circuit";
              help = "Enter the circuit development shell";
              command = "nix develop .#circuit-shell";
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
            echo "  build-app           - Build the app"
            echo "  deploy-app          - Deploy to the coprocessor service"
            echo "  full-pipeline       - Run complete pipeline (build, deploy, prove)"
          '';
        };

        # Set up apps that can be run with 'nix run'
        apps = {
          build-app = {
            type = "app";
            program = "${config.packages.build-app}/bin/build-app";
          };
          
          
          full-pipeline = {
            type = "app";
            program = "${config.packages.full-pipeline}/bin/full-pipeline";
          };
          
          generate-mock-elf = {
            type = "app";
            program = "${config.packages.generate-mock-elf}/bin/generate-mock-elf";
          };
          
          # New Valence commands that mirror cargo-valence functionality
          deploy-app = {
            type = "app";
            program = "${config.packages.deploy-app}/bin/deploy-app";
          };
          
          derive-vk = {
            type = "app";
            program = "${config.packages.derive-vk}/bin/derive-vk";
          };
          
          prove-app = {
            type = "app";
            program = "${config.packages.prove-app}/bin/prove-app";
          };
          
          get-proof = {
            type = "app";
            program = "${config.packages.get-proof}/bin/get-proof";
          };
        };
      });
    });
}