#![no_std]
pub mod ethereum;
extern crate alloc;
use alloc::{string::String, vec::Vec};
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
