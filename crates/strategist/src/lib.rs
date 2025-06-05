//! LBTC IBC Eureka Transfer Strategist
//! 
//! Orchestrates LBTC transfers from Ethereum to Cosmos Hub using:
//! - Skip API for route discovery and message construction
//! - Coprocessor for ZK proof generation and validation
//! - Ethereum client for transaction submission

use anyhow::Result;
use tracing::info;
use serde::{Deserialize, Serialize};
use tracing::{warn, error};

mod skip_api;
mod types;
mod clients;

pub use skip_api::SkipApiClient;
pub use types::*;
pub use clients::*;

/// Main strategist for orchestrating LBTC transfers via IBC Eureka
pub struct LbtcTransferStrategist {
    /// Coprocessor client for ZK proof generation
    coprocessor: CoprocessorClient,
    /// Ethereum client for transaction submission
    ethereum: EthereumClient,
    /// Skip API client for route/message discovery
    skip_api: SkipApiClient,
}

impl LbtcTransferStrategist {
    /// Creates a new LBTC transfer strategist
    pub fn new(
        coprocessor_url: &str,
        ethereum_rpc_url: &str,
        mnemonic: &str,
        environment: Environment,
    ) -> Result<Self> {
        info!("Initializing LBTC Transfer Strategist for {:?}", environment);

        // Initialize domain clients
        let coprocessor = CoprocessorClient::new(coprocessor_url)?;
        let ethereum = EthereumClient::new(ethereum_rpc_url, mnemonic)?;
        let skip_api = SkipApiClient::new();

        Ok(Self {
            coprocessor,
            ethereum,
            skip_api,
        })
    }

    /// Executes a complete LBTC transfer flow
    pub async fn execute_transfer(&self, request: TransferRequest) -> Result<TransferResult> {
        info!("Starting LBTC transfer execution for amount: {}", request.amount);

        // Step 1: Get Skip API messages with route and fee information
        let messages = self.skip_api.get_messages(&request).await?;
        info!("Retrieved {} messages from Skip API", messages.operations.len());

        // Step 2: Generate ZK proof validating route and fees
        let proof_request = self.build_proof_request(&messages, &request)?;
        let proof = self.coprocessor.generate_proof(proof_request).await?;
        info!("Generated ZK proof for transfer validation");

        // Step 3: Submit transaction to Ethereum
        let tx_hash = self.ethereum.submit_transaction(&messages, &proof).await?;
        info!("Submitted Ethereum transaction: {}", tx_hash);

        Ok(TransferResult {
            transaction_hash: tx_hash,
            proof_hash: proof.hash,
            estimated_duration: messages.estimated_route_duration_seconds,
            fees_paid: messages.total_fees(),
        })
    }

    /// Builds coprocessor proof request from Skip API response
    fn build_proof_request(
        &self,
        messages: &SkipApiResponse,
        request: &TransferRequest,
    ) -> Result<ProofRequest> {
        // Extract relevant data for ZK circuit validation
        let route_data = RouteData::from_skip_response(messages)?;
        let fee_data = FeeData::from_skip_response(messages)?;

        Ok(ProofRequest {
            route_data,
            fee_data,
            destination_address: request.destination.clone(),
            expected_route_hash: HARDCODED_ROUTE_HASH.to_string(),
        })
    }
}

/// Hardcoded route hash for LBTC Eureka transfers
const HARDCODED_ROUTE_HASH: &str = "a041afeb1546e275ec0038183732036ce653b197e8129748da95cf6c7de43abf";

/// Hardcoded destination address for testing
const HARDCODED_DESTINATION: &str = "cosmos1zxj6y5h3r8k9v7n2m4l1q8w5e3t6y9u0i7o4p2s5d8f6g3h1j4k7l9n2";

/// Fee threshold in LBTC wei (0.0000189 LBTC = $2.00 equivalent)
const FEE_THRESHOLD_LBTC_WEI: u64 = 1890000000000000; 