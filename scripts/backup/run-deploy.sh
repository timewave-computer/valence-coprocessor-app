#!/usr/bin/env bash
# Wrapper script to replace `cargo run --manifest-path script/Cargo.toml -- deploy`

set -e

PRJ_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# Display info about the script
echo "Valence Coprocessor App Deployment"
echo "=================================="
echo "Running deploy script to build WASM binary"
echo ""

# Call the deploy.sh script
"${PRJ_ROOT}/scripts/deploy.sh"

echo ""
echo "You can access the wasm binary at: ${PRJ_ROOT}/target/wasm32-unknown-unknown/optimized/valence_coprocessor_app_lib.wasm"
echo ""
echo "To verify: nix develop .#wasm-shell -c wasm-objdump -x target/wasm32-unknown-unknown/optimized/valence_coprocessor_app_lib.wasm" 