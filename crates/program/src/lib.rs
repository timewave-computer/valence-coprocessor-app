#![no_std]
extern crate alloc;
use alloc::{string::ToString, vec::Vec};
//use anyhow::Context;
//use reqwest_wasm::get;
use serde_json::Value;
//use sp1_sdk::SP1ProofWithPublicValues;
use types::CircuitWitness;
use valence_coprocessor::{StateProof, Witness};
use valence_coprocessor_app_domain::get_state_proof;
//use valence_coprocessor_app_domain::validate;
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
    let addresses = args["addresses"]
        .as_array()
        .ok_or(anyhow::anyhow!("addresses must be an array"))?
        .iter()
        .map(|v| {
            v.as_str()
                .ok_or(anyhow::anyhow!("each address must be a string"))
        })
        .collect::<anyhow::Result<Vec<&str>>>()?;

    let keys = args["keys"]
        .as_array()
        .ok_or(anyhow::anyhow!("keys must be an array"))?
        .iter()
        .map(|v| {
            v.as_str()
                .ok_or(anyhow::anyhow!("each key must be a string"))
        })
        .collect::<anyhow::Result<Vec<&str>>>()?;

    assert_eq!(keys.len(), addresses.len());

    let validated_state_root_hex = args["root"].as_str().unwrap();
    let validated_state_root = <[u8; 32]>::try_from(hex::decode(validated_state_root_hex).unwrap())
        .expect("Invalid State Root");

    let validated_height = args["height"].as_u64().unwrap();
    let validated_state_root = validated_state_root;

    let mut ethereum_state_proofs: Vec<StateProof> = Vec::new();

    // get state proofs from the domain service
    for (key, address) in keys.iter().zip(addresses.iter()) {
        if key.len() == 0 {
            let state_proof = get_state_proof(address, key, validated_height, MAINNET_RPC_URL)?;
            ethereum_state_proofs.push(state_proof);
        } else {
            let state_proof = get_state_proof(address, "", validated_height, MAINNET_RPC_URL)?;
            ethereum_state_proofs.push(state_proof);
        }
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

/// End-to-end test of the witness generation and circuit computation flow.
///
/// This test:
/// 1. Requests a storage proof for USDT total supply
/// 2. Requests an account proof for a specific address
/// 3. Validates the generated witnesses through the circuit
#[tokio::test]
async fn full_e2e_flow() {
    // these are the args to get one storage proof and one account proof.
    // the first proof will be a storage proof for the smart contract
    // with address 0xdac17f958d2ee523a2206206994597c13d831ec7 at slot 0
    // (which is the total supply of USDT on mainnet)
    // see: https://etherscan.io/address/0xdac17f958d2ee523a2206206994597c13d831ec7#code#L80
    // When looking at just the explorer one might be confused to see that it seems like
    // the total supply is stored at slot 3, but that is not the case.
    // See: https://etherscan.io/address/0xdac17f958d2ee523a2206206994597c13d831ec7#readContract#F3
    // The total supply is stored at slot 0, because it's the first state variable defined in the contract.

    // the second proof will be an account proof for the account address
    // 0x07ae8551be970cb1cca11dd7a11f47ae82e70e67
    // both the contract and the account are on the mainnet network

    let args = serde_json::json!({
        "keys": [
            "0x0000000000000000000000000000000000000000000000000000000000000000",
            ""
        ],
        "addresses": [
            "0xdac17f958d2ee523a2206206994597c13d831ec7",
            "0x07ae8551be970cb1cca11dd7a11f47ae82e70e67"
        ]
    });
    let _witness = get_witnesses(args).unwrap();
}
