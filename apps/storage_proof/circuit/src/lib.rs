use alloy_rpc_types_eth::EIP1186AccountProofResponse;

use storage_proof_core::consts::CW20_ADDR;
use storage_proof_core::proof::verify_proof;
use valence_coprocessor::Witness;

use cosmwasm_std::{to_json_binary, Uint128};
use valence_authorization_utils::{
    authorization::{AtomicSubroutine, AuthorizationMsg, Priority, Subroutine},
    authorization_message::{Message, MessageDetails, MessageType},
    domain::Domain,
    function::AtomicFunction,
    msg::ProcessorMessage,
    zk_authorization::ZkMessage,
};

pub fn circuit(witnesses: Vec<Witness>) -> anyhow::Result<Vec<u8>> {
    assert!(
        witnesses.len() == 2,
        "Expected 2 witnesses: account state proof and neutron addr"
    );

    // extract the witnesses
    let state_proof_bytes = witnesses[0]
        .as_state_proof()
        .expect("Failed to get state proof bytes");
    let neutron_addr_bytes = witnesses[1]
        .as_data()
        .expect("failed to get neutron addr bytes");

    let proof: EIP1186AccountProofResponse = serde_json::from_slice(&state_proof_bytes.proof)
        .expect("failed to deserialize the proof bytes");

    verify_proof(&proof).expect("proof verification failed");

    let neutron_addr = core::str::from_utf8(neutron_addr_bytes)
        .expect("failed to convert neutron addr bytes to str");

    let evm_balance = proof.storage_proof[0].value;
    let evm_balance: u128 = match evm_balance.try_into() {
        Ok(bal) => bal,
        Err(_) => panic!("U256 -> u128 parsing of evm balance failed ({evm_balance})"),
    };

    let zk_msg = build_zk_msg(neutron_addr.to_string(), evm_balance);

    let zk_msg = serde_json::to_vec(&zk_msg)?;

    Ok(zk_msg)
}

pub fn build_zk_msg(recipient: String, amount: u128) -> ZkMessage {
    let mint_cw20_msg = cw20::Cw20ExecuteMsg::Mint {
        recipient,
        amount: Uint128::new(amount),
    };

    let processor_msg = ProcessorMessage::CosmwasmExecuteMsg {
        msg: to_json_binary(&mint_cw20_msg).unwrap(),
    };

    let function = AtomicFunction {
        domain: Domain::Main,
        message_details: MessageDetails {
            message_type: MessageType::CosmwasmExecuteMsg,
            message: Message {
                name: "mint".to_string(),
                params_restrictions: None,
            },
        },
        contract_address: valence_library_utils::LibraryAccountType::Addr(CW20_ADDR.to_string()),
    };

    let subroutine = AtomicSubroutine {
        functions: Vec::from([function]),
        retry_logic: None,
        expiration_time: None,
    };

    let message = AuthorizationMsg::EnqueueMsgs {
        id: 0,
        msgs: Vec::from([processor_msg]),
        subroutine: Subroutine::Atomic(subroutine),
        priority: Priority::Medium,
        expiration_time: None,
    };

    ZkMessage {
        registry: 0,
        block_number: 0,
        domain: Domain::Main,
        authorization_contract: None,
        message,
    }
}
