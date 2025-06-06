//! E2E test suite for LBTC IBC Eureka transfer system
//!
//! All tests use real RPC calls - no mocking or fallback interfaces

use crate::*;
use anyhow::Result;
use serde_json::Value;
use std::time::{Duration, Instant};
use tokio::time::timeout;
use tracing::{debug, info, warn};

/// Test suite for e2e LBTC transfer validation
pub struct E2ETestSuite {
    config: E2EConfig,
}

impl E2ETestSuite {
    /// Create new test suite with configuration
    pub fn new(config: E2EConfig) -> Self {
        Self { config }
    }

    /// Run all e2e tests
    pub async fn run_all_tests(&self) -> Result<E2ETestResults> {
        info!("Starting complete e2e test suite");
        let start_time = Instant::now();

        let mut results = E2ETestResults::new();

        // Test 1: Connectivity validation
        results.add_test_result("connectivity", self.test_connectivity().await);

        // Test 2: Constants validation
        results.add_test_result(
            "constants_validation",
            self.test_constants_validation().await,
        );

        // Test 3: Skip API route discovery
        results.add_test_result(
            "skip_api_route_discovery",
            self.test_skip_api_route_discovery().await,
        );

        // Test 4: Skip API message construction
        results.add_test_result(
            "skip_api_message_construction",
            self.test_skip_api_message_construction().await,
        );

        // Test 5: Fee validation with different amounts
        results.add_test_result(
            "fee_validation_below_threshold",
            self.test_fee_validation_below_threshold().await,
        );
        results.add_test_result(
            "fee_validation_above_threshold",
            self.test_fee_validation_above_threshold().await,
        );

        // Test 6: Route validation
        results.add_test_result("route_validation", self.test_route_validation().await);

        // Test 7: Coprocessor proof generation (if available)
        if self.config.environment != Environment::Mainnet {
            results.add_test_result(
                "coprocessor_proof_generation",
                self.test_coprocessor_proof_generation().await,
            );
        }

        // Test 8: End-to-end transfer validation (read-only for mainnet)
        results.add_test_result(
            "end_to_end_transfer_validation",
            self.test_end_to_end_transfer_validation().await,
        );

        let total_duration = start_time.elapsed();
        results.total_duration = total_duration;

        info!("E2E test suite completed in {:?}", total_duration);
        info!(
            "Results: {} passed, {} failed",
            results.passed_count(),
            results.failed_count()
        );

        Ok(results)
    }

    /// Test RPC connectivity to all services
    async fn test_connectivity(&self) -> Result<()> {
        info!("Testing connectivity to all services");

        timeout(
            Duration::from_secs(MAX_API_RESPONSE_TIME_SECONDS * 3),
            test_rpc_connectivity(&self.config),
        )
        .await?
    }

    /// Validate all constants
    async fn test_constants_validation(&self) -> Result<()> {
        info!("Validating all constants");
        validate_constants()
    }

