//! Client wrappers for e2e testing with real RPC calls

use crate::*;
use anyhow::Result;
use serde_json::Value;
use std::time::{Duration, Instant};
use tracing::{info, debug};
use tokio::time::timeout;

/// Real Skip API client for e2e testing
pub struct E2ESkipApiClient {
    client: reqwest::Client,
    base_url: String,
}

impl Default for E2ESkipApiClient {
    fn default() -> Self {
        Self::new()
    }
}

impl E2ESkipApiClient {
    /// Create new Skip API client
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(MAX_API_RESPONSE_TIME_SECONDS))
                .build()
                .expect("Failed to create HTTP client"),
            base_url: SKIP_API_BASE_URL.to_string(),
        }
    }

    /// Get real route information from Skip API
    pub async fn get_route(&self, amount: u64) -> Result<Value> {
        let route_request = serde_json::json!({
            "amount_in": amount.to_string(),
            "source_asset_denom": TOKEN_CONTRACT_ADDRESS,
            "source_asset_chain_id": EXPECTED_SOURCE_CHAIN,
            "dest_asset_denom": TOKEN_COSMOS_HUB_DENOM,
            "dest_asset_chain_id": EXPECTED_DEST_CHAIN
        });

        let url = format!("{}/v2/fungible/route", self.base_url);
        debug!("Calling Skip API route: {}", url);

        let start_time = Instant::now();
        let response = self.client
            .post(&url)
            .json(&route_request)
            .send()
            .await?;

        let api_duration = start_time.elapsed();
        debug!("Skip API route call took {:?}", api_duration);

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Skip API route request failed: {}", response.status()));
        }

        let route_data: Value = response.json().await?;
        Ok(route_data)
    }

    /// Get messages from Skip API
    pub async fn get_messages(&self, amount: u64, source_address: &str) -> Result<Value> {
        let messages_request = serde_json::json!({
            "amount_in": amount.to_string(),
            "source_asset_denom": TOKEN_CONTRACT_ADDRESS,
            "source_asset_chain_id": EXPECTED_SOURCE_CHAIN,
            "dest_asset_denom": TOKEN_COSMOS_HUB_DENOM,
            "dest_asset_chain_id": EXPECTED_DEST_CHAIN,
            "address_list": [source_address, EXPECTED_DESTINATION]
        });

        let url = format!("{}/v2/fungible/msgs", self.base_url);
        debug!("Calling Skip API messages: {}", url);

        let start_time = Instant::now();
        let response = self.client
            .post(&url)
            .json(&messages_request)
            .send()
            .await?;

        let api_duration = start_time.elapsed();
        debug!("Skip API messages call took {:?}", api_duration);

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Skip API messages request failed: {}", response.status()));
        }

        let messages_data: Value = response.json().await?;
        Ok(messages_data)
    }

    /// Extract total fees from Skip API response
    pub fn extract_total_fees(&self, response: &Value) -> Result<u64> {
        let estimated_fees = response["estimated_fees"].as_array()
            .ok_or_else(|| anyhow::anyhow!("No estimated_fees found in response"))?;

        let mut total_fees = 0u64;
        for fee in estimated_fees {
            if let Some(amount_str) = fee["amount"].as_str() {
                if let Ok(amount) = amount_str.parse::<u64>() {
                    total_fees = total_fees.saturating_add(amount);
                }
            }
        }

        Ok(total_fees)
    }

    /// Validate that response contains eureka_transfer operation
    pub fn validate_eureka_operation(&self, response: &Value) -> Result<()> {
        let operations = response["operations"].as_array()
            .ok_or_else(|| anyhow::anyhow!("No operations found in response"))?;

        let has_eureka = operations.iter().any(|op| {
            op["type"].as_str() == Some("eureka_transfer")
        });

        if !has_eureka {
            return Err(anyhow::anyhow!("No eureka_transfer operation found"));
        }

        Ok(())
    }
}

/// Ethereum RPC client for e2e testing
pub struct E2EEthereumClient {
    client: reqwest::Client,
    rpc_url: String,
}

