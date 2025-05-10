#!/usr/bin/env bash
# Deployment script for Valence coprocessor

set -e

PRJ_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# Ensure proper directory structure
mkdir -p "${PRJ_ROOT}/bin"
mkdir -p "${PRJ_ROOT}/target/wasm32-unknown-unknown/release" 
mkdir -p "${PRJ_ROOT}/target/wasm32-unknown-unknown/optimized"

# Step 1: Install cargo-prove if needed (though we won't use it now)
if [ ! -f "${PRJ_ROOT}/bin/cargo-prove" ]; then
  echo "Installing cargo-prove..."
  "${PRJ_ROOT}/scripts/install-cargo-prove.sh"
fi

# Step 2: Build the WASM binary using the nix wasm-shell
echo "Building WASM with nightly Rust toolchain..."
nix develop .#wasm-shell -c bash -c 'export RUSTFLAGS="--cfg=web_sys_unstable_apis"; cargo build --target wasm32-unknown-unknown --release -p valence-coprocessor-app-lib'

# Copy the WASM to the expected location if it was built
if [ -f "${PRJ_ROOT}/target/wasm32-unknown-unknown/release/valence_coprocessor_app_lib.wasm" ]; then
  echo "Copying WASM binary to optimized directory..."
  cp "${PRJ_ROOT}/target/wasm32-unknown-unknown/release/valence_coprocessor_app_lib.wasm" "${PRJ_ROOT}/target/wasm32-unknown-unknown/optimized/"
else
  echo "WASM binary not found! Build failed."
  exit 1
fi

echo "WASM build completed successfully!"
echo ""
echo "Note: SP1 circuit building is currently disabled due to toolchain issues."
echo "The WASM binary is available at: ${PRJ_ROOT}/target/wasm32-unknown-unknown/optimized/valence_coprocessor_app_lib.wasm" 