use hex;
use sp1_sdk::SP1ProofWithPublicValues;
use types::CircuitOutput;
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
    let proof_base64 = "2gFcRWJhZ25TODZOL0VqcUlJSHVMNzJmUnNVdEFBNmlrODU1QVRjbnluUVVXc1J1RHQ3RTl1WUxXUktCRFdHejA0NDN0Q09wUWU0RnRWR05hL0xSOVNQVEd1Q25Ka0gvT2g2TkZjK29wVEdJVmJBQU1TYWlPbWVyallEbVliSlBwdFg2RFBSNVEvbGRlQ3AxcGxZblkvbm9DSUhoZWlBcDd2dno1T1BXSnJJMlNGdm9LYmREVDZ4OE1iRjFpN1dkZU9XTVBrbW55UFg5eHRSWndyaHRkRTBxdzd0VkNJU1ZwcnJicjdNcUwxWnlIbVdZUk9JSkFRRkFPWS9OVnRjLzlWTjJWODliQTl5RjcreG5xMlNGZk1ITloveEVFNk5rN01SZXVNb00yMERhWWpjL09KckdLVTc4Z1dPd3BYRTFaMDRLVzJrQUxvTS9YQ2lqZ3g5YnBnaHd3SEh1NGM92fhBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFCN0luZHBkR2hrY21GM1gzSmxjWFZsYzNSeklqcGJYU3dpYzNSaGRHVmZjbTl2ZENJNld6RTNNQ3d5TXpRc01qUTNMREl6TERJME9Dd3hORFVzTWpRd0xETXdMRGcxTERRd0xERXlOQ3d5TURnc01UVTVMREV4TkN3eE5EVXNNeXd4TURrc01qZ3NOaXd4TmpFc01UVXdMREl5T1N3MU1pdzROaXd4T1Rjc01UTXdMREV6T1N3eU1ETXNNakF6TERVM0xETTNMREV6WFgwPQ==";
    let proof = valence_coprocessor::Proof::try_from_base64(proof_base64).unwrap();
    let (proof, public_values) = proof.decode().unwrap();
    let output: CircuitOutput = serde_json::from_slice(&public_values[32..]).unwrap();
    println!("Output: {:?}", output);
}