impl E2EEthereumClient {
    /// Create new Ethereum RPC client
    pub fn new(rpc_url: String) -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(MAX_API_RESPONSE_TIME_SECONDS))
                .build()
                .expect("Failed to create HTTP client"),
            rpc_url,
        }
    }

    /// Get current block number
    pub async fn get_block_number(&self) -> Result<u64> {
        let payload = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "eth_blockNumber",
            "params": [],
            "id": 1
        });

        let response = self.client
            .post(&self.rpc_url)
            .json(&payload)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Ethereum RPC request failed: {}", response.status()));
        }

        let result: Value = response.json().await?;
        let block_hex = result["result"].as_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid block number response"))?;

        // Convert hex to u64
        let block_number = u64::from_str_radix(&block_hex[2..], 16)
            .map_err(|_| anyhow::anyhow!("Failed to parse block number"))?;

        Ok(block_number)
    }

    /// Get LBTC balance for an address
    pub async fn get_lbtc_balance(&self, address: &str) -> Result<u64> {
        let payload = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "eth_call",
            "params": [{
                "to": TOKEN_CONTRACT_ADDRESS,
                "data": format!("0x70a08231000000000000000000000000{}", &address[2..]) // balanceOf(address)
            }, "latest"],
            "id": 1
        });

        let response = self.client
            .post(&self.rpc_url)
            .json(&payload)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Balance query failed: {}", response.status()));
        }

        let result: Value = response.json().await?;
        let balance_hex = result["result"].as_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid balance response"))?;

        // Convert hex to u64
        let balance = u64::from_str_radix(&balance_hex[2..], 16)
            .map_err(|_| anyhow::anyhow!("Failed to parse balance"))?;

        Ok(balance)
    }

    /// Check if LBTC contract exists at expected address
    pub async fn validate_lbtc_contract(&self) -> Result<()> {
        let payload = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "eth_getCode",
            "params": [TOKEN_CONTRACT_ADDRESS, "latest"],
            "id": 1
        });

        let response = self.client
            .post(&self.rpc_url)
            .json(&payload)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Contract validation failed: {}", response.status()));
        }

        let result: Value = response.json().await?;
        let code = result["result"].as_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid code response"))?;

        if code == "0x" {
            return Err(anyhow::anyhow!("LBTC contract not found at expected address"));
        }

        info!("LBTC contract validated at {}", TOKEN_CONTRACT_ADDRESS);
        Ok(())
    }
}

/// Real Coprocessor client for e2e testing
pub struct E2ECoprocessorClient {
    client: reqwest::Client,
    base_url: String,
}

impl E2ECoprocessorClient {
    /// Create new coprocessor client
    pub fn new(base_url: String) -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(MAX_PROOF_GENERATION_TIME_SECONDS))
                .build()
                .expect("Failed to create HTTP client"),
            base_url,
        }
    }

    /// Check coprocessor health
    pub async fn health_check(&self) -> Result<()> {
        let url = format!("{}/health", self.base_url);
        
        let response = timeout(
            Duration::from_secs(MAX_API_RESPONSE_TIME_SECONDS),
            self.client.get(&url).send()
        ).await??;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Coprocessor health check failed: {}", response.status()));
        }

        info!("Coprocessor health check passed");
        Ok(())
    }

    /// List available controllers (circuits)
    pub async fn list_controllers(&self) -> Result<Value> {
        let url = format!("{}/controllers", self.base_url);
        
        let response = timeout(
            Duration::from_secs(MAX_API_RESPONSE_TIME_SECONDS),
            self.client.get(&url).send()
        ).await??;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Failed to list controllers: {}", response.status()));
        }

        let controllers: Value = response.json().await?;
        Ok(controllers)
    }
}

/// Client factory for creating e2e test clients
pub struct E2EClientFactory;

impl E2EClientFactory {
    /// Create all clients for a given configuration
    pub fn create_clients(config: &E2EConfig) -> (E2ESkipApiClient, E2EEthereumClient, E2ECoprocessorClient) {
        let skip_client = E2ESkipApiClient::new();
        let ethereum_client = E2EEthereumClient::new(config.ethereum_rpc_url.clone());
        let coprocessor_client = E2ECoprocessorClient::new(config.coprocessor_url.clone());

        (skip_client, ethereum_client, coprocessor_client)
    }

    /// Test connectivity for all clients
    pub async fn test_all_connectivity(config: &E2EConfig) -> Result<()> {
        let (_skip_client, ethereum_client, coprocessor_client) = Self::create_clients(config);

        // Test Ethereum connectivity
        let block_number = ethereum_client.get_block_number().await?;
        info!("Ethereum connected, block: {}", block_number);

        // Test coprocessor connectivity
        coprocessor_client.health_check().await?;
        
        // Test Skip API by getting chain info
        let client = reqwest::Client::new();
        let url = format!("{}/v2/info/chains", SKIP_API_BASE_URL);
        let response = client.get(&url).send().await?;
        
        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Skip API connectivity failed"));
        }

        info!("All clients connected successfully");
        Ok(())
    }
} 