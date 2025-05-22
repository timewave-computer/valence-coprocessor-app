use serde_json::Value;
use valence_coprocessor::Witness;
use valence_coprocessor_wasm::abi;
pub mod utils;

pub fn get_witnesses(args: Value) -> anyhow::Result<Vec<Witness>> {
    // the witness data required to validate the Helios wrapper proof
    // e.g. the Block proof
    // question: do we want to do this here or elsewhere?
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

    Ok([Witness::Data(proof_bytes.to_vec())].to_vec())
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
