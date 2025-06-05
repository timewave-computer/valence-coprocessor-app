#!/usr/bin/env bash

# Run the production SP1 proving flow e2e test
# This script validates the complete pipeline from Skip API to SP1 proof generation

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo "Valence Coprocessor Production E2E Test"
echo "=========================================="
echo

# Fix SOURCE_DATE_EPOCH for ring crate compilation
export SOURCE_DATE_EPOCH=$(date +%s)

# Default configuration
COPROCESSOR_URL=${COPROCESSOR_URL:-"http://localhost:37281"}
CONTROLLER_ID=${CONTROLLER_ID:-"2a326a320c2a4269241d2f39a6c8e253ae14b9bccb5e7f141d9d1e4223e485bb"}
EXPECTED_DESTINATION=${EXPECTED_DESTINATION:-"cosmos1zxj6y5h3r8k9v7n2m4l1q8w5e3t6y9u0i7o4p2s5d8f6g3h1j4k7l9n2"}

# Export for test usage
export COPROCESSOR_URL
export CONTROLLER_ID
export EXPECTED_DESTINATION

# Check if we're in the right directory
if [ ! -f "flake.nix" ]; then
    echo "Error: Must be run from the project root directory"
    echo "   Current directory: $(pwd)"
    echo "   Expected to find: flake.nix"
    exit 1
fi

# Check if coprocessor service is running
echo "Checking coprocessor service availability..."
if curl -s http://localhost:37281/api/stats > /dev/null 2>&1; then
    echo "Coprocessor service is running"
else
    echo "Coprocessor service not detected on localhost:37281"
    echo "   Starting coprocessor service..."
    
    # Start the service in background
    nix develop --command valence-coprocessor start --coprocessor-path ./valence-coprocessor-service-0.1.0-x86_64-apple-darwin.tar.gz > /tmp/coprocessor-e2e.log 2>&1 &
    
    # Wait for service to start
    echo "   Waiting for service to start..."
    sleep 5
    
    # Check again
    if curl -s http://localhost:37281/api/stats > /dev/null 2>&1; then
        echo "   Coprocessor service started successfully"
    else
        echo "   Failed to start coprocessor service"
        echo "   Check logs: /tmp/coprocessor-e2e.log"
        exit 1
    fi
fi

# Check if controller is deployed
echo "Checking controller deployment..."

# Try a simple dev call to verify controller
if curl -s -X POST "http://localhost:37281/api/registry/controller/${CONTROLLER_ID}/dev" \
   -H "Content-Type: application/json" \
   -d '{"args":{"payload":{"cmd":"validate","destination":"test","memo":"","path":"/tmp/test.json","skip_response":{"operations":[],"estimated_fees":[]}}}}' \
   > /dev/null 2>&1; then
    echo "Controller is deployed and accessible"
else
    echo "Controller not detected, attempting deployment..."
    
    # Build and deploy
    echo "   Building WASM..."
    nix develop --command build-wasm
    
    echo "   Deploying controller..."
    nix develop --command deploy-to-service
    
    echo "Controller deployment completed"
fi

echo
echo "Test Configuration:"
echo "   Coprocessor URL: $COPROCESSOR_URL"
echo "   Controller ID: $CONTROLLER_ID"
echo "   Expected Destination: $EXPECTED_DESTINATION"
echo

# Run the test
echo "Running production SP1 proving flow test..."
echo "   This may take 120+ seconds for SP1 proof generation..."
echo

cd e2e
if nix develop --command cargo run --bin run_production_test; then
    echo
    echo "Production E2E Test PASSED!"
    echo "The valence-coprocessor-app is production-ready!"
    echo
    echo "Next steps:"
    echo "   1. Integration with Valence smart contracts"
    echo "   2. Production deployment on mainnet"
    echo "   3. Real token transfer execution with SP1 proofs"
else
    echo
    echo "Production E2E Test FAILED!"
    echo "Troubleshooting tips:"
    echo "   1. Check coprocessor service logs: /tmp/coprocessor-e2e.log"
    echo "   2. Verify internet connection for Skip API"
    echo "   3. Ensure SP1 prover is working correctly"
    echo "   4. Check that all dependencies are properly installed"
    exit 1
fi 