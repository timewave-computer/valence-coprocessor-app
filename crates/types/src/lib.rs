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
