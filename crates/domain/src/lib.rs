#![no_std]
//! Ethereum state proof validation and verification module.
//!
//! This module provides functionality for:
//! - Validating Ethereum blocks using Groth16 zero-knowledge proofs
//! - Retrieving and verifying Ethereum state proofs (account and storage proofs)
//! - Working with Ethereum merkle proofs for both account and storage data

use anyhow::Context;
use recursion_types::WrapperCircuitOutputs;
use serde_json::{json, Value};
use sp1_verifier::{Groth16Verifier, GROTH16_VK_BYTES};
use valence_coprocessor::{StateProof, ValidatedBlock};

extern crate alloc;
use alloc::{str, vec::Vec};
use hex;
use valence_coprocessor_wasm::abi;

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
    let proof_hex = args["proof"]
        .as_str()
        .context("Failed to get proof from args")?;
    let public_values_hex = args["public_values"]
        .as_str()
        .context("Failed to get public_values from args")?;
    let vk_str = args["vk"]
        .as_str()
        .context("Failed to get verifying key from args")?;

    // Convert hex strings to bytes
    let proof_bytes =
        hex::decode(proof_hex).map_err(|_| anyhow::anyhow!("Failed to decode proof hex"))?;
    let public_values_bytes = hex::decode(public_values_hex)
        .map_err(|_| anyhow::anyhow!("Failed to decode public_values hex"))?;

    let valid_block = validate(&proof_bytes, &public_values_bytes, vk_str)?;
    Ok(valid_block)
}

pub fn get_state_proof(args: Value) -> anyhow::Result<StateProof> {
    // Extract parameters from args
    let address = args["address"]
        .as_str()
        .context("Failed to get address from args")?;
    let key = args["key"]
        .as_str()
        .context("Failed to get key from args")?;
    let height = args["height"]
        .as_u64()
        .context("Failed to get height from args")?;
    let ethereum_url = args["ethereum_url"]
        .as_str()
        .context("Failed to get ethereum_url from args")?;

    let state_proof_request = json!({
        "method": "POST",
        "url": "http://165.1.70.239:7777/",
        "headers": {
            "Content-Type": "application/json"
        },
        "json": {
        "address": address,
        "ethereum_url": ethereum_url,
        "height": height,
        "key": key  // empty string for account proof
        }
    });
    let response = abi::http(&state_proof_request)?;
    let body_bytes: Vec<u8> = response["body"]
        .as_array()
        .ok_or("body not an array")
        .unwrap()
        .iter()
        .map(|v| Ok::<u8, &str>(v.as_u64().unwrap() as u8))
        .collect::<Result<Vec<u8>, _>>()
        .unwrap();
    Ok(serde_json::from_slice(&body_bytes)?)
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
    let wrapper_outputs: WrapperCircuitOutputs =
        borsh::from_slice(public_values_bytes).expect("Failed to deserialize public values");
    let verified_block = ValidatedBlock {
        number: wrapper_outputs.height,
        root: wrapper_outputs.root,
        payload: Vec::new(),
    };
    Ok(verified_block)
}