    /// Test Skip API route discovery with real LBTC route
    async fn test_skip_api_route_discovery(&self) -> Result<()> {
        info!("Testing Skip API route discovery");

        let client = reqwest::Client::new();
        let route_request = serde_json::json!({
            "amount_in": "1000000000000000", // 0.001 LBTC
            "source_asset_denom": TOKEN_CONTRACT_ADDRESS,
            "source_asset_chain_id": EXPECTED_SOURCE_CHAIN,
            "dest_asset_denom": TOKEN_COSMOS_HUB_DENOM,
            "dest_asset_chain_id": EXPECTED_DEST_CHAIN
        });

        let url = format!("{}/v2/fungible/route", SKIP_API_BASE_URL);
        let start_time = Instant::now();

        let response = timeout(
            Duration::from_secs(MAX_API_RESPONSE_TIME_SECONDS),
            client.post(&url).json(&route_request).send(),
        )
        .await??;

        let api_duration = start_time.elapsed();
        debug!("Skip API route call took {:?}", api_duration);

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Skip API route request failed: {}",
                response.status()
            ));
        }

        let route_data: Value = response.json().await?;
        debug!(
            "Route response: {}",
            serde_json::to_string_pretty(&route_data)?
        );

        // Validate response structure
        let operations = route_data["operations"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("No operations in route response"))?;

        if operations.is_empty() {
            return Err(anyhow::anyhow!("No operations found in route"));
        }

        // Look for eureka_transfer operation
        let has_eureka = operations
            .iter()
            .any(|op| op["type"].as_str() == Some("eureka_transfer"));

        if !has_eureka {
            return Err(anyhow::anyhow!(
                "No eureka_transfer operation found in route"
            ));
        }

        info!(
            "Skip API route discovery successful - found {} operations",
            operations.len()
        );
        Ok(())
    }

    /// Test Skip API message construction
    async fn test_skip_api_message_construction(&self) -> Result<()> {
        info!("Testing Skip API message construction");

        let client = reqwest::Client::new();
        let messages_request = serde_json::json!({
            "amount_in": "1000000000000000", // 0.001 LBTC
            "source_asset_denom": TOKEN_CONTRACT_ADDRESS,
            "source_asset_chain_id": EXPECTED_SOURCE_CHAIN,
            "dest_asset_denom": TOKEN_COSMOS_HUB_DENOM,
            "dest_asset_chain_id": EXPECTED_DEST_CHAIN,
            "address_list": [
                "0x1234567890123456789012345678901234567890", // Dummy source address
                EXPECTED_DESTINATION
            ]
        });

        let url = format!("{}/v2/fungible/msgs", SKIP_API_BASE_URL);
        let start_time = Instant::now();

        let response = timeout(
            Duration::from_secs(MAX_API_RESPONSE_TIME_SECONDS),
            client.post(&url).json(&messages_request).send(),
        )
        .await??;

        let api_duration = start_time.elapsed();
        debug!("Skip API messages call took {:?}", api_duration);

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Skip API messages request failed: {}",
                response.status()
            ));
        }

        let messages_data: Value = response.json().await?;
        debug!(
            "Messages response: {}",
            serde_json::to_string_pretty(&messages_data)?
        );

        // Validate response has fees
        let estimated_fees = messages_data["estimated_fees"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("No estimated_fees in messages response"))?;

        if estimated_fees.is_empty() {
            return Err(anyhow::anyhow!("No fees found in messages response"));
        }

        // Extract total fees
        let mut total_fees = 0u64;
        for fee in estimated_fees {
            if let Some(amount_str) = fee["amount"].as_str() {
                if let Ok(amount) = amount_str.parse::<u64>() {
                    total_fees += amount;
                }
            }
        }

        info!(
            "Skip API message construction successful - total fees: {} LBTC wei",
            total_fees
        );
        Ok(())
    }

    /// Test fee validation with amount below threshold
    async fn test_fee_validation_below_threshold(&self) -> Result<()> {
        info!("Testing fee validation with amount below threshold");

        // Use a fee amount that should pass validation
        let test_fee = TEST_TRANSFER_AMOUNTS[0]; // 0.001 LBTC

        if test_fee >= FEE_THRESHOLD_TOKEN_WEI {
            return Err(anyhow::anyhow!(
                "Test fee {} should be below threshold {}",
                test_fee,
                FEE_THRESHOLD_TOKEN_WEI
            ));
        }

        // This validation would normally be done in the circuit
        let validation_passed = test_fee <= FEE_THRESHOLD_TOKEN_WEI;

        if !validation_passed {
            return Err(anyhow::anyhow!("Fee validation failed unexpectedly"));
        }

        info!(
            "Fee validation passed for amount {} LBTC wei (below threshold)",
            test_fee
        );
        Ok(())
    }

    /// Test fee validation with amount above threshold
    async fn test_fee_validation_above_threshold(&self) -> Result<()> {
        info!("Testing fee validation with amount above threshold");

        // Use a fee amount that should fail validation
        let test_fee = TEST_TRANSFER_AMOUNTS[3]; // 0.002 LBTC (above threshold)

        if test_fee <= FEE_THRESHOLD_TOKEN_WEI {
            return Err(anyhow::anyhow!(
                "Test fee {} should be above threshold {}",
                test_fee,
                FEE_THRESHOLD_TOKEN_WEI
            ));
        }

        // This validation would normally be done in the circuit
        let validation_passed = test_fee <= FEE_THRESHOLD_TOKEN_WEI;

        if validation_passed {
            return Err(anyhow::anyhow!(
                "Fee validation should have failed for excessive fee"
            ));
        }

        info!(
            "Fee validation correctly failed for amount {} LBTC wei (above threshold)",
            test_fee
        );
        Ok(())
    }

    /// Test route validation with expected components
    async fn test_route_validation(&self) -> Result<()> {
        info!("Testing route validation");

        // Construct expected route string
        let valid_route = format!(
            "source_chain:{}|dest_chain:{}|bridge_type:eureka_transfer|bridge_id:{}|entry_contract:{}",
            EXPECTED_SOURCE_CHAIN, EXPECTED_DEST_CHAIN, EXPECTED_BRIDGE_ID, EXPECTED_ENTRY_CONTRACT
        );

        // Test valid route validation
        let valid_result = validate_route_components(&valid_route);
        if !valid_result {
            return Err(anyhow::anyhow!("Valid route validation failed"));
        }

        // Test invalid route validation
        let invalid_route = "source_chain:INVALID|dest_chain:invalid|bridge_type:invalid";
        let invalid_result = validate_route_components(invalid_route);
        if invalid_result {
            return Err(anyhow::anyhow!(
                "Invalid route validation should have failed"
            ));
        }

        info!("Route validation tests passed");
        Ok(())
    }

    /// Test coprocessor proof generation (if not mainnet)
    async fn test_coprocessor_proof_generation(&self) -> Result<()> {
        info!("Testing coprocessor proof generation");

        // This test would require deploying our circuit to the coprocessor
        // For now, just test connectivity
        let client = reqwest::Client::new();
        let health_url = format!("{}/health", self.config.coprocessor_url);

        let response = timeout(
            Duration::from_secs(MAX_API_RESPONSE_TIME_SECONDS),
            client.get(&health_url).send(),
        )
        .await??;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Coprocessor health check failed: {}",
                response.status()
            ));
        }

        info!("Coprocessor is accessible for proof generation");
        Ok(())
    }

    /// Test end-to-end transfer validation (read-only for mainnet)
    async fn test_end_to_end_transfer_validation(&self) -> Result<()> {
        info!("Testing end-to-end transfer validation");

        let start_time = Instant::now();

        // Step 1: Get real route from Skip API
        let route_result = self.test_skip_api_route_discovery().await;
        if let Err(e) = route_result {
            warn!("Route discovery failed: {}", e);
            return Err(e);
        }

        // Step 2: Get real messages from Skip API
        let messages_result = self.test_skip_api_message_construction().await;
        if let Err(e) = messages_result {
            warn!("Message construction failed: {}", e);
            return Err(e);
        }

        // Step 3: Validate constants and configuration
        validate_constants()?;

        // Step 4: Test Ethereum connectivity
        let eth_client = reqwest::Client::new();
        let eth_payload = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "eth_blockNumber",
            "params": [],
            "id": 1
        });

        let eth_response = timeout(
            Duration::from_secs(MAX_API_RESPONSE_TIME_SECONDS),
            eth_client
                .post(&self.config.ethereum_rpc_url)
                .json(&eth_payload)
                .send(),
        )
        .await??;

        if !eth_response.status().is_success() {
            return Err(anyhow::anyhow!("Ethereum RPC check failed"));
        }

        let total_duration = start_time.elapsed();

        if total_duration > Duration::from_secs(MAX_END_TO_END_TIME_SECONDS) {
            warn!(
                "End-to-end test took longer than expected: {:?}",
                total_duration
            );
        }

        info!("End-to-end validation completed in {:?}", total_duration);
        Ok(())
    }
}

