#![no_std]

extern crate alloc;

use alloc::{string::ToString as _, vec::Vec};
use alloy_sol_types::SolValue;
use ethereum_merkle_proofs::merkle_lib::{digest_keccak, types::EthereumProofType};
use hex::encode;
use serde_json::{json, Value};
use valence_coprocessor::Witness;
use valence_coprocessor_wasm::abi::{self, get_state_proof};

/// Mainnet RPC endpoint for Ethereum network
const MAINNET_RPC_URL: &str = "https://erigon-tw-rpc.polkachu.com";
const VAULT_ADDRESS: &str = "0xf2B85C389A771035a9Bd147D4BF87987A7F9cf98";

pub fn get_witnesses(args: Value) -> anyhow::Result<Vec<Witness>> {
    let withdrawal_request_id = args["withdrawal_request_id"].as_u64().unwrap();
    let withdrawal_request_id_bytes = withdrawal_request_id.to_le_bytes().to_vec();

    let block = abi::get_latest_block("ethereum-alpha")?.unwrap();
    let key_mapping = (0, 9).abi_encode();
    let key_hash = digest_keccak(&key_mapping);
    let key_hex = encode(key_hash);

    let proof = get_state_proof(
        "ethereum-alpha",
        &json!({
            "ethereum_url": MAINNET_RPC_URL,
            "address": VAULT_ADDRESS,
            "key": &key_hex,
            "height": block.number,
        }),
    )?;

    let id_and_owner_proof_type: EthereumProofType = serde_json::from_slice(&proof.proof).unwrap();

    match id_and_owner_proof_type {
        EthereumProofType::Simple(ethereum_simple_proof) => {
            let stored_value = ethereum_simple_proof.get_stored_value();
            let id_bytes = &stored_value[stored_value.len() - 8..];
            let id = u64::from_be_bytes(id_bytes.try_into().unwrap());

            abi::log!("ID: {}", id)?;
        }
        _ => {}
    }

    Ok([Witness::Data(withdrawal_request_id_bytes)].to_vec())
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
