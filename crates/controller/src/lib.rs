#![no_std]
extern crate alloc;

// Core dependencies for the valence coprocessor controller
use alloc::{format, string::ToString, vec::Vec};
use alloy_primitives::U256; // Ethereum primitive types for handling large numbers
use ethereum_merkle_proofs::merkle_lib::types::EthereumProofType;
use serde_json::Value;
use sha3::{Digest, Keccak256}; // Keccak256 hashing for Ethereum storage slot calculation
use types::CircuitWitness;
use utils::storage_key;
use valence_coprocessor::{StateProof, Witness};

// Re-export domain functions to make them available for deployment
pub use valence_coprocessor_app_domain::{get_state_proof, validate_block};
use valence_coprocessor_wasm::abi;

/// Mainnet RPC endpoint for Ethereum network
/// This is the primary endpoint used to fetch blockchain data
const MAINNET_RPC_URL: &str =
    "https://eth-mainnet.g.alchemy.com/v2/D1CbidVntzlEbD4x7iyHnZZaPWzvDe9I";

/// Retrieves and validates witnesses for the circuit computation.
///
/// This is the main function that orchestrates the entire witness generation process.
/// It performs several critical steps:
/// 1. Fetches the latest validated block from Helios light client
/// 2. Retrieves Ethereum state proofs for specific storage slots
/// 3. Handles dynamic string storage by reading multiple consecutive slots
/// 4. Constructs a complete circuit witness for zero-knowledge proof generation
///
/// The function is designed to work with Ethereum's storage layout where:
/// - Fixed-size data is stored in single slots
/// - Dynamic strings are stored across multiple consecutive slots
/// - Storage slots are calculated using Keccak256 hashing
///
/// # Arguments
/// * `args` - JSON value containing:
///   * `addresses` - Array of Ethereum addresses to get proofs for
///   * `keys` - Array of storage keys (empty string for account proofs)
///   * `proof` - Hex-encoded proof bytes for block validation
///   * `public_values` - Hex-encoded public values for block validation
///   * `vk` - Hex-encoded verification key for block validation
///
/// # Returns
/// * `Vec<Witness>` - Vector containing the circuit witness data
///
/// # Errors
/// * If required fields are missing or invalid
/// * If Helios proof validation fails
/// * If state proof retrieval fails
pub fn get_witnesses(args: Value) -> anyhow::Result<Vec<Witness>> {
    // Extract event_idx from the arguments
    let event_idx: u64 = args["event_idx"]
        .as_u64()
        .ok_or_else(|| anyhow::anyhow!("Missing or invalid event_idx in arguments"))?;
    // Step 1: Get the latest validated block from Helios light client
    // This ensures we're working with a cryptographically verified blockchain state
    let block = abi::get_latest_block("ethereum-alpha")?.expect("Failed to get block");
    let validated_height = block.number;
    let validated_state_root = block.root; // This is the Merkle root of the entire Ethereum state

    // Step 2: Define the target contract and storage slots we want to prove
    // This contract address contains the data we need to verify
    let contract_address = "0x0B3B3a2C11D6676816fe214B7F23446D12D762FF";

    // These are the specific storage slots containing our target data
    // Each key represents a different piece of data stored in the contract
    let keys = Vec::from([
        storage_key(event_idx, 0),
        storage_key(event_idx, 1),
        storage_key(event_idx, 2),
    ]);

    // This key represents the starting slot for a dynamic string
    // Strings in Ethereum storage can span multiple slots if they're longer than 32 bytes
    let string_key = storage_key(event_idx, 3);

    // Step 3: Collect state proofs for all fixed-size storage slots
    let mut ethereum_state_proofs: Vec<StateProof> = Vec::new();

    // For each predefined storage key, fetch its Merkle proof
    for key in keys {
        let state_proof_args = serde_json::json!({
            "address": contract_address,
            "key": key,
            "height": validated_height,
            "ethereum_url": MAINNET_RPC_URL
        });

        // Retrieve the state proof from our ethereum-alpha domain
        // This proof demonstrates that the data exists in the validated state root
        let state_proof = abi::get_state_proof("ethereum-alpha", &state_proof_args)?;
        ethereum_state_proofs.push(state_proof);
    }

    // Step 4: Handle dynamic string storage
    // Ethereum stores strings across multiple consecutive storage slots
    // We need to read all chunks until we find an empty slot (end of string)

    // Calculate the actual storage slot for the string using Keccak256
    // This follows Ethereum's storage layout rules for dynamic data
    let hashed_slot = Keccak256::digest(&hex::decode(string_key).unwrap());
    let current_slot = U256::from_be_slice(&hashed_slot);
    let mut i = 0;

    // Iteratively read string chunks until we reach the end
    loop {
        // Calculate the slot for this chunk of the string
        let chunk_slot = current_slot + U256::from(i);
        let chunk_slot_hex = format!("{:064x}", chunk_slot);

        let string_chunk_proof_args = serde_json::json!({
            "address": contract_address,
            "key": chunk_slot_hex,
            "height": validated_height,
            "ethereum_url": MAINNET_RPC_URL
        });

        // Try to get the state proof for this chunk
        let string_chunk_proof =
            match abi::get_state_proof("ethereum-alpha", &string_chunk_proof_args) {
                Ok(proof) => proof,
                Err(_) => {
                    // If we can't get a proof, we've reached the end of the string
                    // This is normal behavior when the string ends
                    break;
                }
            };

        // Parse the proof to check if this chunk contains data
        let simple_proof: EthereumProofType = serde_json::from_slice(&string_chunk_proof.proof)?;
        match simple_proof {
            EthereumProofType::Simple(storage_proof) => {
                // Check if this chunk is empty (indicates end of string)
                if storage_proof.get_stored_value().is_empty() {
                    // We've reached the end of the string, stop reading
                    break;
                }
            }
            _ => {
                // Unexpected proof type - log an error and continue
                abi::log!("Invalid proof type for chunk of receiver string!")?;
            }
        }

        // This chunk contains data, add it to our proofs and continue
        ethereum_state_proofs.push(string_chunk_proof);
        i += 1;
    }

    // Step 5: Construct the final circuit witness
    // This witness contains all the state proofs we've collected plus the validated state root
    // The circuit will use this to prove that all the data existed in the blockchain state
    let circuit_witness = CircuitWitness {
        state_proofs: ethereum_state_proofs, // All the Merkle proofs we collected
        event_idx,
        state_root: validated_state_root, // The cryptographically verified state root
    };

    // Serialize the witness into the format expected by the zero-knowledge circuit
    Ok([Witness::Data(serde_json::to_vec(&circuit_witness)?)].to_vec())
}

