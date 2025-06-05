#!/usr/bin/env bash

# Setup script for Token Transfer Strategist environment configuration
# This script helps users quickly set up their .env file

set -e

echo "🚀 Token Transfer Strategist Environment Setup"
echo "============================================"

# Check if .env already exists
if [ -f .env ]; then
    echo ""
    echo "⚠️  .env file already exists!"
    echo "Current .env file:"
    echo "---"
    head -10 .env | sed 's/^/  /'
    echo "---"
    echo ""
    read -p "Do you want to overwrite it? (y/N): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo "Setup cancelled. Existing .env file preserved."
        exit 0
    fi
fi

# Ask user which template to use
echo ""
echo "Choose your setup option:"
echo "1. Quick setup (env.example - minimal configuration)"
echo "2. Custom setup (manual configuration)"
echo ""
read -p "Enter your choice (1-2): " -n 1 -r
echo

case $REPLY in
    1)
        echo ""
        echo "📋 Setting up with quick template (env.example)..."
        cp env.example .env
        echo "✅ Copied env.example to .env"
        ;;
    2)
        echo ""
        echo "📋 Creating custom .env file..."
        cat > .env << 'EOF'
# Token Transfer Strategist Configuration
# Custom setup - add your configuration here

STRATEGIST_ENVIRONMENT=local
ETHEREUM_RPC_URL=http://localhost:8545
WALLET_MNEMONIC=abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about

# Add your API keys here:
# SKIP_API_KEY=your_skip_api_key_here
EOF
        echo "✅ Created basic .env file"
        ;;
    *)
        echo "Invalid choice. Exiting."
        exit 1
        ;;
esac

# Provide next steps
echo ""
echo "🎉 Environment setup complete!"
echo ""
echo "📝 Next steps:"
echo "  1. Edit .env with your actual values:"
echo "     nano .env    # or use your preferred editor"
echo ""
echo "  2. For testnet/mainnet, set your Ethereum RPC URL:"
echo "     - Infura: ETHEREUM_RPC_URL=https://mainnet.infura.io/v3/YOUR_PROJECT_ID"
echo "     - Alchemy: ETHEREUM_RPC_URL=https://eth-mainnet.g.alchemy.com/v2/YOUR_API_KEY"
echo "     - Or any other Ethereum RPC provider"
echo ""
echo "  3. For testnet/mainnet, set your mnemonic:"
echo "     - Add: WALLET_MNEMONIC=your twelve word mnemonic phrase"
echo "     - ⚠️  NEVER commit real mnemonics to git!"
echo ""
echo "  4. Test your configuration:"
echo "     cargo test --package strategist config::tests"
echo ""
echo "📚 For more information:"
echo "  - Complete documentation: docs/configuration.md"
echo "  - Test configuration: ./scripts/test-env-config.sh"
echo ""
echo "🔒 Security reminder:"
echo "  - .env files should NEVER be committed to version control"
echo "  - Add .env to your .gitignore if it's not already there"
echo "  - Use different wallets for different environments"

# Check if .gitignore exists and contains .env
if [ -f .gitignore ]; then
    if ! grep -q "^\.env$" .gitignore; then
        echo ""
        read -p "Add .env to .gitignore? (Y/n): " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Nn]$ ]]; then
            echo ".env" >> .gitignore
            echo "✅ Added .env to .gitignore"
        fi
    else
        echo ""
        echo "✅ .env is already in .gitignore"
    fi
else
    echo ""
    read -p "Create .gitignore with .env entry? (Y/n): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Nn]$ ]]; then
        echo ".env" > .gitignore
        echo "✅ Created .gitignore with .env entry"
    fi
fi

echo ""
echo "🎯 Setup complete! You're ready to use environment-based configuration." 