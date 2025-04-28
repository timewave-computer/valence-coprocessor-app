use alloc::{string::String, vec::Vec};
use valence_coprocessor::Witness;

pub fn run(witnesses: Vec<Witness>) -> String {
    let message = witnesses[0].as_data().unwrap().to_vec();
    let message = String::from_utf8(message).unwrap();

    message
}
