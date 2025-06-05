//! Circuit module for processing and verifying withdraw requests from Ethereum state proofs.
//!
//! This module contains the main circuit logic that takes witness data, extracts and verifies
//! withdraw request information from Ethereum storage proofs, and outputs verified withdraw
//! requests along with the state root used for verification.

use core::panic;

use alloy_primitives::{Address, U256};
use common_merkle_proofs::merkle::types::MerkleVerifiable;
use ethereum_merkle_proofs::merkle_lib::types::EthereumProofType;
use num_bigint::BigUint;
use types::{CircuitOutput, CircuitWitness, WithdrawRequest};
use valence_coprocessor::{StateProof, Witness};

use utils::{storage_key, string_slot_key};

mod helper;

/// Processes witness data to extract and verify withdraw requests from Ethereum state proofs.
///
/// This function takes a vector of witnesses containing circuit input data and state proofs,
/// then extracts withdraw request information (ID, owner, redemption rate, shares amount,
/// and receiver address) by verifying multiple Ethereum storage proofs against a state root.
///
/// # Arguments
///
/// * `witnesses` - A vector of `Witness` instances. Expected to contain at least one `Witness::Data`
///   variant with serialized `CircuitWitness` data.
///
/// # Returns
///
/// Returns a serialized `CircuitOutput` as `Vec<u8>` containing:
/// - Verified withdraw requests with all extracted data
/// - The state root used for proof verification
///
/// # Proof Structure
///
/// The function expects state proofs in the following order:
/// 1. **Proof 0**: ID, receiver type and owner information (combined in single storage slot)
/// 2. **Proof 1**: Redemption rate value
/// 3. **Proof 2**: Shares amount value  
/// 4. **Proofs 3+**: Receiver address chunks (split across multiple slots for long addresses)
///
/// All proofs must verify against the same state root and use the expected contract address
/// `0b3b3a2c11d6676816fe214b7f23446d12d762ff`.
pub fn circuit(witnesses: Vec<Witness>) -> Vec<u8> {
    // Extract the first witness which contains our circuit input data
    let circuit_input_witness = witnesses.first().unwrap();
    // this macro isn't necessary, but rust analyzer throws a false positive
    #[allow(unused)]
    let mut circuit_input_serialized: Vec<u8> = Vec::new();

    // Extract the serialized data from the witness
    // We expect all data to be encoded in a single Data field
    match circuit_input_witness {
        Witness::Data(data) => {
            circuit_input_serialized = data.clone();
        }
        Witness::StateProof(_) => panic!(
            "Unexpected Input: For this example template we encode all data in a single field."
        ),
    }

    // Initialize variables to store the extracted withdraw request data
    let mut id: u64 = 0;
    let mut owner: String = "".to_string();
    let mut redemption_rate: BigUint = BigUint::from(0u64);
    let mut shares_amount: BigUint = BigUint::from(0u64);
    let mut receiver: Vec<u8> = Vec::new();

    // Deserialize the CircuitWitness from the input data
    let input: CircuitWitness = serde_json::from_slice(&circuit_input_serialized).unwrap();
    let event_idx = input.event_idx;

    // Collection to store all processed withdraw requests
    let mut withdraw_requests: Vec<WithdrawRequest> = Vec::new();

    // Extract specific proofs from the input state proofs
    // Proof 0: Contains both ID and owner information
    let id_and_owner_proof = input.state_proofs.first().unwrap();
    // Proof 1: Contains the redemption rate
    let redemption_rate_proof = input.state_proofs.get(1).unwrap();
    // Proof 2: Contains the shares amount
    let shares_amount_proof = input.state_proofs.get(2).unwrap();
    // Proofs 3+: Multiple proofs containing parts of the receiver address
    let mut receiver_proofs: Vec<StateProof> = Vec::new();
    for i in 3..input.state_proofs.len() {
        receiver_proofs.push(input.state_proofs.get(i).unwrap().clone());
    }

    // ===== PROCESS ID AND OWNER PROOF =====
    let id_and_owner_proof_type: EthereumProofType =
        serde_json::from_slice(&id_and_owner_proof.proof).unwrap();

    let mut is_receiver_contract: bool = false;

    match &id_and_owner_proof_type {
        EthereumProofType::Simple(storage_proof) => {
            // the address used in the merkle proof must equal the
            // contract address without 0x prefix and all lowercase
            let should_be_contract_address = storage_proof.get_address();
            assert_contract_address(&should_be_contract_address);
            assert_eq!(
                hex::encode(storage_proof.get_key()),
                storage_key(event_idx, 0)
            );

            let stored_value = storage_proof.get_stored_value();

            is_receiver_contract = stored_value[stored_value.len() - 29] != 0;

            // Extract ID from the last 8 bytes of the stored value
            let id_bytes = &stored_value[stored_value.len() - 8..];
            id = u64::from_be_bytes(id_bytes.try_into().unwrap());

            // Extract owner address from the 20 bytes before the ID
            // Address is the 20 bytes before the index (last 8 bytes)
            let address_start = stored_value.len() - 8 - 20;
            let address_end = stored_value.len() - 8;
            owner = Address::from_slice(&stored_value[address_start..address_end]).to_string();

            // Verify this proof against the provided state root
            assert!(storage_proof.verify(&input.state_root).unwrap());
        }
        _ => {}
    }

    // ===== PROCESS REDEMPTION RATE PROOF =====
    let redemption_rate_proof_type: EthereumProofType =
        serde_json::from_slice(&redemption_rate_proof.proof).unwrap();
    match &redemption_rate_proof_type {
        EthereumProofType::Simple(storage_proof) => {
            // the address used in the merkle proof must equal the
            // contract address without 0x prefix and all lowercase
            let should_be_contract_address = storage_proof.get_address();
            assert_contract_address(&should_be_contract_address);
            assert_eq!(
                hex::encode(storage_proof.get_key()),
                storage_key(event_idx, 1)
            );
            let redemption_rate_rlp = &storage_proof.get_stored_value();

            // Handle RLP encoding: if more than 1 byte, skip the first RLP length byte
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
            // Verify this proof against the provided state root
            assert!(storage_proof.verify(&input.state_root).unwrap());
        }
        _ => {}
    }

    // ===== PROCESS SHARES AMOUNT PROOF =====
    let shares_amount_proof_type: EthereumProofType =
        serde_json::from_slice(&shares_amount_proof.proof).unwrap();
    match &shares_amount_proof_type {
        EthereumProofType::Simple(storage_proof) => {
            // the address used in the merkle proof must equal the
            // contract address without 0x prefix and all lowercase
            let should_be_contract_address = storage_proof.get_address();
            assert_contract_address(&should_be_contract_address);
            assert_eq!(
                hex::encode(storage_proof.get_key()),
                storage_key(event_idx, 2)
            );
            let shares_rlp = &storage_proof.get_stored_value();

            // Handle RLP encoding: if more than 1 byte, skip the first RLP length byte
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
            // Verify this proof against the provided state root
            assert!(storage_proof.verify(&input.state_root).unwrap());
        }
        _ => {}
    }

    // ===== PROCESS RECEIVER ADDRESS PROOFS =====
    // The receiver address is split across multiple storage proofs
    // Each proof contains a chunk of the full receiver string
    for (idx, proof) in receiver_proofs.iter().enumerate() {
        // if we have 2 proofs it's a 46 byte address,
        // if we have 3 proofs it's a 66 byte address
        let proof_type: EthereumProofType = serde_json::from_slice(&proof.proof).unwrap();
        match &proof_type {
            EthereumProofType::Simple(storage_proof) => {
                // the address used in the merkle proof must equal the
                // contract address without 0x prefix and all lowercase
                let should_be_contract_address = storage_proof.get_address();
                assert_contract_address(&should_be_contract_address);
                // the key used in this merkle proof
                // should match the expected key for the string slot
                let expected_key = string_slot_key(&storage_key(event_idx, 3), idx);
                assert_eq!(hex::encode(storage_proof.get_key()), expected_key);
                receiver.extend_from_slice(&storage_proof.get_stored_value()[1..]);
                // Verify this proof against the provided state root
                assert!(storage_proof.verify(&input.state_root).unwrap());
            }
            _ => {}
        }
    }

    // ===== FINALIZE RECEIVER ADDRESS =====
    // decode receiver from rlp-decoded bytes
    let receiver: String = String::from_utf8_lossy(&helper::truncate_neutron_address(
        receiver,
        is_receiver_contract,
    ))
    .to_string();

    // ===== CREATE WITHDRAW REQUEST =====
    // Combine all extracted and verified data into a WithdrawRequest
    let withdraw_request = WithdrawRequest {
        id,
        owner,
        redemption_rate,
        shares_amount,
        receiver,
    };
    withdraw_requests.push(withdraw_request);

    // ===== RETURN CIRCUIT OUTPUT =====
    // commit the verified withdraw requests and the root that was used as an output
    let output = CircuitOutput {
        withdraw_requests,
        state_root: input.state_root,
    };
    serde_json::to_vec(&output).expect("Failed to serialize circuit output")
}

fn assert_contract_address(address: &[u8]) {
    assert_eq!(
        hex::encode(address),
        "0b3b3a2c11d6676816fe214b7f23446d12d762ff"
    );
}
