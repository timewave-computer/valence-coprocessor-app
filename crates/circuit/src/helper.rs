use alloy_primitives::U256;
use alloy_sol_types::SolValue;
use ethereum_merkle_proofs::merkle_lib::digest_keccak;

// we don't know exactly how much padding occurred
// therefore we decide based on the slot size of the string
pub fn truncate_neutron_address(mut data: Vec<u8>, proofs_count: usize) -> Vec<u8> {
    let address_length = receiver_len(proofs_count);
    while data.last() == Some(&0x00) && data.len() > address_length {
        data.pop();
    }
    data
}

// this is inconvenient,
// but we have to do this because we
// don't know exactly how much padding occurred
pub fn receiver_len(proofs_count: usize) -> usize {
    match proofs_count {
        // a 46 byte neutron address
        2 => 46,
        // a 66 byte neutron address
        3 => 66,
        // assume there was a mistake and truncate to 66 bytes
        _ => 66,
    }
}

pub fn storage_key(idx: usize) -> String {
    // key of the WithdrawRequest dictionary
    let dict_key = (0, 9).abi_encode();
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
