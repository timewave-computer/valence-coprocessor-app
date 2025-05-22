use std::env;

use dotenvy::dotenv;
use ethereum_merkle_proofs::{
    ethereum_rpc::rpc::EvmMerkleRpcClient,
    merkle_lib::types::{EthereumProofType, EthereumSimpleProof},
};
use recursion_types::WrapperCircuitOutputs;
use serde_json::Value;
use sp1_verifier::{Groth16Verifier, GROTH16_VK_BYTES};
use valence_coprocessor::{StateProof, ValidatedBlock};

pub fn validate_block(args: Value) -> anyhow::Result<ValidatedBlock> {
    let proof_bytes = args["proof"]
        .as_str()
        .ok_or(anyhow::anyhow!("proof must be a string"))?
        .as_bytes();
    let public_values_bytes = args["public_values"]
        .as_str()
        .ok_or(anyhow::anyhow!("public_values must be a string"))?
        .as_bytes();
    let vk_bytes = args["vk"]
        .as_str()
        .ok_or(anyhow::anyhow!("vk must be a string"))?
        .as_bytes();
    let vk_str = std::str::from_utf8(&vk_bytes).expect("Failed to convert vk bytes to string");
    let valid_block = validate(proof_bytes, public_values_bytes, vk_str)?;
    Ok(valid_block)
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

fn validate(
    proof_bytes: &[u8],
    public_values_bytes: &[u8],
    vk_str: &str,
) -> anyhow::Result<ValidatedBlock> {
    // verify the wrapper proof from the Helios operator
    Groth16Verifier::verify(proof_bytes, public_values_bytes, vk_str, &GROTH16_VK_BYTES)?;
    // deserialize the public values
    let wrapper_outputs: WrapperCircuitOutputs = borsh::from_slice(&public_values_bytes)?;
    let verified_block = ValidatedBlock {
        number: wrapper_outputs.height,
        root: wrapper_outputs.root,
        payload: Vec::new(),
    };
    Ok(verified_block)
}

pub(crate) fn read_ethereum_url() -> String {
    dotenv().ok();
    env::var("ETHEREUM_URL").expect("Missing Ethereum url!")
}

#[cfg(test)]
mod tests {
    use crate::validate;
    // test the helios wrapper proof verification
    #[test]
    fn test_validate_block() {
        let fixture = get_fixture();
        let vk_str =
            std::str::from_utf8(&fixture.vk_bytes).expect("Failed to convert vk bytes to string");
        let valid_block = validate(&fixture.proof_bytes, &fixture.public_values_bytes, vk_str)
            .expect("Failed to validate block");
        println!("Validated block: {:?}", valid_block);
    }

    struct Fixture {
        proof_bytes: Vec<u8>,
        public_values_bytes: Vec<u8>,
        vk_bytes: Vec<u8>,
    }

    fn get_fixture() -> Fixture {
        let proof_bytes = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/fixture/proof.bin"))
            .expect("Failed to read proof file");
        let public_values_bytes = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/fixture/public_outputs.bin"
        ))
        .unwrap();
        let vk_bytes = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/fixture/vk.bin"))
            .expect("Failed to read vk file");
        Fixture {
            proof_bytes,
            public_values_bytes,
            vk_bytes,
        }
    }

    // test an ethereum storage proof
    #[cfg(feature = "dev")]
    #[tokio::test]
    async fn test_simple_state_proof() {
        use alloy::providers::{Provider, ProviderBuilder};
        use common_merkle_proofs::merkle::types::MerkleVerifiable;
        use ethereum_merkle_proofs::merkle_lib::types::{EthereumProofType, EthereumSimpleProof};
        use serde_json::json;
        use std::str::FromStr;
        use url::Url;

        use crate::{get_state_proof, read_ethereum_url};
        let sepolia_height = read_sepolia_height().await.unwrap();
        let storage_slot_key = hex::decode(read_ethereum_vault_balances_storage_key()).unwrap();

        let provider = ProviderBuilder::new().on_http(Url::from_str(&read_ethereum_url()).unwrap());
        let block = provider
            .get_block_by_number(alloy::eips::BlockNumberOrTag::Number(sepolia_height))
            .await
            .unwrap()
            .unwrap();

        // Prepare arguments for get_state_proof
        let args = json!({
            "address": read_ethereum_vault_contract_address(),
            "height": sepolia_height,
            "abi_encoded_key_hex": alloy::hex::encode(&storage_slot_key)
        });

        // Get state proof using our function
        let state_proof = get_state_proof(args).await.unwrap();

        // Deserialize the proof bytes into EthereumProofType
        let proof_type: EthereumProofType = serde_json::from_slice(&state_proof.proof).unwrap();

        // Match on the proof type and verify
        match proof_type {
            EthereumProofType::Simple(simple_proof) => {
                assert!(simple_proof
                    .verify(block.header.state_root.as_slice())
                    .unwrap());
            }
            EthereumProofType::Account(_account_proof) => {
                /*assert!(account_proof
                .verify(block.header.state_root.as_slice())
                .unwrap());*/
                panic!("Expected Simple proof but got Account proof");
            }
            _ => {
                panic!("Unsupported EthereumProofType: The MVP only supports SimpleProof and AccountProof");
            }
        }
        // note that alernatively we can just call EthereumProofType::verify()
        // on either an AccountProof or SimpleProof, if we don't care about the details.
    }

    #[cfg(feature = "dev")]
    async fn read_sepolia_height() -> Result<u64, anyhow::Error> {
        use alloy::providers::{Provider, ProviderBuilder};
        use std::str::FromStr;
        use url::Url;

        use crate::read_ethereum_url;
        let provider = ProviderBuilder::new().on_http(Url::from_str(&read_ethereum_url())?);
        let block = provider
            .get_block_by_number(alloy::eips::BlockNumberOrTag::Latest)
            .await?
            .expect("Failed to get Block!");
        Ok(block.header.number)
    }

    #[cfg(feature = "dev")]
    pub(crate) fn read_ethereum_vault_balances_storage_key() -> String {
        use std::env;

        dotenvy::dotenv().ok();
        env::var("ETHEREUM_SEPOLIA_VAULT_BALANCES_STORAGE_KEY")
            .expect("Missing Sepolia Vault Balances Storage Key!")
    }

    #[cfg(feature = "dev")]
    pub(crate) fn read_ethereum_vault_contract_address() -> String {
        use std::env;

        dotenvy::dotenv().ok();
        env::var("ETHEREUM_SEPOLIA_VAULT_EXAMPLE_CONTRACT_ADDRESS")
            .expect("Missing Sepolia Vault Contract Address!")
    }
}
