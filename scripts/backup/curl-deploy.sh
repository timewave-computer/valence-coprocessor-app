#!/usr/bin/env bash
# Deploy WASM binary directly to the co-processor service using curl

set -e

PRJ_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WASM_PATH="${PRJ_ROOT}/target/wasm32-unknown-unknown/optimized/valence_coprocessor_app_lib.wasm"
SERVICE_URL="http://localhost:37281/api/registry/program"

# Ensure the WASM binary exists
if [ ! -f "$WASM_PATH" ]; then
  echo "Error: WASM binary not found at $WASM_PATH"
  echo "Please run ./scripts/run-deploy.sh first to build the WASM binary"
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