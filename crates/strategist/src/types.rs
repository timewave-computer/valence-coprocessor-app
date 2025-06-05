//! Type definitions for token transfer operations

use serde::{Deserialize, Serialize};
use anyhow::{Result, anyhow};

/// Transfer request for IBC Eureka transfers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferRequest {
    /// Amount of tokens to transfer (in wei)
    pub amount: u64,
    /// Source address on Ethereum (0x...)
    pub source_address: String,
    /// Destination address on Cosmos Hub (cosmos1...)
    pub destination: String,
    /// Maximum acceptable fee (in token wei)
    pub max_fee: Option<u64>,
}

/// Result of a completed transfer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferResult {
    /// Ethereum transaction hash
    pub transaction_hash: String,
    /// Coprocessor proof hash
    pub proof_hash: String,
    /// Estimated transfer duration in seconds
    pub estimated_duration: u32,
    /// Total fees paid (in token wei)
    pub fees_paid: u64,
}

/// Skip API response structure (simplified)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkipApiResponse {
    pub operations: Vec<Operation>,
    pub estimated_route_duration_seconds: u64,
    pub estimated_fees: Vec<Fee>,
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
        self.operations.iter().any(|op| {
            matches!(op, Operation::EurekaTransfer(_))
        })
    }
}

/// Skip API operation types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Operation {
    #[serde(rename = "evm_swap")]
    EvmSwap(EvmSwapOperation),
    #[serde(rename = "eureka_transfer")]
    EurekaTransfer(EurekaTransferOperation),
    #[serde(rename = "swap")]
    Swap(SwapOperation),
    #[serde(rename = "transfer")]
    Transfer(TransferOperation),
}

/// EVM swap operation (token conversion)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvmSwapOperation {
    pub input_token: String,
    pub amount_in: String,
    pub amount_out: String,
    pub denom_in: String,
    pub denom_out: String,
}

/// IBC Eureka transfer operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EurekaTransferOperation {
    pub from_chain_id: String,
    pub to_chain_id: String,
    pub denom_in: String,
    pub denom_out: String,
    pub bridge_id: String,
    pub entry_contract_address: String,
    pub smart_relay: bool,
    pub smart_relay_fee_quote: Option<SmartRelayFee>,
}

/// Smart relay fee information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartRelayFee {
    pub fee_amount: String,
    pub fee_denom: String,
    pub expiration: String,
}

/// Swap operation on intermediate chains
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapOperation {
    pub chain_id: String,
    pub denom_in: String,
    pub denom_out: String,
}

/// IBC transfer operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferOperation {
    pub from_chain_id: String,
    pub to_chain_id: String,
    pub denom_in: String,
    pub denom_out: String,
    pub bridge_id: String,
}

/// Fee information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fee {
    pub fee_type: String,
    pub bridge_id: Option<String>,
    pub amount: String,
    pub chain_id: String,
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
    pub entry_contract: String,
}

impl RouteData {
    /// Extract route data from Skip API response
    pub fn from_skip_response(response: &SkipApiResponse) -> Result<Self> {
        // Find the eureka_transfer operation
        let eureka_op = response.operations
            .iter()
            .find_map(|op| match op {
                Operation::EurekaTransfer(eureka) => Some(eureka),
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
            entry_contract: eureka_op.entry_contract_address.clone(),
        })
    }

    /// Generate route hash for validation
    pub fn generate_hash(&self) -> String {
        use sha3::{Digest, Sha3_256};
        
        let canonical_route = format!(
            "source_chain:{}|dest_chain:{}|source_denom:{}|dest_denom:{}|bridge_type:{}|bridge_id:{}|entry_contract:{}",
            self.source_chain,
            self.dest_chain,
            self.source_denom,
            self.dest_denom,
            self.bridge_type,
            self.bridge_id,
            self.entry_contract
        );

        let mut hasher = Sha3_256::new();
        hasher.update(canonical_route.as_bytes());
        hex::encode(hasher.finalize())
    }
}

/// Fee data for ZK circuit validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeData {
    pub total_fee_token_wei: u64,
    pub fee_breakdown: Vec<Fee>,
}

impl FeeData {
    /// Extract fee data from Skip API response
    pub fn from_skip_response(response: &SkipApiResponse) -> Result<Self> {
        let total_fee_token_wei = response.total_fees();
        
        Ok(FeeData {
            total_fee_token_wei,
            fee_breakdown: response.estimated_fees.clone(),
        })
    }
}

/// Coprocessor proof request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofRequest {
    pub route_data: RouteData,
    pub fee_data: FeeData,
    pub destination_address: String,
    pub expected_route_hash: String,
} 