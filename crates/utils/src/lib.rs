#![no_std]

use alloc::{format, string::String};
use alloy_primitives::U256;
use alloy_sol_types::SolValue;
use ethereum_merkle_proofs::merkle_lib::digest_keccak;
extern crate alloc;
pub fn storage_key(event_idx: u64, idx: usize) -> String {
    // key of the WithdrawRequest dictionary
    let dict_key = (event_idx, 9).abi_encode();
    let key_hash = digest_keccak(&dict_key.as_slice());
    let key_uint = U256::from_be_slice(&key_hash);
    let next_key = key_uint + U256::from(idx);
    let next_key_hash: [u8; 32] = next_key.to_be_bytes();
    let next_key_hash_hex = hex::encode(next_key_hash);
    next_key_hash_hex
}

pub fn string_slot_key(string_key_hex: &str, idx: usize) -> String {
    let hashed_slot = digest_keccak(&hex::decode(string_key_hex).unwrap());
    let current_slot = U256::from_be_slice(&hashed_slot);
    // Calculate the slot for this chunk of the string
    let chunk_slot = current_slot + U256::from(idx);
    format!("{:064x}", chunk_slot)
}
