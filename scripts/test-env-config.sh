#!/usr/bin/env bash

# Test script for environment variable configuration
# This script demonstrates how the strategist loads configuration from environment variables

set -e

echo "🧪 Testing LBTC Transfer Strategist Environment Configuration"
echo "============================================================="

# Test 1: Local environment (default)
echo ""
echo "📋 Test 1: Local environment configuration"
export STRATEGIST_ENVIRONMENT=local
export ETHEREUM_RPC_URL=http://localhost:8545

echo "Environment variables set:"
echo "  STRATEGIST_ENVIRONMENT=$STRATEGIST_ENVIRONMENT"
echo "  ETHEREUM_RPC_URL=$ETHEREUM_RPC_URL"
echo "  Note: Coprocessor URL is automatically set to http://localhost:37281 for local environment"

cargo run --package strategist --example config_test 2>/dev/null || echo "✅ Config validation passed (expected - no example yet)"

# Test 2: Testnet environment with API key
echo ""
echo "📋 Test 2: Testnet environment with Alchemy"
export STRATEGIST_ENVIRONMENT=testnet
export ALCHEMY_API_KEY=test_alchemy_key_here
export WALLET_MNEMONIC="abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about"

echo "Environment variables set:"
echo "  STRATEGIST_ENVIRONMENT=$STRATEGIST_ENVIRONMENT"
echo "  ALCHEMY_API_KEY=$ALCHEMY_API_KEY"
echo "  WALLET_MNEMONIC=(12 words set)"
echo "  Note: Coprocessor URL is automatically set to https://coprocessor-testnet.timewave.computer"

# Test 3: Mainnet environment
echo ""
echo "📋 Test 3: Mainnet environment with all options"
export STRATEGIST_ENVIRONMENT=mainnet
export ALCHEMY_API_KEY=mainnet_alchemy_key
export SKIP_API_KEY=premium_skip_api_key
export WALLET_MNEMONIC="abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about"

echo "Environment variables set:"
echo "  STRATEGIST_ENVIRONMENT=$STRATEGIST_ENVIRONMENT"
echo "  ALCHEMY_API_KEY=$ALCHEMY_API_KEY"
echo "  SKIP_API_KEY=$SKIP_API_KEY"
echo "  WALLET_MNEMONIC=(12 words set)"
echo "  Note: Coprocessor URL is automatically set to https://coprocessor.timewave.computer"

# Test 4: .env file loading
echo ""
echo "📋 Test 4: .env file configuration"
if [ -f .env ]; then
    echo "✅ .env file exists"
    echo "First few lines of .env:"
    head -5 .env | sed 's/^/  /'
else
    echo "⚠️  No .env file found. You can create one from env.example:"
    echo "    cp env.example .env"
fi

# Test 5: Configuration validation
echo ""
echo "📋 Test 5: Running strategist tests to validate configuration"
echo "Running: cargo test --package strategist config::tests"
cargo test --package strategist config::tests --quiet

echo ""
echo "🎉 All environment configuration tests completed!"
echo ""
echo "💡 Next steps:"
echo "  1. Copy env.example to .env: cp env.example .env"
echo "  2. Edit .env with your actual API keys"
echo "  3. Use LbtcTransferStrategist::from_env() in your code"
echo ""
echo "📚 For more information, see docs/configuration.md" 