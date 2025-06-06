extern crate alloc;
use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Route request to Skip API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteRequest {
    pub amount_in: String,
    pub source_asset_denom: String,
    pub source_asset_chain_id: String,
    pub dest_asset_denom: String,
    pub dest_asset_chain_id: String,
}

/// Skip API response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkipApiResponse {
    pub dest_asset_denom: String,
    pub estimated_fees: Vec<Fee>,
    pub operations: Vec<Operation>,
}

impl SkipApiResponse {
    /// Calculate total fees from all operations
    pub fn total_fees(&self) -> u64 {
        self.estimated_fees
            .iter()
            .filter_map(|fee| fee.amount.parse::<u64>().ok())
            .sum()
    }

    /// Check if this response contains a eureka_transfer operation
    pub fn has_eureka_transfer(&self) -> bool {
        self.operations
            .iter()
            .any(|op| matches!(op, Operation::EurekaTransfer { .. }))
    }
}

/// Skip API operation types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Operation {
    EurekaTransfer {
        eureka_transfer: EurekaTransferOperation,
    },
    Other(serde_json::Value), // Capture all other operation types we don't use
}

/// IBC Eureka transfer operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EurekaTransferOperation {
    pub bridge_id: String,
    pub denom_in: String,
    pub denom_out: String,
    pub from_chain_id: String,
    pub to_chain_id: String,
}

/// Fee information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fee {
    pub amount: String,
    pub usd_amount: String,
}

/// Route data for ZK circuit validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteData {
    pub source_chain: String,
    pub dest_chain: String,
    pub source_denom: String,
    pub dest_denom: String,
    pub bridge_type: String,
    pub bridge_id: String,
}

impl RouteData {
    /// Extract route data from Skip API response
    pub fn from_skip_response(response: &SkipApiResponse) -> Result<Self> {
        // Find the eureka_transfer operation
        let eureka_op = response
            .operations
            .iter()
            .find_map(|op| match op {
                Operation::EurekaTransfer {
                    eureka_transfer, ..
                } => Some(eureka_transfer),
                _ => None,
            })
            .ok_or_else(|| anyhow!("No eureka_transfer operation found"))?;

        Ok(RouteData {
            source_chain: eureka_op.from_chain_id.clone(),
            dest_chain: eureka_op.to_chain_id.clone(),
            source_denom: eureka_op.denom_in.clone(),
            dest_denom: eureka_op.denom_out.clone(),
            bridge_type: "eureka_transfer".to_string(),
            bridge_id: eureka_op.bridge_id.clone(),
        })
    }

    /// Compute SHA256 hash of the route for validation
    pub fn compute_hash(&self) -> String {
        let route_string = format!(
            "{}|{}|{}|{}|{}|{}",
            self.source_chain,
            self.dest_chain,
            self.source_denom,
            self.dest_denom,
            self.bridge_type,
            self.bridge_id,
        );

        let mut hasher = Sha256::new();
        hasher.update(route_string.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Validate route against expected hash
    pub fn validate_route_hash(&self, expected_hash: &str) -> Result<()> {
        let computed_hash = self.compute_hash();
        if computed_hash != expected_hash {
            return Err(anyhow!(
                "Route hash mismatch. Expected: {}, Computed: {}. Route may have been tampered with.",
                expected_hash,
                computed_hash
            ));
        }
        Ok(())
    }

    /// Check if this is a valid LBTC Eureka transfer route
    pub fn is_valid_lbtc_eureka_route(&self) -> Result<()> {
        // Validate source chain is Ethereum mainnet
        if self.source_chain != "1" {
            return Err(anyhow!(
                "Invalid source chain: {}. Expected Ethereum mainnet (1)",
                self.source_chain
            ));
        }

        // Validate destination chain is either Cosmos Hub or Ledger (intermediate chain)
        // The eureka_transfer operation goes from Ethereum to Ledger first
        let valid_dest_chains = ["cosmoshub-4", "ledger-mainnet-1"];
        if !valid_dest_chains.contains(&self.dest_chain.as_str()) {
            return Err(anyhow!(
                "Invalid destination chain: {}. Expected one of: {:?}",
                self.dest_chain,
                valid_dest_chains
            ));
        }

        // Validate bridge type is eureka_transfer
        if self.bridge_type != "eureka_transfer" {
            return Err(anyhow!(
                "Invalid bridge type: {}. Expected eureka_transfer",
                self.bridge_type
            ));
        }

        // Validate bridge ID is EUREKA
        if self.bridge_id != "EUREKA" {
            return Err(anyhow!(
                "Invalid bridge ID: {}. Expected EUREKA",
                self.bridge_id
            ));
        }

        // Validate source denom is a valid LBTC token
        if !self.is_valid_lbtc_token(&self.source_denom) {
            return Err(anyhow!(
                "Invalid source denom: {}. Not a recognized LBTC token",
                self.source_denom
            ));
        }

        Ok(())
    }

    /// Check if a token address is a valid LBTC token
    fn is_valid_lbtc_token(&self, token_address: &str) -> bool {
        // Known LBTC token addresses on Ethereum
        let valid_lbtc_tokens = [
            "0x8236a87084f8B84306f72007F36F2618A5634494", // LBTC token
        ];

        valid_lbtc_tokens.contains(&token_address)
    }
}

/// Fee data for ZK circuit validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeData {
    pub fee_breakdown: Vec<Fee>,
}

impl FeeData {
    /// Extract fee data from Skip API response
    pub fn from_skip_response(response: &SkipApiResponse) -> Result<Self> {
        Ok(FeeData {
            fee_breakdown: response.estimated_fees.clone(),
        })
    }
}

/// Helper functions for route validation
impl SkipApiResponse {
    /// Validate that this response represents a secure LBTC transfer route
    pub fn validate_lbtc_route(&self) -> Result<()> {
        // Check that we have a eureka_transfer operation
        if !self.has_eureka_transfer() {
            return Err(anyhow!(
                "Response does not contain eureka_transfer operation"
            ));
        }

        // Extract and validate route data
        let route_data = RouteData::from_skip_response(self)?;
        route_data.is_valid_lbtc_eureka_route()?;

        // Validate fee structure
        let fee_data = FeeData::from_skip_response(self)?;
        if fee_data.fee_breakdown.is_empty() {
            return Err(anyhow!("No fee information provided"));
        }

        // Check for reasonable fee amounts (not exceeding $10 USD equivalent)
        const MAX_FEE_USD: f64 = 10.0;
        for fee in &fee_data.fee_breakdown {
            if let Ok(usd_amount) = fee.usd_amount.parse::<f64>() {
                if usd_amount > MAX_FEE_USD {
                    return Err(anyhow!(
                        "Fee {} USD exceeds maximum allowed fee of {} USD",
                        usd_amount,
                        MAX_FEE_USD
                    ));
                }
            }
        }

        Ok(())
    }

    /// Get the computed route hash for this response
    pub fn get_route_hash(&self) -> Result<String> {
        let route_data = RouteData::from_skip_response(self)?;
        Ok(route_data.compute_hash())
    }
}
