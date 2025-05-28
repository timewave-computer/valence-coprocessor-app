#![no_std]

extern crate alloc;

use alloc::vec::Vec;
use valence_coprocessor::Witness;
use valence_authorization_utils::{
    authorization::AuthorizationMsg, domain::Domain, zk_authorization::ZkMessage,
};

pub fn circuit(witnesses: Vec<Witness>) -> Vec<u8> {
    let registry = witnesses[0].as_data().unwrap();
    let registry = <[u8; 8]>::try_from(registry).unwrap();
    let registry = u64::from_le_bytes(registry);

    let message = match registry {
        1 => AuthorizationMsg::Pause {},
        2 => AuthorizationMsg::Resume {},
        _ => panic!("Invalid registry"),
    };

    let value = ZkMessage {
        registry,
        block_number: 1,
        domain: Domain::Main,
        authorization_contract: None,
        message,
    };

    serde_json::to_vec(&value).unwrap()
}