/// Main entrypoint for the coprocessor application.
///
/// This function serves as the primary interface for external interactions.
/// It processes incoming requests and routes them to appropriate handlers.
/// Currently supports storage operations but can be extended for other commands.
///
/// # Arguments
/// * `args` - JSON payload containing:
///   * `payload.cmd` - The command to execute ("store", etc.)
///   * `payload.path` - For store command: the storage path
///   * Additional fields depending on the command
///
/// # Returns
/// * `Value` - Echo of the input arguments (may change based on command)
///
/// # Panics
/// * If an unknown command is provided
pub fn entrypoint(args: Value) -> anyhow::Result<Value> {
    // Log the incoming request for debugging and monitoring
    abi::log!(
        "received an entrypoint request with arguments {}",
        serde_json::to_string(&args).unwrap_or_default()
    )?;

    // Extract the command from the payload
    let cmd = args["payload"]["cmd"].as_str().unwrap();

    // Route the request based on the command type
    match cmd {
        "store" => {
            // Handle storage operations
            // Extract the storage path from the payload
            let path = args["payload"]["path"].as_str().unwrap().to_string();

            // Serialize the entire args object as the data to store
            let bytes = serde_json::to_vec(&args).unwrap();

            // Store the data using the coprocessor's storage system
            abi::set_storage_file(&path, &bytes).unwrap();
        }

        // Add new commands here as the application grows
        _ => panic!("unknown entrypoint command"),
    }

    // Return the original arguments (acts as confirmation of processing)
    Ok(args)
}
