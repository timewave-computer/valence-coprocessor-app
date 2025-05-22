use ethereum_merkle_proofs::{
    ethereum_rpc::rpc::EvmMerkleRpcClient,
    merkle_lib::types::{EthereumProofType, EthereumSimpleProof},
};
use valence_coprocessor::StateProof;

/// Retrieves an Ethereum state proof for a given address and block height.
///
/// This function can return either:
/// * An account proof - when no storage key is provided
/// * A storage proof - when a storage key is provided
///
/// # Arguments
///
/// * `args` - A JSON value containing:
///   * `address` - Ethereum address to get proof for
///   * `height` - Block height to get proof for
///   * `abi_encoded_key_hex` - (Optional) Storage slot key for storage proofs
///
/// # Returns
///
/// Returns a `StateProof` containing:
/// * The proof type (account or storage)
/// * The domain ("ethereum")
/// * The proof bytes
///
/// # Errors
///
/// Returns an error if:
/// * Required fields are missing
/// * RPC request fails
/// * Proof serialization fails
pub async fn get_state_proof(
    address: &str,
    ethereum_url: &str,
    height: u64,
    key: Option<&str>,
) -> anyhow::Result<StateProof> {
    let merkle_prover = EvmMerkleRpcClient {
        rpc_url: ethereum_url.to_string(),
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
            Ok(StateProof {
                domain: "ethereum".to_string(),
                // todo: use the height to get the root
                // we can decide if we want to do this here,
                // requires construction of RPC client to get
                // the header for the specified height.
                root: [0; 32],
                payload: Vec::new(),
                proof: proof_bytes,
            })
        }
        None => {
            // request an account proof
            let account_proof = merkle_prover.get_account_proof(address, height).await;
            let proof = EthereumProofType::Account(account_proof.unwrap());
            let proof_bytes = serde_json::to_vec(&proof)?;
            Ok(StateProof {
                domain: "ethereum".to_string(),
                root: [0; 32],
                payload: Vec::new(),
                proof: proof_bytes,
            })
        }
    }
}
