use alloy_primitives::{Address, U256};
use alloy_rlp::{decode_exact, Rlp};
use anyhow::{Context, Result};
use common_merkle_proofs::merkle::types::MerkleVerifiable;
use ethereum_merkle_proofs::merkle_lib::{
    rlp_decode_bytes,
    types::{EthereumAccount, EthereumProofType},
    RlpDecodable,
};
use hex;
use num_bigint::BigUint;
use reqwest::get;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sp1_sdk::SP1ProofWithPublicValues;
use types::CircuitOutput;
use valence_coprocessor::{StateProof, ValidatedBlock};
use valence_coprocessor_app_domain::validate;

/// Endpoint for the Helios prover service
const HELIOS_PROVER_ENDPOINT: &str = "http://165.1.70.239:7778/";
/// Verification key for the Helios wrapper proof
const HELIOS_WRAPPER_VK: &str =
    "0x006beadaace48146e0389403f70b490980e612c439a9294877446cd583e50fce";

pub async fn get_helios_block() -> Result<ValidatedBlock, anyhow::Error> {
    let client = reqwest::Client::new();
    let response = client.get(HELIOS_PROVER_ENDPOINT).send().await.unwrap();
    let hex_str = response.text().await.unwrap();
    let bytes = hex::decode(hex_str)?;
    let state_proof: SP1ProofWithPublicValues = serde_json::from_slice(&bytes)?;
    let state_proof = validate(
        &state_proof.bytes(),
        &state_proof.public_values.to_vec(),
        HELIOS_WRAPPER_VK,
    )?;
    Ok(state_proof)
}

#[tokio::test]
async fn test_get_latest_helios_block() {
    // get and validate a helios block
    let helios_block = get_helios_block().await.unwrap();
    let validated_block_root = hex::encode(helios_block.root);
    // get the latest block from the chain
    println!("Validated block root: {:?}", validated_block_root);
    println!("Validated block height: {:?}", helios_block.number);
}

#[test]
fn test_decode_public_values() {
    let proof_base64 = "2gFcRWJhZ25TQjVKekxZQi9iOUtWbHNTRVJxU01qTXBkVEQ2UDNRVEN3eVdhV3o2cDVvQW9vWkRRbFg4UUNLMFVGREdpVVdzNk9ub1RHc1VVRkJxQ0JiUGRzSjk2NFc5ZUczUnpjT0ZhK2E2VUx6bHNDMUU0NUttTEczRXVqdk9wOEk2a0grcHlCZWJOaGs5bkwvQ2xRbjJQeFpGUE5EeGh5RWhQMXJyY0p0T0RKbXp1UlVLZ0NhL0hvSmZ0cW0zOHFNbE5FRGNkSjh5ZDlVRHNNL0V4em9HOGJ4d3J3WUkycXhEMGk3aUNJMUdyVjZnU2ZZemdrODRCQWVoU0ZCM3ZnSEN1anpMaHM5MnVTWDVoZjFyVDYrakJBUGdNY2l3NG1PUTRlSTFFVGFwbTZLMFJQZEJKK0E5cWt3anRlRFA0US8xeGE4cjd4ZHZnS3lBS1N4WkI2VmdrUlpuKzA92fhBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFCN0luZHBkR2hrY21GM1gzSmxjWFZsYzNSeklqcGJYU3dpYzNSaGRHVmZjbTl2ZENJNld6RTNNQ3d5TXpRc01qUTNMREl6TERJME9Dd3hORFVzTWpRd0xETXdMRGcxTERRd0xERXlOQ3d5TURnc01UVTVMREV4TkN3eE5EVXNNeXd4TURrc01qZ3NOaXd4TmpFc01UVXdMREl5T1N3MU1pdzROaXd4T1Rjc01UTXdMREV6T1N3eU1ETXNNakF6TERVM0xETTNMREV6WFgwPQ==";
    let proof = valence_coprocessor::Proof::try_from_base64(proof_base64).unwrap();
    let (_, public_values) = proof.decode().unwrap();
    let output: CircuitOutput = serde_json::from_slice(&public_values[32..]).unwrap();
    println!("Output: {:?}", output);
}

#[tokio::test]
async fn test_get_state_proof() -> Result<(), anyhow::Error> {
    let client = reqwest::Client::new();
    let base_key = "0xec8156718a8372b1db44bb411437d0870f3e3790d4a08526d024ce1b0b668f6b";

    let response = client
        .post("http://165.1.70.239:7777/")
        .header("Content-Type", "application/json")
        .json(&json!({
            "address": "0xf2B85C389A771035a9Bd147D4BF87987A7F9cf98",
            "ethereum_url": "https://erigon-tw-rpc.polkachu.com",
            "height": 22580997,
            "key": base_key
        }))
        .send()
        .await
        .unwrap();

    let body_bytes = response.bytes().await.unwrap();
    let result: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    let proof: StateProof = serde_json::from_value(result).unwrap();
    let proof: EthereumProofType = serde_json::from_slice(&proof.proof).unwrap();

    match &proof {
        EthereumProofType::Simple(storage_proof) => {
            let is_valid = storage_proof.clone().verify(
                hex::decode("aaeaf717f891f01e55287cd09f7291036d1c06a196e53456c5828bcbcb39250d")
                    .unwrap()
                    .as_slice(),
            )?;
            assert!(is_valid);

            println!("value: {:?}", storage_proof.value);
            let receiver: String = decode_exact(&storage_proof.value).unwrap();
            println!("receiver: {:?}", receiver);
        }
        _ => {
            panic!("Unexpected proof type");
        }
    }

    Ok(())
}
