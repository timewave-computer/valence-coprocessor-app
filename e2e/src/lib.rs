//! End-to-end tests for LBTC IBC Eureka transfer system
//!
//! Tests the complete flow:
//! - Skip API RPC for route discovery and fee information
//! - Coprocessor RPC for ZK proof generation  
//! - Ethereum RPC for transaction validation

pub mod clients;
pub mod constants;
pub mod tests;
pub mod utils;

pub use constants::*;

use anyhow::Result;
use std::time::Duration;
use tracing::info;

/// Initialize logging for e2e tests
pub fn init_logging() {
    tracing_subscriber::fmt().with_test_writer().try_init().ok();
}

/// Test configuration for e2e runs
#[derive(Debug, Clone)]
pub struct E2EConfig {
    /// Ethereum RPC URL (real mainnet or testnet)
    pub ethereum_rpc_url: String,
    /// Coprocessor service URL (public or local)
    pub coprocessor_url: String,
    /// Mnemonic for signing (test mnemonic only)
    pub mnemonic: String,
    /// Skip API key if required
    pub skip_api_key: Option<String>,
    /// Test environment (mainnet, testnet, local)
    pub environment: Environment,
}

/// Test environment type
#[derive(Debug, Clone, PartialEq)]
pub enum Environment {
    Local,   // Local anvil + local coprocessor
    Testnet, // Ethereum testnet + public coprocessor
    Mainnet, // Ethereum mainnet + public coprocessor (read-only)
}

impl E2EConfig {
    /// Create configuration from environment variables
    pub fn from_env() -> Result<Self> {
        let ethereum_rpc_url = std::env::var(ENV_ETHEREUM_RPC_URL)
            .unwrap_or_else(|_| LOCAL_ETHEREUM_RPC_URL.to_string());

        let coprocessor_url = std::env::var(ENV_COPROCESSOR_URL)
            .unwrap_or_else(|_| PUBLIC_COPROCESSOR_URL.to_string());

        let mnemonic = std::env::var(ENV_MNEMONIC).unwrap_or_else(|_| TEST_MNEMONIC.to_string());

        let skip_api_key = std::env::var(ENV_SKIP_API_KEY).ok();

        // Determine environment based on URLs
        let environment =
            if ethereum_rpc_url.contains("127.0.0.1") || ethereum_rpc_url.contains("localhost") {
                Environment::Local
            } else if ethereum_rpc_url.contains("sepolia") || ethereum_rpc_url.contains("holesky") {
                Environment::Testnet
            } else {
                Environment::Mainnet
            };

        info!(
            "E2E Config: environment={:?}, eth_rpc={}, coprocessor={}",
            environment, ethereum_rpc_url, coprocessor_url
        );

        Ok(Self {
            ethereum_rpc_url,
            coprocessor_url,
            mnemonic,
            skip_api_key,
            environment,
        })
    }

    /// Create local development configuration
    pub fn local() -> Self {
        Self {
            ethereum_rpc_url: LOCAL_ETHEREUM_RPC_URL.to_string(),
            coprocessor_url: LOCAL_COPROCESSOR_URL.to_string(),
            mnemonic: TEST_MNEMONIC.to_string(),
            skip_api_key: None,
            environment: Environment::Local,
        }
    }

    /// Create mainnet configuration (read-only for validation)
    pub fn mainnet(ethereum_rpc_url: String) -> Self {
        Self {
            ethereum_rpc_url,
            coprocessor_url: PUBLIC_COPROCESSOR_URL.to_string(),
            mnemonic: TEST_MNEMONIC.to_string(), // Not used for read-only operations
            skip_api_key: None,
            environment: Environment::Mainnet,
        }
    }
}

/// Utility function to test RPC connectivity
pub async fn test_rpc_connectivity(config: &E2EConfig) -> Result<()> {
    info!(
        "Testing RPC connectivity for environment: {:?}",
        config.environment
    );

    // Test Ethereum RPC connectivity
    let eth_client = reqwest::Client::new();
    let eth_payload = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "eth_blockNumber",
        "params": [],
        "id": 1
    });

    let eth_response = eth_client
        .post(&config.ethereum_rpc_url)
        .json(&eth_payload)
        .timeout(Duration::from_secs(MAX_API_RESPONSE_TIME_SECONDS))
        .send()
        .await?;

    if !eth_response.status().is_success() {
        return Err(anyhow::anyhow!(
            "Ethereum RPC not accessible: {}",
            eth_response.status()
        ));
    }

    let eth_result: serde_json::Value = eth_response.json().await?;
    let block_number = eth_result["result"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Invalid block number response"))?;

    info!("Ethereum RPC connected, latest block: {}", block_number);

    // Test coprocessor connectivity
    let coprocessor_client = reqwest::Client::new();
    let health_url = format!("{}/health", config.coprocessor_url);

    let coprocessor_response = coprocessor_client
        .get(&health_url)
        .timeout(Duration::from_secs(MAX_API_RESPONSE_TIME_SECONDS))
        .send()
        .await?;

    if !coprocessor_response.status().is_success() {
        return Err(anyhow::anyhow!(
            "Coprocessor not accessible: {}",
            coprocessor_response.status()
        ));
    }

    info!("Coprocessor service connected");

    // Test Skip API connectivity
    let skip_client = reqwest::Client::new();
    let skip_url = format!("{}/v2/info/chains", SKIP_API_BASE_URL);

    let skip_response = skip_client
        .get(&skip_url)
        .timeout(Duration::from_secs(MAX_API_RESPONSE_TIME_SECONDS))
        .send()
        .await?;

    if !skip_response.status().is_success() {
        return Err(anyhow::anyhow!(
            "Skip API not accessible: {}",
            skip_response.status()
        ));
    }

    info!("Skip API connected");

    Ok(())
}

/// Validate that all required constants are properly set
pub fn validate_constants() -> Result<()> {
    // Validate Ethereum address format
    if !TOKEN_CONTRACT_ADDRESS.starts_with("0x") || TOKEN_CONTRACT_ADDRESS.len() != 42 {
        return Err(anyhow::anyhow!("Invalid LBTC contract address format"));
    }

    if !EXPECTED_ENTRY_CONTRACT.starts_with("0x") || EXPECTED_ENTRY_CONTRACT.len() != 42 {
        return Err(anyhow::anyhow!("Invalid entry contract address format"));
    }

    // Validate Cosmos address format
    if !EXPECTED_DESTINATION.starts_with("cosmos1") {
        return Err(anyhow::anyhow!("Invalid Cosmos destination address format"));
    }

    // Validate fee threshold is reasonable (not zero, not too high)
    if FEE_THRESHOLD_TOKEN_WEI == 0 || FEE_THRESHOLD_TOKEN_WEI > 10_000_000_000_000_000 {
        return Err(anyhow::anyhow!(
            "Invalid fee threshold: {}",
            FEE_THRESHOLD_TOKEN_WEI
        ));
    }

    // Validate route hash format (should be hex string)
    if EXPECTED_ROUTE_HASH.len() != 64 {
        return Err(anyhow::anyhow!(
            "Invalid route hash length: {}",
            EXPECTED_ROUTE_HASH.len()
        ));
    }

    if !EXPECTED_ROUTE_HASH.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(anyhow::anyhow!("Route hash contains non-hex characters"));
    }

    info!("All constants validated successfully");
    Ok(())
}