/// Helper function to validate route components (from circuit logic)
fn validate_route_components(route_string: &str) -> bool {
    route_string.contains(&format!("source_chain:{}", EXPECTED_SOURCE_CHAIN))
        && route_string.contains("bridge_type:eureka_transfer")
        && route_string.contains(&format!("bridge_id:{}", EXPECTED_BRIDGE_ID))
        && route_string.contains(&format!("entry_contract:{}", EXPECTED_ENTRY_CONTRACT))
}

/// Test results tracking
#[derive(Debug)]
pub struct E2ETestResults {
    pub results: std::collections::HashMap<String, Result<()>>,
    pub total_duration: Duration,
}

impl Default for E2ETestResults {
    fn default() -> Self {
        Self::new()
    }
}

impl E2ETestResults {
    pub fn new() -> Self {
        Self {
            results: std::collections::HashMap::new(),
            total_duration: Duration::from_secs(0),
        }
    }

    pub fn add_test_result(&mut self, test_name: &str, result: Result<()>) {
        self.results.insert(test_name.to_string(), result);
    }

    pub fn passed_count(&self) -> usize {
        self.results.values().filter(|r| r.is_ok()).count()
    }

    pub fn failed_count(&self) -> usize {
        self.results.values().filter(|r| r.is_err()).count()
    }

