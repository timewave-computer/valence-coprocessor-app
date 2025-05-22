#![no_std]
//! Ethereum state proof validation and verification module.
//!
//! This module provides functionality for:
//! - Validating Ethereum blocks using Groth16 zero-knowledge proofs
//! - Retrieving and verifying Ethereum state proofs (account and storage proofs)
//! - Working with Ethereum merkle proofs for both account and storage data

use ethereum_merkle_proofs::{
    ethereum_rpc::rpc::EvmMerkleRpcClient,
    merkle_lib::types::{EthereumProofType, EthereumSimpleProof},
};
use recursion_types::WrapperCircuitOutputs;
use serde_json::Value;
use sp1_verifier::{Groth16Verifier, GROTH16_VK_BYTES};
use valence_coprocessor::{StateProof, ValidatedBlock};

extern crate alloc;
use alloc::{str, string::ToString, vec::Vec};

/// Validates an Ethereum block using a Groth16 zero-knowledge proof.
///
/// # Arguments
///
/// * `args` - A JSON value containing:
///   * `proof` - Hex-encoded proof bytes as a string
///   * `public_values` - Hex-encoded public values as a string
///   * `vk` - Hex-encoded verification key as a string
///
/// # Returns
///
/// Returns a `ValidatedBlock` containing the verified block number and state root.
///
/// # Errors
///
/// Returns an error if:
/// * Any required fields are missing or invalid
/// * Proof verification fails
/// * Public values cannot be deserialized
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
    let vk_str = str::from_utf8(vk_bytes).expect("Failed to convert vk bytes to string");
    let valid_block = validate(proof_bytes, public_values_bytes, vk_str)?;
    Ok(valid_block)
}

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
pub async fn get_state_proof(args: Value) -> anyhow::Result<StateProof> {
    let address = args["address"]
        .as_str()
        .ok_or(anyhow::anyhow!("address is required"))?;

    let height = args["height"]
        .as_u64()
        .ok_or(anyhow::anyhow!("height is required"))?;

    let ethereum_url = args["ethereum_url"]
        .as_str()
        .ok_or(anyhow::anyhow!("ethereum_url is required"))?;

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

/// Internal function to validate a block using Groth16 proof verification.
///
/// # Arguments
///
/// * `proof_bytes` - Raw proof bytes
/// * `public_values_bytes` - Raw public values bytes
/// * `vk_str` - Verification key as a string
///
/// # Returns
///
/// Returns a `ValidatedBlock` if verification succeeds.
pub fn validate(
    proof_bytes: &[u8],
    public_values_bytes: &[u8],
    vk_str: &str,
) -> anyhow::Result<ValidatedBlock> {
    // verify the wrapper proof from the Helios operator
    Groth16Verifier::verify(proof_bytes, public_values_bytes, vk_str, &GROTH16_VK_BYTES)?;
    // deserialize the public values
    let wrapper_outputs: WrapperCircuitOutputs = borsh::from_slice(public_values_bytes)?;
    let verified_block = ValidatedBlock {
        number: wrapper_outputs.height,
        root: wrapper_outputs.root,
        payload: Vec::new(),
    };
    Ok(verified_block)
}
