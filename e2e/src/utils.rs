//! Utility functions for e2e testing

use crate::*;
use anyhow::Result;
use std::time::Duration;
use tracing::{debug, info};

/// Format LBTC amount from wei to human readable
pub fn format_lbtc_amount(amount_wei: u64) -> String {
    let amount_lbtc = amount_wei as f64 / 10_f64.powi(LBTC_DECIMALS as i32);
    format!("{:.8} LBTC", amount_lbtc)
}

/// Convert LBTC amount to wei
pub fn lbtc_to_wei(amount_lbtc: f64) -> u64 {
    (amount_lbtc * 10_f64.powi(LBTC_DECIMALS as i32)) as u64
}

/// Validate Ethereum address format
pub fn validate_ethereum_address(address: &str) -> Result<()> {
    if !address.starts_with("0x") {
        return Err(anyhow::anyhow!("Address must start with 0x"));
    }

    if address.len() != 42 {
        return Err(anyhow::anyhow!("Address must be 42 characters long"));
    }

    if !address[2..].chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(anyhow::anyhow!("Address contains invalid characters"));
    }

    Ok(())
}

/// Validate Cosmos address format  
pub fn validate_cosmos_address(address: &str) -> Result<()> {
    if !address.starts_with("cosmos1") {
        return Err(anyhow::anyhow!("Cosmos address must start with cosmos1"));
    }

    if address.len() < 39 || address.len() > 63 {
        return Err(anyhow::anyhow!(
            "Invalid Cosmos address length: {}, must be between 39-63 characters",
            address.len()
        ));
    }

    Ok(())
}

/// Generate test source address for different test scenarios
pub fn generate_test_address() -> String {
    "0x1234567890123456789012345678901234567890".to_string()
}

/// Calculate expected fee percentage of transfer amount
pub fn calculate_fee_percentage(transfer_amount: u64, fee_amount: u64) -> f64 {
    if transfer_amount == 0 {
        return 0.0;
    }
    (fee_amount as f64 / transfer_amount as f64) * 100.0
}

/// Validate that fee is within acceptable range
pub fn validate_fee_amount(fee_amount: u64, transfer_amount: u64) -> Result<()> {
    // Check absolute fee threshold
    if fee_amount > FEE_THRESHOLD_TOKEN_WEI {
        return Err(anyhow::anyhow!(
            "Fee {} wei exceeds threshold {} wei",
            fee_amount,
            FEE_THRESHOLD_TOKEN_WEI
        ));
    }

    // Check percentage threshold (fees shouldn't be more than 10% of transfer)
    let percentage = calculate_fee_percentage(transfer_amount, fee_amount);
    if percentage > 10.0 {
        return Err(anyhow::anyhow!("Fee {}% is too high (max 10%)", percentage));
    }

    info!(
        "Fee validation passed: {} wei ({:.4}%)",
        fee_amount, percentage
    );
    Ok(())
}

/// Create test transfer request
pub fn create_test_transfer_request(amount: u64) -> serde_json::Value {
    serde_json::json!({
        "amount": amount.to_string(),
        "source_address": generate_test_address(),
        "destination": EXPECTED_DESTINATION,
        "max_fee_token_wei": FEE_THRESHOLD_TOKEN_WEI
    })
}

/// Validate Skip API response structure
pub fn validate_skip_response_structure(response: &serde_json::Value) -> Result<()> {
    // Check for required top-level fields
    if !response.is_object() {
        return Err(anyhow::anyhow!("Response must be a JSON object"));
    }

    let obj = response.as_object().unwrap();

    // Check for operations array
    if !obj.contains_key("operations") {
        return Err(anyhow::anyhow!("Response missing 'operations' field"));
    }

    let operations = obj["operations"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("'operations' must be an array"))?;

    if operations.is_empty() {
        return Err(anyhow::anyhow!("Operations array cannot be empty"));
    }

    // Check for estimated_fees array
    if !obj.contains_key("estimated_fees") {
        return Err(anyhow::anyhow!("Response missing 'estimated_fees' field"));
    }

    let fees = obj["estimated_fees"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("'estimated_fees' must be an array"))?;

    if fees.is_empty() {
        return Err(anyhow::anyhow!("Estimated fees array cannot be empty"));
    }

    debug!("Skip response structure validation passed");
    Ok(())
}

