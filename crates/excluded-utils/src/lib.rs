use alloy_primitives::U256;
use anyhow::Result;
use ethereum_merkle_proofs::merkle_lib::types::EthereumProofType;
use hex;
use num_bigint::BigUint;
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
    let proof_base64 = "2gFcRWJhZ25SSnNJWmRUUWpHelphb01aUUZzUXRpTjFsYnI3aEZFd0NmWm5JRGFuem1XSFFQWlFtbkwvOU5WSnJ6NXRsYUZ6VHBtazBTcXRvWFFVSDBPZC9ocTFqOFkwbTh5eDVjLyt0RTU0YTcyUStmOEhTRFpEMTB1WVNoTElrclhUSGtUcWdsVnVtMThuMTBybnhaWldVR0VScTQzYkMxckl6NmUwVUpJRHBZWEZFcCtIYXJaM3dLeG5zSEVtS1RIcVBFTk8veE5jMW5JTk1QOWh5ZWR2ZUxqZkx3TXY3WEpKK3N3MGRBR3Naa2VuVzdUM29CeTdXVmJnZTJub3hhMmt5WDVnd0NXZ21XVnJjZXlUa3ZhZXFoNHo0ZDB5aFc0SkloeHhuTTNQWmhGQkp5dUE2WnJ5b1dUUjZxb0syVlM5aGkwTTdHT0ExZXV1T3pGSFJaK1YvcEdZYWM92gGgQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQjdJbmRwZEdoa2NtRjNYM0psY1hWbGMzUnpJanBiZXlKcFpDSTZNQ3dpYjNkdVpYSWlPaUl3ZUdRNVFUSXpZalU0WlRZNE5FSTVPRFZHTmpZeFEyVTNNREExUVVFNFJURXdOak13TVRVd1l6RWlMQ0p5WldSbGJYQjBhVzl1WDNKaGRHVWlPbHN4TURBd01EQXdNREJkTENKemFHRnlaWE5mWVcxdmRXNTBJanBiTVRBd1hTd2ljbVZqWldsMlpYSWlPaUlpZlYwc0luTjBZWFJsWDNKdmIzUWlPbHN4TnpBc01qTTBMREkwTnl3eU15d3lORGdzTVRRMUxESTBNQ3d6TUN3NE5TdzBNQ3d4TWpRc01qQTRMREUxT1N3eE1UUXNNVFExTERNc01UQTVMREk0TERZc01UWXhMREUxTUN3eU1qa3NOVElzT0RZc01UazNMREV6TUN3eE16a3NNakF6TERJd015dzFOeXd6Tnl3eE0xMTk=";
    let proof = valence_coprocessor::Proof::try_from_base64(proof_base64).unwrap();
    let (_, public_values) = proof.decode().unwrap();
    let output: CircuitOutput = serde_json::from_slice(&public_values[32..]).unwrap();
    println!("Output: {:?}", output);
}

#[tokio::test]
async fn test_get_state_proof() -> Result<(), anyhow::Error> {
    use common_merkle_proofs::merkle::types::MerkleVerifiable;
    let client = reqwest::Client::new();
    let base_key = "ec8156718a8372b1db44bb411437d0870f3e3790d4a08526d024ce1b0b668f6d";

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
            let stored_value = storage_proof.get_stored_value()[0..].to_vec();
            println!(
                "Stored value: {:?}",
                U256::from_str_radix(&hex::encode(stored_value), 16).unwrap()
            );
            let redemption_rate = &U256::from_be_slice(&storage_proof.get_stored_value());
            println!("Redemption rate: {:?}", redemption_rate);
            let is_valid = storage_proof.clone().verify(
                hex::decode("aaeaf717f891f01e55287cd09f7291036d1c06a196e53456c5828bcbcb39250d")
                    .unwrap()
                    .as_slice(),
            )?;
            assert!(is_valid);
        }
        _ => {
            panic!("Unexpected proof type");
        }
    }

    Ok(())
}
