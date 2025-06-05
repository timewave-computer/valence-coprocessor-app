//! Configuration management for the token transfer strategist
//! 
//! Reads sensitive configuration like RPC URLs and API keys from environment variables.
//! Uses .env file support for local development.

use anyhow::{Result, anyhow};
use std::env;

/// Configuration for the token transfer strategist
#[derive(Debug, Clone)]
pub struct StrategistConfig {
    /// Ethereum RPC URL (with API key if needed)
    pub ethereum_rpc_url: String,
    /// Optional Skip API key for authenticated requests
    pub skip_api_key: Option<String>,
    /// Wallet mnemonic for Ethereum transactions
    pub mnemonic: String,
    /// Environment (local, testnet, mainnet)
    pub environment: Environment,
}

/// Environment types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Environment {
    Local,
    Testnet,
    Mainnet,
}

impl std::str::FromStr for Environment {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "local" => Ok(Environment::Local),
            "testnet" => Ok(Environment::Testnet),
            "mainnet" => Ok(Environment::Mainnet),
            _ => Err(anyhow!("Invalid environment: {}. Must be 'local', 'testnet', or 'mainnet'", s)),
        }
    }
}

impl StrategistConfig {
    /// Load configuration from environment variables
    /// 
    /// This will first try to load from a .env file if it exists,
    /// then read from environment variables.
    pub fn from_env() -> Result<Self> {
        // Try to load .env file (ignore if it doesn't exist)
        let _ = dotenvy::dotenv();

        let environment = env::var("STRATEGIST_ENVIRONMENT")
            .unwrap_or_else(|_| "local".to_string())
            .parse::<Environment>()?;

        // Get Ethereum RPC URL
        let ethereum_rpc_url = Self::build_ethereum_rpc_url(&environment)?;

        // Get Skip API configuration
        let skip_api_key = env::var("SKIP_API_KEY").ok();

        // Get wallet mnemonic
        let mnemonic = env::var("WALLET_MNEMONIC")
            .unwrap_or_else(|_| {
                if environment == Environment::Local {
                    "test test test test test test test test test test test junk".to_string()
                } else {
                    panic!("WALLET_MNEMONIC environment variable is required for non-local environments")
                }
            });

        Ok(Self {
            ethereum_rpc_url,
            skip_api_key,
            mnemonic,
            environment,
        })
    }

    /// Get coprocessor URL based on environment
    pub fn coprocessor_url(&self) -> String {
        match self.environment {
            Environment::Local => "http://localhost:37281".to_string(),
            Environment::Testnet => "https://coprocessor-testnet.timewave.computer".to_string(),
            Environment::Mainnet => "https://coprocessor.timewave.computer".to_string(),
        }
    }

    /// Get Skip API base URL (always the same public endpoint)
    pub fn skip_api_base_url(&self) -> String {
        "https://api.skip.build".to_string()
    }

    /// Build Ethereum RPC URL based on environment
    fn build_ethereum_rpc_url(environment: &Environment) -> Result<String> {
        match environment {
            Environment::Local => {
                // For local development, use a local node or custom RPC
                Ok(env::var("ETHEREUM_RPC_URL")
                    .unwrap_or_else(|_| "http://localhost:8545".to_string()))
            }
            Environment::Testnet | Environment::Mainnet => {
                // For testnet and mainnet, require custom RPC URL
                env::var("ETHEREUM_RPC_URL")
                    .map_err(|_| anyhow!("For {} environment, you must provide ETHEREUM_RPC_URL", 
                        match environment {
                            Environment::Testnet => "testnet",
                            Environment::Mainnet => "mainnet", 
                            Environment::Local => unreachable!(),
                        }))
            }
        }
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<()> {
        // Validate URLs
        if !self.ethereum_rpc_url.starts_with("http") {
            return Err(anyhow!("Invalid Ethereum RPC URL: {}", self.ethereum_rpc_url));
        }

        // Validate mnemonic (basic check)
        if self.mnemonic.split_whitespace().count() < 12 {
            return Err(anyhow!("Mnemonic must contain at least 12 words"));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_environment_parsing() {
        assert_eq!("local".parse::<Environment>().unwrap(), Environment::Local);
        assert_eq!("testnet".parse::<Environment>().unwrap(), Environment::Testnet);
        assert_eq!("mainnet".parse::<Environment>().unwrap(), Environment::Mainnet);
        assert!("invalid".parse::<Environment>().is_err());
    }

    #[test]
    fn test_config_validation() {
        let mut config = StrategistConfig {
            ethereum_rpc_url: "http://localhost:8545".to_string(),
            skip_api_key: None,
            mnemonic: "test test test test test test test test test test test junk".to_string(),
            environment: Environment::Local,
        };

        assert!(config.validate().is_ok());

        // Test invalid mnemonic
        config.mnemonic = "too short".to_string();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_coprocessor_urls() {
        let config = StrategistConfig {
            ethereum_rpc_url: "http://localhost:8545".to_string(),
            skip_api_key: None,
            mnemonic: "test test test test test test test test test test test junk".to_string(),
            environment: Environment::Local,
        };

        assert_eq!(config.coprocessor_url(), "http://localhost:37281");

        let testnet_config = StrategistConfig {
            environment: Environment::Testnet,
            ..config.clone()
        };
        assert_eq!(testnet_config.coprocessor_url(), "https://coprocessor-testnet.timewave.computer");

        let mainnet_config = StrategistConfig {
            environment: Environment::Mainnet,
            ..config
        };
        assert_eq!(mainnet_config.coprocessor_url(), "https://coprocessor.timewave.computer");
    }

    #[test]
    fn test_skip_api_url() {
        let config = StrategistConfig {
            ethereum_rpc_url: "http://localhost:8545".to_string(),
            skip_api_key: None,
            mnemonic: "test test test test test test test test test test test junk".to_string(),
            environment: Environment::Local,
        };

        assert_eq!(config.skip_api_base_url(), "https://api.skip.build");
    }
} 