/// Extract and validate route components from Skip API response
pub fn extract_route_components(response: &serde_json::Value) -> Result<RouteComponents> {
    let operations = response["operations"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("No operations found"))?;

    for operation in operations {
        if operation["type"].as_str() == Some("eureka_transfer") {
            let components = RouteComponents {
                operation_type: operation["type"].as_str().unwrap_or("").to_string(),
                from_chain_id: operation["from_chain_id"]
                    .as_str()
                    .unwrap_or("")
                    .to_string(),
                to_chain_id: operation["to_chain_id"].as_str().unwrap_or("").to_string(),
                bridge_id: operation["bridge_id"].as_str().unwrap_or("").to_string(),
                entry_contract: operation["entry_contract_address"]
                    .as_str()
                    .unwrap_or("")
                    .to_string(),
                denom_in: operation["denom_in"].as_str().unwrap_or("").to_string(),
                denom_out: operation["denom_out"].as_str().unwrap_or("").to_string(),
            };

            return Ok(components);
        }
    }

    Err(anyhow::anyhow!("No eureka_transfer operation found"))
}

/// Route components extracted from Skip API
#[derive(Debug, Clone)]
pub struct RouteComponents {
    pub operation_type: String,
    pub from_chain_id: String,
    pub to_chain_id: String,
    pub bridge_id: String,
    pub entry_contract: String,
    pub denom_in: String,
    pub denom_out: String,
}

impl RouteComponents {
    /// Validate against expected constants
    pub fn validate_against_expected(&self) -> Result<()> {
        if self.operation_type != "eureka_transfer" {
            return Err(anyhow::anyhow!(
                "Expected eureka_transfer, got {}",
                self.operation_type
            ));
        }

        if self.from_chain_id != EXPECTED_SOURCE_CHAIN {
            return Err(anyhow::anyhow!(
                "Expected source chain {}, got {}",
                EXPECTED_SOURCE_CHAIN,
                self.from_chain_id
            ));
        }

        if self.to_chain_id != EXPECTED_DEST_CHAIN {
            return Err(anyhow::anyhow!(
                "Expected dest chain {}, got {}",
                EXPECTED_DEST_CHAIN,
                self.to_chain_id
            ));
        }

        if self.bridge_id != EXPECTED_BRIDGE_ID {
            return Err(anyhow::anyhow!(
                "Expected bridge {}, got {}",
                EXPECTED_BRIDGE_ID,
                self.bridge_id
            ));
        }

        if self.entry_contract.to_lowercase() != EXPECTED_ENTRY_CONTRACT.to_lowercase() {
            return Err(anyhow::anyhow!(
                "Expected entry contract {}, got {}",
                EXPECTED_ENTRY_CONTRACT,
                self.entry_contract
            ));
        }

        if self.denom_in.to_lowercase() != TOKEN_CONTRACT_ADDRESS.to_lowercase() {
            return Err(anyhow::anyhow!(
                "Expected source denom {}, got {}",
                TOKEN_CONTRACT_ADDRESS,
                self.denom_in
            ));
        }

        if self.denom_out != TOKEN_COSMOS_HUB_DENOM {
            return Err(anyhow::anyhow!(
                "Expected dest denom {}, got {}",
                TOKEN_COSMOS_HUB_DENOM,
                self.denom_out
            ));
        }

        info!("Route components validation passed");
        Ok(())
    }

    /// Generate canonical route string for hashing
    pub fn to_canonical_string(&self) -> String {
        format!(
            "source_chain:{}|dest_chain:{}|source_denom:{}|dest_denom:{}|bridge_type:{}|bridge_id:{}|entry_contract:{}",
            self.from_chain_id,
            self.to_chain_id,
            self.denom_in,
            self.denom_out,
            self.operation_type,
            self.bridge_id,
            self.entry_contract
        )
    }
}

