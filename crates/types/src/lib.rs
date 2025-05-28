#![no_std]
pub mod ethereum;
extern crate alloc;
use alloc::{string::String, vec::Vec};
use anyhow::{Context, Result};
use ethereum::{rlp_decode_bytes, RlpDecodable};
use num_bigint::BigUint;
use serde::{Deserialize, Serialize};
use valence_coprocessor::StateProof;

/// Represents the witness data needed for the circuit computation.
/// This struct contains all the necessary proofs and state information
/// required to verify Ethereum state transitions.
#[derive(Serialize, Deserialize)]
pub struct CircuitWitness {
    /// Vector of state proofs that need to be verified
    pub state_proofs: Vec<StateProof>,
    /// The Merkle root of the Ethereum state tree that proofs are verified against
    pub state_root: [u8; 32],
}

/// Represents the output of the circuit computation.
/// Contains the list of withdraw requests and the updated state root
/// after processing these requests.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitOutput {
    /// List of withdraw requests that were processed
    pub withdraw_requests: Vec<WithdrawRequest>,
    /// The updated Merkle root of the Ethereum state tree after processing requests
    pub state_root: [u8; 32],
}

/// Represents a request to withdraw funds from the system.
/// Contains all necessary information to process a withdrawal including
/// the owner, amount, and destination address.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WithdrawRequest {
    /// Unique identifier for the withdraw request
    pub id: u64,
    /// Address of the account initiating the withdrawal
    pub owner: String,
    /// Rate at which the withdrawal should be processed
    pub redemption_rate: BigUint,
    /// Amount of shares to be withdrawn
    pub shares_amount: BigUint,
    /// Address that will receive the withdrawn funds
    pub receiver: String,
}

impl RlpDecodable for WithdrawRequest {
    /// Decodes a WithdrawRequest from RLP encoded bytes.
    ///
    /// # Arguments
    /// * `rlp` - A slice of bytes containing the RLP encoded WithdrawRequest
    ///
    /// # Returns
    /// * `Result<Self>` - The decoded WithdrawRequest or an error if decoding fails
    ///
    /// # Errors
    /// Returns an error if:
    /// * The RLP encoding is invalid
    /// * Required fields are missing
    /// * String fields contain invalid UTF-8
    fn rlp_decode(rlp: &[u8]) -> Result<Self> {
        let request_rlp_bytes = rlp_decode_bytes(rlp)?;
        let id = u64::from_be_bytes({
            let mut padded = [0u8; 8];
            let id_slice = request_rlp_bytes.first().unwrap().as_ref();
            let start = 8 - id_slice.len();
            padded[start..].copy_from_slice(id_slice);
            padded
        });

        let owner = String::from_utf8(
            request_rlp_bytes
                .get(1)
                .context("Failed to get owner")?
                .to_vec(),
        )
        .unwrap();

        let redemption_rate = BigUint::from_bytes_be(
            request_rlp_bytes
                .get(2)
                .context("Failed to get redemption rate")?
                .as_ref(),
        );

        let shares_amount = BigUint::from_bytes_be(
            request_rlp_bytes
                .get(3)
                .context("Failed to get shares")?
                .as_ref(),
        );

        let receiver = String::from_utf8(
            request_rlp_bytes
                .get(4)
                .context("Failed to get receiver")?
                .to_vec(),
        )
        .unwrap();

        Ok(WithdrawRequest {
            id,
            owner,
            redemption_rate,
            shares_amount,
            receiver,
        })
    }
}
