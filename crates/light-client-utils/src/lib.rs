use serde_json::json;
use sp1_sdk::SP1ProofWithPublicValues;
use valence_coprocessor::{StateProof, ValidatedBlock};
use valence_coprocessor_app_domain::validate;

/// Endpoint for the Helios prover service
const HELIOS_PROVER_ENDPOINT: &str = "http://165.1.70.239:7778/";
/// Verification key for the Helios wrapper proof
const HELIOS_WRAPPER_VK: &str =
    "0x0063a53fc1418a7432356779e09fc81a4c0ad6440162480cecf5309f21c65e3b";

pub fn get_helios_block() -> Result<ValidatedBlock, anyhow::Error> {
    let request = json!({
        "method": "POST",
        "url": HELIOS_PROVER_ENDPOINT,
        "headers": {
            "Content-Type": "application/json"
        },
    });
    let state_proof: SP1ProofWithPublicValues = serde_json::from_value(request)?;
    let state_proof = validate(
        &state_proof.bytes(),
        &state_proof.public_values.to_vec(),
        HELIOS_WRAPPER_VK,
    )?;
    Ok(state_proof)
}

#[test]
fn test_get_latest_helios_block() {
    // get and validate a helios block
    let helios_block = get_helios_block().unwrap();
    // get the latest block from the chain
    println!("Helios block: {:?}", helios_block);
}
