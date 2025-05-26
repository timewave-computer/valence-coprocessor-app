use common_merkle_proofs::merkle::types::MerkleVerifiable;
use ethereum_merkle_proofs::merkle_lib::{
    types::{EthereumAccount, EthereumProofType},
    RlpDecodable,
};
use types::CircuitWitness;
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
    let mut circuit_input_serialized: Vec<u8> = vec![];
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

    // Verify all Ethereum proofs against the state root
    for proof in input.state_proofs {
        let proof: EthereumProofType = serde_json::from_slice(&proof.proof).unwrap();
        match &proof {
            EthereumProofType::Account(account_proof) => {
                // Decode and print the account state for debugging
                let _decoded_account = EthereumAccount::rlp_decode(&account_proof.value).unwrap();
                /* Example of how to access account data:
                let account_balance = decoded_account.balance;
                println!("Account ETH balance: {:?}", account_balance);
                */
            }
            _ => {}
        }
        // Verify the proof against the state root
        assert!(proof.verify(&input.state_root).unwrap());
    }

    // commit the helios root that we used for verification as an output
    input.state_root.to_vec()
}
