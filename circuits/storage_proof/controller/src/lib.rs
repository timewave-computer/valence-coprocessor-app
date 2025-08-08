#![cfg_attr(not(test), no_std)]
extern crate alloc;

use core::str::FromStr;

use alloc::{format, string::ToString as _, vec::Vec};
use alloy_primitives::{hex, Address};
use alloy_rpc_types_eth::EIP1186AccountProofResponse;
use serde_json::{json, Value};
use storage_proof_core::{proof::mapping_slot_key, ControllerInputs};
use valence_coprocessor::{StateProof, Witness};
use valence_coprocessor_wasm::abi;

const NETWORK: &str = "eth-mainnet";
const DOMAIN: &str = "ethereum-electra-alpha";

// This component contains off-chain logic executed as Wasm within the
// Valence ZK Coprocessor's sandboxed environment.
//
// This Controller acts as an intermediary between user inputs and the ZK circuit.
// Key responsibilities include:
// - receiving input arguments (often JSON) for proof requests
// - processing inputs to generate a "witness" (private and
//   public data the ZK circuit needs)
// - interacting with the Coprocessor service to initiate proof generation.
//
// The Controller handles proof computation results; it has an entrypoint
// function the Coprocessor calls upon successful proof generation,
// allowing the Controller to store the proof or log information.
//
// expects ControllerInputs serialized as json
pub fn get_witnesses(args: Value) -> anyhow::Result<Vec<Witness>> {
    let args_pretty = serde_json::to_string_pretty(&args)?;
    abi::log!("received a proof request with arguments {args_pretty}")?;

    let witness_inputs: ControllerInputs = serde_json::from_value(args)?;
    let erc20_addr = Address::from_str(&witness_inputs.erc20_addr)?;
    let eth_addr = Address::from_str(&witness_inputs.eth_addr)?;

    let block =
        abi::get_latest_block(DOMAIN)?.ok_or_else(|| anyhow::anyhow!("no valid domain block"))?;

    let root = block.root;
    abi::log!("root: {}", hex::encode(root))?;

    let block = format!("{:#x}", block.number);

    let slot_key = mapping_slot_key(eth_addr, witness_inputs.erc20_balances_map_storage_index);
    let slot_key = format!("{slot_key:#x}");

    abi::log!("storage key = {slot_key}")?;

    let proof = abi::alchemy(
        NETWORK,
        "eth_getProof",
        &json!([erc20_addr, [slot_key], block]),
    )?;

    let proof: EIP1186AccountProofResponse = serde_json::from_value(proof)?;
    abi::log!("proof: {}", serde_json::to_string_pretty(&proof)?)?;
    let proof = serde_json::to_vec(&proof)?;

    let state_proof = StateProof {
        domain: DOMAIN.into(),
        root,
        payload: Default::default(),
        proof,
    };

    let witnesses = [
        // witness 0: eth address state proof
        Witness::StateProof(state_proof),
        // witness 1: neutron addr (destination)
        Witness::Data(witness_inputs.neutron_addr.as_bytes().to_vec()),
    ]
    .to_vec();

    Ok(witnesses)
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
