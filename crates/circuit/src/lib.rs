use common_merkle_proofs::merkle::types::MerkleVerifiable;
use ethereum_merkle_proofs::merkle_lib::types::{
    EthereumAccount, EthereumProofType, RlpDecodable as MerkleProofRlpDecodable,
};
use types::ethereum::RlpDecodable;
use types::{CircuitOutput, CircuitWitness, WithdrawRequest};
use valence_coprocessor::Witness;

/// Main circuit function that processes and verifies Ethereum state proofs.
///
/// This function:
/// 1. Takes a vector of witnesses containing the circuit input data
/// 2. Deserializes the input into a CircuitWitness
/// 3. Verifies all Ethereum state proofs against the provided state root
/// 4. Returns the verified state root as output
///
/// # Arguments
/// * `witnesses` - Vector of Witness objects containing the input data
///
/// # Returns
/// * `Vec<u8>` - The verified state root as a byte vector
///
/// # Panics
/// * If the input witness is not of type Data
/// * If any proof verification fails
pub fn circuit(witnesses: Vec<Witness>) -> Vec<u8> {
    let circuit_input_witness = witnesses.first().unwrap();
    // this macro isn't necessary, but rust analyzer throws a false positive
    #[allow(unused)]
    let mut circuit_input_serialized: Vec<u8> = Vec::new();
    match circuit_input_witness {
        Witness::Data(data) => {
            circuit_input_serialized = data.clone();
        }
        Witness::StateProof(_) => panic!(
            "Unexpected Input: For this example template we encode all data in a single field."
        ),
    }

    // Deserialize the CircuitWitness from the input data
    let input: CircuitWitness = serde_json::from_slice(&circuit_input_serialized).unwrap();

    let mut withdraw_requests: Vec<WithdrawRequest> = Vec::new();

    // Verify all Ethereum proofs against the state root
    assert_eq!(input.state_proofs.len(), 2);
    for proof in input.state_proofs {
        let proof: EthereumProofType = serde_json::from_slice(&proof.proof).unwrap();
        match &proof {
            EthereumProofType::Account(account_proof) => {
                // Decode and print the account state for debugging
                let _decoded_account = EthereumAccount::rlp_decode(&account_proof.value).unwrap();
            }
            EthereumProofType::Simple(storage_proof) => {
                let withdraw_request = WithdrawRequest::rlp_decode(&storage_proof.value).unwrap();
                withdraw_requests.push(withdraw_request);
            }
            _ => {}
        }
        // Verify the proof against the state root
        assert!(proof.verify(&input.state_root).unwrap());
    }

    // commit the verified withdraw requests and the root that was used as an output
    let output = CircuitOutput {
        withdraw_requests,
        state_root: input.state_root,
    };
    serde_json::to_vec(&output).expect("Failed to serialize circuit output")
}
