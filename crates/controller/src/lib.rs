#![no_std]
extern crate alloc;
use alloc::{format, string::ToString, vec::Vec};
use alloy_primitives::U256;
use ethereum_merkle_proofs::merkle_lib::types::EthereumProofType;
use serde_json::Value;
use sha3::{Digest, Keccak256};
use types::CircuitWitness;
use valence_coprocessor::{StateProof, Witness};
// publically expose the domain functions for deployment
pub use valence_coprocessor_app_domain::{get_state_proof, validate_block};
use valence_coprocessor_wasm::abi;

/// Mainnet RPC endpoint for Ethereum network
const MAINNET_RPC_URL: &str = "https://erigon-tw-rpc.polkachu.com";

/// Retrieves and validates witnesses for the circuit computation.
///
/// This function:
/// 1. Takes Ethereum addresses and storage keys as input
/// 2. Fetches the latest Helios proof and validates it
/// 3. Retrieves Ethereum state proofs (account or storage) for each address
/// 4. Constructs and returns the circuit witness
///
/// # Arguments
/// * `args` - JSON value containing:
///   * `addresses` - Array of Ethereum addresses to get proofs for
///   * `keys` - Array of storage keys (empty string for account proofs)
///   * `proof` - Hex-encoded proof bytes for block validation
///   * `public_values` - Hex-encoded public values for block validation
///   * `vk` - Hex-encoded verification key for block validation
///
/// # Returns
/// * `Vec<Witness>` - Vector containing the circuit witness data
///
/// # Errors
/// * If required fields are missing or invalid
/// * If Helios proof validation fails
/// * If state proof retrieval fails
pub fn get_witnesses(args: Value) -> anyhow::Result<Vec<Witness>> {
    let validated_state_root_hex = args["root"].as_str().unwrap();
    let validated_state_root = <[u8; 32]>::try_from(hex::decode(validated_state_root_hex).unwrap())
        .expect("Invalid State Root");

    let validated_height = args["height"].as_u64().unwrap();
    let validated_state_root = validated_state_root;

    let contract_address = "0xf2B85C389A771035a9Bd147D4BF87987A7F9cf98";
    let keys = Vec::from([
        "ec8156718a8372b1db44bb411437d0870f3e3790d4a08526d024ce1b0b668f6b",
        "ec8156718a8372b1db44bb411437d0870f3e3790d4a08526d024ce1b0b668f6c",
        "ec8156718a8372b1db44bb411437d0870f3e3790d4a08526d024ce1b0b668f6d",
    ]);
    let string_key = "ec8156718a8372b1db44bb411437d0870f3e3790d4a08526d024ce1b0b668f6e";

    let mut ethereum_state_proofs: Vec<StateProof> = Vec::new();
    // get the state proofs for non-dynamic data
    // this is straightforward, we just get the state proofs for the keys

    for key in keys {
        let state_proof_args = serde_json::json!({
            "address": contract_address,
            "key": key,
            "height": validated_height,
            "ethereum_url": MAINNET_RPC_URL
        });
        // use the abi call to get the state proof from our ethereum-alpha domain
        let state_proof = get_state_proof(state_proof_args)?;
        ethereum_state_proofs.push(state_proof);
    }

    let hashed_slot = Keccak256::digest(&hex::decode(string_key).unwrap());
    let current_slot = U256::from_be_slice(&hashed_slot);
    let mut i = 0;
    // this loop will get us the merkle proofs for the string values
    loop {
        let chunk_slot = current_slot + U256::from(i);
        let chunk_slot_hex = format!("{:064x}", chunk_slot);
        let string_chunk_proof_args = serde_json::json!({
            "address": contract_address,
            "key": chunk_slot_hex,
            "height": validated_height,
            "ethereum_url": MAINNET_RPC_URL
        });
        let string_chunk_proof = match get_state_proof(string_chunk_proof_args) {
            Ok(proof) => proof,
            Err(_) => {
                // the proof does not exist, stop reading the string
                break;
            }
        };
        let simple_proof: EthereumProofType = serde_json::from_slice(&string_chunk_proof.proof)?;
        match simple_proof {
            EthereumProofType::Simple(storage_proof) => {
                // check if the next chunk slot contains a merkle proof for an empty value
                if storage_proof.get_stored_value().is_empty() {
                    // at this point we have the full receiver string
                    break;
                }
            }
            _ => {
                abi::log!("Invalid proof type for chunk of receiver string!")?;
            }
        }

        ethereum_state_proofs.push(string_chunk_proof);
        i += 1;
    }

    // the final witness for our state proof circuit :D
    // with a real, verified helios root
    let circuit_witness = CircuitWitness {
        state_proofs: ethereum_state_proofs,
        state_root: validated_state_root,
    };

    // commit the ethereum_state_proofs as the circuit witness
    Ok([Witness::Data(serde_json::to_vec(&circuit_witness)?)].to_vec())
}

pub fn entrypoint(args: Value) -> anyhow::Result<Value> {
    abi::log!(
        "received an entrypoint request with arguments {}",
        serde_json::to_string(&args).unwrap_or_default()
    )?;

    let cmd = args["payload"]["cmd"].as_str().unwrap();

    match cmd {
        "store" => {
            let path = args["payload"]["path"].as_str().unwrap().to_string();
            let bytes = serde_json::to_vec(&args).unwrap();

            abi::set_storage_file(&path, &bytes).unwrap();
        }

        _ => panic!("unknown entrypoint command"),
    }

    Ok(args)
}
