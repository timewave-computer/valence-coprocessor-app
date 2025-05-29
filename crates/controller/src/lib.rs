#![no_std]

extern crate alloc;

use alloc::{string::ToString as _, vec::Vec};
use serde_json::Value;
use valence_coprocessor::Witness;
use valence_coprocessor_wasm::abi;

pub fn get_witnesses(args: Value) -> anyhow::Result<Vec<Witness>> {
    let withdrawal_request_id = args["withdrawal_request_id"].as_u64().unwrap();
    let withdrawal_request_id = withdrawal_request_id.to_le_bytes().to_vec();

    Ok([Witness::Data(withdrawal_request_id)].to_vec())
}

pub fn entrypoint(args: Value) -> anyhow::Result<Value> {
    abi::log!(
        "received an entrypoint request with arguments {}",
        serde_json::to_string(&args).unwrap_or_default()
    )?;

    let cmd = args["payload"]["cmd"].as_str().unwrap();

    match cmd {
        "store" => {
            let path = args["payload"]["path"].as_str().unwrap().to_string();
            let bytes = serde_json::to_vec(&args).unwrap();

            abi::set_storage_file(&path, &bytes).unwrap();
        }

        _ => panic!("unknown entrypoint command"),
    }

    Ok(args)
}
