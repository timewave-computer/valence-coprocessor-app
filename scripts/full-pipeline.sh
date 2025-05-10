#!/usr/bin/env bash
# Complete pipeline script for Valence coprocessor app
# Builds WASM, deploys to service, and attempts to generate/verify proof

set -e

PRJ_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

echo "=========================================="
echo "Valence Coprocessor App - Complete Pipeline"
echo "=========================================="

# Step 1: Build the WASM binary
echo ""
echo "Step 1: Building WASM binary..."
"$PRJ_ROOT/scripts/build-wasm.sh"

# Step 2: Deploy to the co-processor service
echo ""
echo "Step 2: Deploying to co-processor service..."
DEPLOY_OUTPUT=$("$PRJ_ROOT/scripts/deploy-to-service.sh")
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