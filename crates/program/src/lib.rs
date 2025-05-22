use reqwest::get;
use serde_json::Value;
use sp1_sdk::SP1ProofWithPublicValues;
use types::CircuitWitness;
use valence_coprocessor::{StateProof, Witness};
use valence_coprocessor_app_domain::validate;
use valence_coprocessor_wasm::abi;
pub mod utils;

const SEPOLIA_RPC_URL: &str = "https://ethereum-sepolia-rpc.publicnode.com";
//const SEPOLIA_HEIGHT: u64 = 17000000;
const HELIOS_PROVER_ENDPOINT: &str = "http://165.1.70.239:7778/";
const HELIOS_WRAPPER_VK: &str =
    "0x0063a53fc1418a7432356779e09fc81a4c0ad6440162480cecf5309f21c65e3b";

pub async fn get_witnesses(args: Value) -> anyhow::Result<Vec<Witness>> {
    // the witness data required to validate the Helios wrapper proof
    // e.g. the Block proof
    // question: do we want to do this here or elsewhere?

    // for now we ask Helios directly for the most recent proof,
    // very soon we will instead pass the helios proof that
    // is associated with this updated through args
    /*let proof_bytes = args["proof"]
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
        .as_bytes();*/

    let addresses = args["addresses"]
        .as_array()
        .ok_or(anyhow::anyhow!("addresses must be an array"))?
        .iter()
        .map(|v| {
            v.as_str()
                .ok_or(anyhow::anyhow!("each address must be a string"))
        })
        .collect::<anyhow::Result<Vec<&str>>>()?;

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

    // the witness data required to validate the Ethereum merkle proofs
    let keys = args["keys"]
        .as_array()
        .ok_or(anyhow::anyhow!("keys must be an array"))?
        .iter()
        .map(|v| {
            v.as_str()
                .ok_or(anyhow::anyhow!("each key must be a string"))
        })
        .collect::<anyhow::Result<Vec<&str>>>()?;

    // pass an empty key if you want an account proof,
    // the key at index i corresponds to the address at index i
    assert_eq!(keys.len(), addresses.len());

    let helios_zk_proof_response = get(HELIOS_PROVER_ENDPOINT).await?;
    let helios_proof_serialized = helios_zk_proof_response.bytes().await?;
    let helios_proof: SP1ProofWithPublicValues = serde_json::from_slice(&helios_proof_serialized)?;
    let valid_block = validate(
        &helios_proof.bytes(),
        &helios_proof.public_values.to_vec(),
        HELIOS_WRAPPER_VK,
    )?;
    let validated_height = valid_block.number;
    let validated_state_root = valid_block.root;

    let mut ethereum_state_proofs: Vec<StateProof> = Vec::new();
    // populate the ethereum_state_proofs vector with the storage and account proofs
    for (key, address) in keys.iter().zip(addresses.iter()) {
        if key.len() == 0 {
            // if the key is "", we want an account proof
            let state_proof =
                utils::get_state_proof(address, SEPOLIA_RPC_URL, validated_height, None).await;
            ethereum_state_proofs.push(state_proof?);
        } else {
            // if the key is not "", we want a storage proof
            let state_proof =
                utils::get_state_proof(address, SEPOLIA_RPC_URL, validated_height, Some(key)).await;
            ethereum_state_proofs.push(state_proof?);
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
