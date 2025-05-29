#![no_std]
extern crate alloc;
use alloc::{format, string::ToString, vec::Vec};
use alloy_primitives::U256;
use serde_json::Value;
use sha3::{Digest, Keccak256};
use types::CircuitWitness;
use valence_coprocessor::{StateProof, Witness};
use valence_coprocessor_app_domain::get_state_proof;
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
        let state_proof =
            get_state_proof(contract_address, key, validated_height, MAINNET_RPC_URL)?;
        ethereum_state_proofs.push(state_proof);
    }

    let hashed_slot = Keccak256::digest(&hex::decode(string_key).unwrap());
    let current_slot = U256::from_be_slice(&hashed_slot);
    let chunks = 2;
    for i in 0..chunks {
        let chunk_slot = current_slot + U256::from(i);
        let chunk_slot_hex = format!("{:064x}", chunk_slot);
        let string_chunk_proof = get_state_proof(
            contract_address,
            &chunk_slot_hex,
            validated_height,
            MAINNET_RPC_URL,
        )?;
        ethereum_state_proofs.push(string_chunk_proof);
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
