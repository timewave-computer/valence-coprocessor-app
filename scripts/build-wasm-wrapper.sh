#!/usr/bin/env bash
# Simple wrapper script to build the WASM binary

set -e

PRJ_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# Display info about the script
echo "Valence Coprocessor App WASM Build"
echo "=================================="
echo "Building WASM binary for the Valence coprocessor"
echo ""

# Call the build-wasm.sh script
"${PRJ_ROOT}/scripts/build-wasm.sh"

echo ""
echo "You can access the wasm binary at: ${PRJ_ROOT}/target/wasm32-unknown-unknown/optimized/valence_coprocessor_app_lib.wasm"
echo ""
echo "To verify: nix develop .#wasm-shell -c file target/wasm32-unknown-unknown/optimized/valence_coprocessor_app_lib.wasm" 