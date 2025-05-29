use core::panic;

use alloy_primitives::{Address, U256};
use common_merkle_proofs::merkle::types::MerkleVerifiable;
use ethereum_merkle_proofs::merkle_lib::types::EthereumProofType;
use num_bigint::BigUint;
use types::{CircuitOutput, CircuitWitness, WithdrawRequest};
use valence_coprocessor::{StateProof, Witness};

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

    let mut id: u64 = 0;
    let mut owner: String = "".to_string();
    let mut redemption_rate: BigUint = BigUint::from(0u64);
    let mut shares_amount: BigUint = BigUint::from(0u64);
    let mut receiver: String = "".to_string();

    // Deserialize the CircuitWitness from the input data
    let input: CircuitWitness = serde_json::from_slice(&circuit_input_serialized).unwrap();

    let mut withdraw_requests: Vec<WithdrawRequest> = Vec::new();

    let id_and_owner_proof = input.state_proofs.first().unwrap();
    let redemption_rate_proof = input.state_proofs.get(1).unwrap();
    let shares_amount_proof = input.state_proofs.get(2).unwrap();
    let mut receiver_proofs: Vec<StateProof> = Vec::new();
    for i in 3..input.state_proofs.len() {
        receiver_proofs.push(input.state_proofs.get(i).unwrap().clone());
    }

    let id_and_owner_proof_type: EthereumProofType =
        serde_json::from_slice(&id_and_owner_proof.proof).unwrap();

    match &id_and_owner_proof_type {
        EthereumProofType::Simple(storage_proof) => {
            let stored_value = storage_proof.get_stored_value();
            let id_bytes = &stored_value[stored_value.len() - 8..];
            id = u64::from_be_bytes(id_bytes.try_into().unwrap());

            // Address is the 20 bytes before the index (last 8 bytes)
            let address_start = stored_value.len() - 8 - 20;
            let address_end = stored_value.len() - 8;
            owner = Address::from_slice(&stored_value[address_start..address_end]).to_string();

            assert!(storage_proof.verify(&input.state_root).unwrap());
        }
        _ => {}
    }

    let redemption_rate_proof_type: EthereumProofType =
        serde_json::from_slice(&redemption_rate_proof.proof).unwrap();
    match &redemption_rate_proof_type {
        EthereumProofType::Simple(storage_proof) => {
            let redemption_rate_rlp = &storage_proof.get_stored_value();
            if redemption_rate_rlp.len() > 1 {
                // drop the first byte from rlp
                redemption_rate = BigUint::from_bytes_be(
                    &U256::from_be_slice(&storage_proof.get_stored_value()[1..])
                        .to_be_bytes::<32>(),
                );
            } else {
                // just one byte, use as is
                redemption_rate = BigUint::from_bytes_be(
                    &U256::from_be_slice(&storage_proof.get_stored_value()).to_be_bytes::<32>(),
                );
            }
            assert!(storage_proof.verify(&input.state_root).unwrap());
        }
        _ => {}
    }

    let shares_amount_proof_type: EthereumProofType =
        serde_json::from_slice(&shares_amount_proof.proof).unwrap();
    match &shares_amount_proof_type {
        EthereumProofType::Simple(storage_proof) => {
            let shares_rlp = &storage_proof.get_stored_value();
            if shares_rlp.len() > 1 {
                shares_amount = BigUint::from_bytes_be(
                    &U256::from_be_slice(&storage_proof.get_stored_value()[1..])
                        .to_be_bytes::<32>(),
                );
            } else {
                shares_amount = BigUint::from_bytes_be(
                    &U256::from_be_slice(&storage_proof.get_stored_value()).to_be_bytes::<32>(),
                );
            }
            assert!(storage_proof.verify(&input.state_root).unwrap());
        }
        _ => {}
    }

    for proof in receiver_proofs {
        let proof_type: EthereumProofType = serde_json::from_slice(&proof.proof).unwrap();
        match &proof_type {
            EthereumProofType::Simple(storage_proof) => {
                assert!(storage_proof.verify(&input.state_root).unwrap());
            }
            _ => {}
        }
    }

    // todo: decode the values and populate the WithdrawRequest instance
    let withdraw_request = WithdrawRequest {
        id,
        owner,
        redemption_rate,
        shares_amount,
        receiver,
    };
    withdraw_requests.push(withdraw_request);

    // commit the verified withdraw requests and the root that was used as an output
    let output = CircuitOutput {
        withdraw_requests,
        state_root: input.state_root,
    };
    serde_json::to_vec(&output).expect("Failed to serialize circuit output")
}
