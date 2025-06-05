/*
    This file contains some test utilities that are used for debugging
    It is not part of the app template and is excluded from the workspace on purpose
*/

use anyhow::Result;
use hex;
use sp1_sdk::SP1ProofWithPublicValues;
use valence_coprocessor::ValidatedBlock;
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

#[cfg(test)]
mod tests {
    use crate::get_helios_block;
    use alloy_primitives::U256;
    use alloy_sol_types::SolValue;
    use common_merkle_proofs::merkle::types::MerkleVerifiable;
    use ethereum_merkle_proofs::merkle_lib::{digest_keccak, types::EthereumProofType};
    use hex;
    use num_bigint::BigUint;
    use serde_json::json;
    use types::CircuitOutput;
    use valence_coprocessor::StateProof;

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
        let proof_base64 = "2gFcRWJhZ25RY01SNnR4QmlsdzQzandLZ0ZmS250MXdZM3huOUpGRklxVmRMclVBWmoxSXVzZVlSdll5bGpudy9INlllR3lEWndhMVJyWElFRnptbkZnb0RldUU5b3NhbTZ3b1Z4b1NuZ2pWaGxHVHdtTXpHL1BRdTI4ZFZUL1JmUFhMZVNlM2lVbHd0a1V6cm9jTzFrdlptaHc0eFZsaTdDaW1GSThrdE94Y0tkRHIyREJGZUdWTWpCU3ZMWEZRekJYajVJZDZFL252NjdqOFdiQkZvUHR4UUJPaTdRS2tXRHlid3VkbzFscllncWdjVmdQRkdNSEZBOTlTQXYrODF0N09nZ3E3QUhnOThTTytCRG5BMVF3T1VKM3NDQUFlOU5kVHhUQ1RuTGYrckt3WUZhMUR0Y3dTREtmeEFGSDgzYUJJcHpMODA2VDNBU3JiRmY3SEY0TWxiNmlTVEU92gHgYmpRTG5QK3plcGljcFVUbXUzZ0tMSGlRSFQrek56aDJoUkdqQmhldm9CMTdJbmRwZEdoa2NtRjNYM0psY1hWbGMzUnpJanBiZXlKcFpDSTZNQ3dpYjNkdVpYSWlPaUl3ZUdRNVFUSXpZalU0WlRZNE5FSTVPRFZHTmpZeFEyVTNNREExUVVFNFJURXdOak13TVRVd1l6RWlMQ0p5WldSbGJYQjBhVzl1WDNKaGRHVWlPbHN4TURBd01EQXdNREJkTENKemFHRnlaWE5mWVcxdmRXNTBJanBiTVRBd1hTd2ljbVZqWldsMlpYSWlPaUp1WlhWMGNtOXVNVFJ0YkhCa05EaHJOWFpyWlhObGREUjROMlkzT0cxNWVqTnRORGRxWTJGNE0zbHphbXR3SW4xZExDSnpkR0YwWlY5eWIyOTBJanBiTlRrc01UQTFMREUzTkN3eE1EWXNNakVzT0N3eU5EQXNNalEyTERJMExESXlNeXd4TnpNc01UQXNPRElzTVRFeExEWTRMRFlzTVRFeUxERXhOQ3cyT0N3Mk5pd3hNeklzTWpNMUxERTNNU3d4TURJc01UUXhMREl3TXl3eE1qZ3NNVEV5TERFM09Td3hOVGtzTVRRNExESXlYWDA9";
        let proof = valence_coprocessor::Proof::try_from_base64(proof_base64).unwrap();
        let (_, public_values) = proof.decode().unwrap();
        let output: CircuitOutput = serde_json::from_slice(&public_values[32..]).unwrap();
        println!("Output: {:?}", output);
    }

    #[tokio::test]
    async fn test_get_state_proof() -> Result<(), anyhow::Error> {
        let encoded = (1, 9).abi_encode();
        let contract_address = "0xf2B85C389A771035a9Bd147D4BF87987A7F9cf98";
        let key_hash = digest_keccak(encoded.as_slice());
        println!("Key hash: {:?}", hex::encode(key_hash));
        let key_uint = U256::from_be_slice(&key_hash);
        let next_key = key_uint + U256::from(3);
        let next_key_hash: [u8; 32] = next_key.to_be_bytes();
        let next_key_hash_hex = hex::encode(next_key_hash);
        println!("Next key hash: {:?}", next_key_hash_hex);
        

        let client = reqwest::Client::new();
        let base_key = "ec8156718a8372b1db44bb411437d0870f3e3790d4a08526d024ce1b0b668f6c";

        let response = client
            .post("http://165.1.70.239:7777/")
            .header("Content-Type", "application/json")
            .json(&json!({
                "address": "0xf2B85C389A771035a9Bd147D4BF87987A7F9cf98",
                "ethereum_url": "https://eth-mainnet.g.alchemy.com/v2/D1CbidVntzlEbD4x7iyHnZZaPWzvDe9I",
                "height": 
                22630563,
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
                let mut value: BigUint = BigUint::ZERO;
                let stored_value = storage_proof.get_stored_value()[0..].to_vec();
                if stored_value.len() > 1 {
                    value = BigUint::from_bytes_be(
                        &U256::from_be_slice(&storage_proof.get_stored_value()[1..])
                            .to_be_bytes::<32>(),
                    );
                } else {
                    value = BigUint::from_bytes_be(
                        &U256::from_be_slice(&storage_proof.get_stored_value()).to_be_bytes::<32>(),
                    );
                }

                println!("Value: {:?}", value);
                /*let is_valid = storage_proof.clone().verify(
                    hex::decode("3b69ae6a1508f0f618dfad0a526f44067072444284ebab668dcb8070b39f9416")
                        .unwrap()
                        .as_slice(),
                )?;*/
                //assert!(is_valid);
            }
            _ => {
                panic!("Unexpected proof type");
            }
        }

        Ok(())
    }
}
