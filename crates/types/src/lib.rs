use serde::{Deserialize, Serialize};
use valence_coprocessor::StateProof;

#[derive(Serialize, Deserialize)]
pub struct CircuitWitness {
    pub state_proofs: Vec<StateProof>,
    pub state_root: [u8; 32],
}