/// Performance metrics for e2e tests
#[derive(Debug, Default)]
pub struct PerformanceMetrics {
    pub skip_api_route_duration: Option<Duration>,
    pub skip_api_messages_duration: Option<Duration>,
    pub ethereum_rpc_duration: Option<Duration>,
    pub coprocessor_duration: Option<Duration>,
    pub total_duration: Option<Duration>,
}

impl PerformanceMetrics {
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if all metrics are within acceptable thresholds
    pub fn validate_performance(&self) -> Result<()> {
        if let Some(duration) = self.skip_api_route_duration {
            if duration > Duration::from_secs(MAX_API_RESPONSE_TIME_SECONDS) {
                return Err(anyhow::anyhow!(
                    "Skip API route call too slow: {:?}",
                    duration
                ));
            }
        }

        if let Some(duration) = self.skip_api_messages_duration {
            if duration > Duration::from_secs(MAX_API_RESPONSE_TIME_SECONDS) {
                return Err(anyhow::anyhow!(
                    "Skip API messages call too slow: {:?}",
                    duration
                ));
            }
        }

        if let Some(duration) = self.ethereum_rpc_duration {
            if duration > Duration::from_secs(MAX_API_RESPONSE_TIME_SECONDS) {
                return Err(anyhow::anyhow!(
                    "Ethereum RPC call too slow: {:?}",
                    duration
                ));
            }
        }

        if let Some(duration) = self.coprocessor_duration {
            if duration > Duration::from_secs(MAX_PROOF_GENERATION_TIME_SECONDS) {
                return Err(anyhow::anyhow!("Coprocessor call too slow: {:?}", duration));
            }
        }

        if let Some(duration) = self.total_duration {
            if duration > Duration::from_secs(MAX_END_TO_END_TIME_SECONDS) {
                return Err(anyhow::anyhow!("Total e2e flow too slow: {:?}", duration));
            }
        }

        info!("Performance validation passed");
        Ok(())
    }

    /// Print performance summary
    pub fn print_summary(&self) {
        println!("\n=== Performance Metrics ===");

        if let Some(d) = self.skip_api_route_duration {
            println!("Skip API Route: {:?}", d);
        }

        if let Some(d) = self.skip_api_messages_duration {
            println!("Skip API Messages: {:?}", d);
        }

        if let Some(d) = self.ethereum_rpc_duration {
            println!("Ethereum RPC: {:?}", d);
        }

        if let Some(d) = self.coprocessor_duration {
            println!("Coprocessor: {:?}", d);
        }

        if let Some(d) = self.total_duration {
            println!("Total E2E: {:?}", d);
        }

        println!();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_lbtc_amount() {
        assert_eq!(format_lbtc_amount(1000000000000000), "0.00100000 LBTC");
        assert_eq!(format_lbtc_amount(1890000000000000), "0.00189000 LBTC");
    }

    #[test]
    fn test_lbtc_to_wei() {
        assert_eq!(lbtc_to_wei(0.001), 1000000000000000);
        assert_eq!(lbtc_to_wei(0.00189), 1890000000000000);
    }

    #[test]
    fn test_validate_ethereum_address() {
        assert!(validate_ethereum_address(TOKEN_CONTRACT_ADDRESS).is_ok());
        assert!(validate_ethereum_address("0x123").is_err());
        assert!(validate_ethereum_address("invalid").is_err());
    }

    #[test]
    fn test_validate_cosmos_address() {
        assert!(validate_cosmos_address(EXPECTED_DESTINATION).is_ok());
        assert!(validate_cosmos_address("cosmos1short").is_err());
        assert!(validate_cosmos_address("invalid").is_err());
    }

    #[test]
    fn test_calculate_fee_percentage() {
        assert_eq!(
            calculate_fee_percentage(1000000000000000, 10000000000000),
            1.0
        );
        assert_eq!(calculate_fee_percentage(0, 1000), 0.0);
    }

    #[test]
    fn test_validate_fee_amount() {
        // Below threshold should pass - small fee (1%)
        assert!(validate_fee_amount(10000000000000, 1000000000000000).is_ok());

        // Above threshold should fail
        assert!(validate_fee_amount(2000000000000000, 1000000000000000).is_err());
    }
}