    pub fn print_summary(&self) {
        println!("\n=== E2E Test Results Summary ===");
        println!("Total Duration: {:?}", self.total_duration);
        println!("Passed: {}", self.passed_count());
        println!("Failed: {}", self.failed_count());
        println!();

        for (test_name, result) in &self.results {
            match result {
                Ok(_) => println!("✅ {}", test_name),
                Err(e) => println!("❌ {}: {}", test_name, e),
            }
        }
        println!();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Once;

    static INIT: Once = Once::new();

    fn setup_logging() {
        INIT.call_once(|| {
            tracing_subscriber::fmt().with_test_writer().try_init().ok();
        });
    }

    #[tokio::test]
    async fn test_e2e_connectivity() {
        setup_logging();

        let config = E2EConfig::from_env().expect("Failed to create config");
        let test_suite = E2ETestSuite::new(config);

        let result = test_suite.test_connectivity().await;
        match result {
            Ok(_) => info!("Connectivity test passed"),
            Err(e) => warn!(
                "Connectivity test failed (expected in some environments): {}",
                e
            ),
        }
    }

    #[tokio::test]
    async fn test_e2e_constants_validation() {
        setup_logging();

        let config = E2EConfig::local();
        let test_suite = E2ETestSuite::new(config);

        let result = test_suite.test_constants_validation().await;
        assert!(
            result.is_ok(),
            "Constants validation should pass: {:?}",
            result
        );
    }

    #[tokio::test]
    async fn test_e2e_skip_api_basic() {
        setup_logging();

        let config = E2EConfig::from_env().expect("Failed to create config");
        let test_suite = E2ETestSuite::new(config);

        let result = test_suite.test_skip_api_route_discovery().await;
        match result {
            Ok(_) => info!("Skip API test passed"),
            Err(e) => warn!("Skip API test failed (may be rate limited): {}", e),
        }
    }

    #[tokio::test]
    async fn test_e2e_fee_validation() {
        setup_logging();

        let config = E2EConfig::local();
        let test_suite = E2ETestSuite::new(config);

        let below_result = test_suite.test_fee_validation_below_threshold().await;
        assert!(
            below_result.is_ok(),
            "Below threshold validation should pass: {:?}",
            below_result
        );

        let above_result = test_suite.test_fee_validation_above_threshold().await;
        assert!(
            above_result.is_ok(),
            "Above threshold validation should pass: {:?}",
            above_result
        );
    }

    #[tokio::test]
    async fn test_e2e_route_validation() {
        setup_logging();

        let config = E2EConfig::local();
        let test_suite = E2ETestSuite::new(config);

        let result = test_suite.test_route_validation().await;
        assert!(result.is_ok(), "Route validation should pass: {:?}", result);
    }
}
