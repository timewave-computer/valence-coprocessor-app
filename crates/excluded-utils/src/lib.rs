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
    let proof_base64 = "2gFcRWJhZ25TcS9yNGVzZFZCWXRYdXFBWEVkZWt5KzgyZ3VXK2V4ZTdDbHZiNDR4Tk1KTDgrQk5WeUwrak9hUURna0RLRmlaWDdSMndsc1VJaGR0Sy9tZ2V2ZVRlQUZla2VUaUxCNkJpaHNxNU1xUEZ3YkxubzkzYWpNVVYxcS85dXJjQlIwWkNZN3hUcmhxc0pzdi8rSDk0cmtWSnlPMDgrSndEWWFsbkdiSUMvamRLRW1FanU2NUJsM1Vjb1VpdnhrL01halFMWHlLRk0wdHErbkFaYkNXQ1grTCtzZ3VzSE5lOHhpbzZITTErQkZuaWk0WHlocWtwTXNKMTA2T2xIajBzbmh2UUt6K1E0RDNtalUrQzYzbUx6WFU0WGIwaDZ4SHZzU3dzZzArTk0raGx0OEpHeG53S214MVlqL1FMekJLbnNabkdzdWRkSS9HVGhhMnl0bTNic2Vhalk92gGoQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQjdJbmRwZEdoa2NtRjNYM0psY1hWbGMzUnpJanBiZXlKcFpDSTZNQ3dpYjNkdVpYSWlPaUl3ZUdRNVFUSXpZalU0WlRZNE5FSTVPRFZHTmpZeFEyVTNNREExUVVFNFJURXdOak13TVRVd1l6RWlMQ0p5WldSbGJYQjBhVzl1WDNKaGRHVWlPbHN4TURBd01EQXdNREFzTVRNeVhTd2ljMmhoY21WelgyRnRiM1Z1ZENJNld6RXdNRjBzSW5KbFkyVnBkbVZ5SWpvaUluMWRMQ0p6ZEdGMFpWOXliMjkwSWpwYk1UY3dMREl6TkN3eU5EY3NNak1zTWpRNExERTBOU3d5TkRBc016QXNPRFVzTkRBc01USTBMREl3T0N3eE5Ua3NNVEUwTERFME5Td3pMREV3T1N3eU9DdzJMREUyTVN3eE5UQXNNakk1TERVeUxEZzJMREU1Tnl3eE16QXNNVE01TERJd015d3lNRE1zTlRjc016Y3NNVE5kZlE9PQ==";
    let proof = valence_coprocessor::Proof::try_from_base64(proof_base64).unwrap();
    let (_, public_values) = proof.decode().unwrap();
    let output: CircuitOutput = serde_json::from_slice(&public_values[32..]).unwrap();
    println!("Output: {:?}", output);
}

#[tokio::test]
async fn test_get_state_proof() -> Result<(), anyhow::Error> {
    use common_merkle_proofs::merkle::types::MerkleVerifiable;
    let client = reqwest::Client::new();
    let base_key = "ec8156718a8372b1db44bb411437d0870f3e3790d4a08526d024ce1b0b668f6c";

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
