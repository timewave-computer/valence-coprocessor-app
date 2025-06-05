# Token Transfer Strategist Configuration

This document describes how to configure the Token Transfer Strategist using environment variables.

## Overview

The strategist reads configuration from environment variables and `.env` files. This allows you to:
- Keep sensitive data (RPC URLs, mnemonics) out of your code
- Use different configurations for local development, testing, and production
- Easily switch between networks and providers

**Public endpoints** like the coprocessor service and Skip API are hardcoded based on the environment and don't need to be configured.

## Environment Variables

### Required Variables

| Variable | Description | Example |
|----------|-------------|---------|
| `STRATEGIST_ENVIRONMENT` | Target environment | `local`, `testnet`, `mainnet` |

### Network Configuration

| Variable | Description | Example |
|----------|-------------|---------|
| `ETHEREUM_RPC_URL` | Ethereum RPC endpoint | `https://mainnet.infura.io/v3/YOUR_KEY` |

### Security Variables

| Variable | Description | Example |
|----------|-------------|---------|
| `WALLET_MNEMONIC` | 12+ word mnemonic for transactions | `word1 word2 ... word12` |

### Optional Variables

| Variable | Description | Example |
|----------|-------------|---------|
| `SKIP_API_KEY` | Premium Skip API features | `your_skip_api_key` |

## Public Endpoints

These services use public endpoints that are automatically configured based on your environment:

| Service | Local | Testnet | Mainnet |
|---------|-------|---------|---------|
| **Coprocessor** | `http://localhost:37281` | `https://coprocessor-testnet.timewave.computer` | `https://coprocessor.timewave.computer` |
| **Skip API** | `https://api.skip.build` | `https://api.skip.build` | `https://api.skip.build` |

## Environment-Specific Configuration

### Local Development
```env
STRATEGIST_ENVIRONMENT=local
ETHEREUM_RPC_URL=http://localhost:8545
```

**Note:** For local development, a test mnemonic is used automatically. The coprocessor expects to find a local service at `localhost:37281`.

### Testnet
```env
STRATEGIST_ENVIRONMENT=testnet
ETHEREUM_RPC_URL=https://sepolia.infura.io/v3/your_infura_project_id
WALLET_MNEMONIC=your testnet mnemonic here
```

### Mainnet
```env
STRATEGIST_ENVIRONMENT=mainnet
ETHEREUM_RPC_URL=https://mainnet.infura.io/v3/your_infura_project_id
WALLET_MNEMONIC=your mainnet mnemonic here
SKIP_API_KEY=your_premium_skip_api_key
```

## RPC Provider Setup

### Infura
1. Sign up at [infura.io](https://infura.io/)
2. Create a new project
3. Use the endpoint URL in `ETHEREUM_RPC_URL`
   - Mainnet: `https://mainnet.infura.io/v3/YOUR_PROJECT_ID`
   - Sepolia: `https://sepolia.infura.io/v3/YOUR_PROJECT_ID`

### Alchemy
1. Sign up at [alchemy.com](https://www.alchemy.com/)
2. Create an app for Ethereum Mainnet or Sepolia Testnet
3. Use the endpoint URL in `ETHEREUM_RPC_URL`
   - Mainnet: `https://eth-mainnet.g.alchemy.com/v2/YOUR_API_KEY`
   - Sepolia: `https://eth-sepolia.g.alchemy.com/v2/YOUR_API_KEY`

### Other Providers
You can use any Ethereum RPC provider by setting `ETHEREUM_RPC_URL` to their endpoint.

### Skip API (Optional Premium Features)
1. Contact Skip Protocol for premium API access
2. Add your API key to `SKIP_API_KEY`

## Security Best Practices

### ⚠️ **NEVER commit sensitive data to version control!**

1. **Always use `.env` files for local development:**
   - Add `.env` to your `.gitignore`
   - Use `env.example` for sharing configuration templates

2. **For production deployment:**
   - Set environment variables directly in your deployment environment
   - Use secret management services (AWS Secrets Manager, HashiCorp Vault, etc.)
   - Consider using encrypted environment files

3. **Mnemonic security:**
   - Use dedicated wallets for different environments
   - Never use mainnet mnemonics in development
   - Consider hardware wallets for production

## Usage Examples

### Basic Usage
```rust
use strategist::TokenTransferStrategist;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration from environment
    let strategist = TokenTransferStrategist::from_env()?;
    
    // Use the strategist...
    Ok(())
}
```

### Custom Configuration
```rust
use strategist::{TokenTransferStrategist, StrategistConfig, Environment};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create custom configuration
    let config = StrategistConfig {
        ethereum_rpc_url: "https://mainnet.infura.io/v3/YOUR_KEY".to_string(),
        skip_api_key: None,
        mnemonic: "your twelve word mnemonic...".to_string(),
        environment: Environment::Mainnet,
    };
    
    let strategist = TokenTransferStrategist::new(config)?;
    
    // Use the strategist...
    Ok(())
}
```

### Environment Detection
```rust
use strategist::StrategistConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = StrategistConfig::from_env()?;
    
    match config.environment {
        strategist::Environment::Local => {
            println!("Running in local development mode");
            println!("Coprocessor: {}", config.coprocessor_url());
        }
        strategist::Environment::Testnet => {
            println!("Running on testnet");
            println!("Coprocessor: {}", config.coprocessor_url());
        }
        strategist::Environment::Mainnet => {
            println!("Running on mainnet");
            println!("Coprocessor: {}", config.coprocessor_url());
        }
    }
    
    Ok(())
}
```

## Troubleshooting

### Common Issues

1. **"WALLET_MNEMONIC environment variable is required"**
   - Set `WALLET_MNEMONIC` for testnet/mainnet environments
   - For local development, this should be automatic

2. **"For testnet/mainnet environment, you must provide ETHEREUM_RPC_URL"**
   - Add `ETHEREUM_RPC_URL` with your Ethereum RPC endpoint

3. **"Invalid mnemonic must contain at least 12 words"**
   - Ensure your mnemonic has 12 or more space-separated words
   - Common test mnemonic: `abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about`

4. **Connection errors to coprocessor service**
   - For local: ensure coprocessor service is running on port 37281
   - For testnet/mainnet: check network connectivity to timewave.computer

### Validation

You can validate your configuration:

```rust
use strategist::StrategistConfig;

let config = StrategistConfig::from_env()?;
config.validate()?; // Will return Err if configuration is invalid
println!("Configuration is valid!");
``` 