#![no_std]

extern crate alloc;

use alloc::vec::Vec;
use valence_coprocessor::Witness;

pub fn circuit(witnesses: Vec<Witness>) -> Vec<u8> {
    let value = witnesses[0].as_data().unwrap();
    let value = <[u8; 8]>::try_from(value).unwrap();
    let value = u64::from_le_bytes(value);
    let value = value.wrapping_add(1);

    value.to_le_bytes().to_vec()
}
