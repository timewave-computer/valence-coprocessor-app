use std::env;

use dotenvy::dotenv;
use ethereum_merkle_proofs::{
    ethereum_rpc::rpc::EvmMerkleRpcClient,
    merkle_lib::types::{EthereumProofType, EthereumSimpleProof},
};
use serde_json::Value;
use valence_coprocessor::{StateProof, ValidatedBlock};
use valence_coprocessor_wasm::abi;

pub fn validate_block(args: Value) -> anyhow::Result<ValidatedBlock> {
    abi::log!("validate block not implemented, but received {args:?}")?;

    todo!()
}

pub async fn get_state_proof(args: Value) -> anyhow::Result<StateProof> {
    let address = args["address"]
        .as_str()
        .ok_or(anyhow::anyhow!("address is required"))?;

    let height = args["height"]
        .as_u64()
        .ok_or(anyhow::anyhow!("height is required"))?;

    /* Examples to compute the keccak_hash_of_abi_encoded_key_hex:
        1. Stored value under a contract in an account mapping Address -> U256:
            // the storage slot of the mapping
            let slot: U256 = alloy_primitives::U256::from(0);
            // address: the address of the account in the mapping
            let encoded_key = (address, slot).abi_encode();
            // must be hashed under the hood (todo: remove this comment)
            // let keccak_key = digest_keccak(&encoded_key).to_vec();

        2. Stored value in contract at slot:
            // just the hex encoded slot number
            let encoded_key = 0x0000000000000000000000000000000000000000000000000000000000000001

    */

    let key = args["abi_encoded_key_hex"].as_str();

    let merkle_prover = EvmMerkleRpcClient {
        rpc_url: read_ethereum_url().to_string(),
    };

    // if we don't have a key, return an EthereumAccountProof
    // if we do have a key, return an EthereumSimpleProof (=storage proof)
    match key {
        Some(key) => {
            // request a storage proof
            let combined_proof = merkle_prover
                .get_account_and_storage_proof(key, address, height)
                .await;
            let simple_proof = EthereumSimpleProof::from_combined_proof(combined_proof.unwrap());
            let proof = EthereumProofType::Simple(simple_proof);
            let proof_bytes = serde_json::to_vec(&proof)?;
            return Ok(StateProof {
                domain: "ethereum".to_string(),
                // todo: use the height to get the root
                // we can decide if we want to do this here,
                // requires construction of RPC client to get
                // the header for the specified height.
                root: [0; 32],
                payload: Vec::new(),
                proof: proof_bytes,
            });
        }
        None => {
            // request an account proof
            let account_proof = merkle_prover.get_account_proof(address, height).await;
            let proof = EthereumProofType::Account(account_proof.unwrap());
            let proof_bytes = serde_json::to_vec(&proof)?;
            return Ok(StateProof {
                domain: "ethereum".to_string(),
                root: [0; 32],
                payload: Vec::new(),
                proof: proof_bytes,
            });
        }
    }
}

pub(crate) fn read_ethereum_url() -> String {
    dotenv().ok();
    env::var("ETHEREUM_URL").expect("Missing Ethereum url!")
}

#[cfg(test)]
mod tests {
    use sp1_verifier::Groth16Verifier;

    #[test]
    fn test_verify_helios_proof() {
        let proof_bytes = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/fixture/proof.bin"))
            .expect("Failed to read proof file");
        let public_values_bytes = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/fixture/public_outputs.bin"
        ))
        .unwrap();
        let vk_bytes = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/fixture/vk.bin"))
            .expect("Failed to read vk file");
        let vk_str = std::str::from_utf8(&vk_bytes).expect("Failed to convert vk bytes to string");
        let groth16_vk: &[u8] = *sp1_verifier::GROTH16_VK_BYTES;
        Groth16Verifier::verify(&proof_bytes, &public_values_bytes, vk_str, groth16_vk)
            .expect("Failed to verify");
    }
}
