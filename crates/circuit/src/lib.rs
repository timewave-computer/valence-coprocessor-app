#![no_std]

extern crate alloc;

use alloc::{string::ToString, vec::Vec};
use cosmwasm_std::{coins, to_json_binary, Uint64};
use valence_authorization_utils::{
    authorization::{AtomicSubroutine, AuthorizationMsg, Priority, Subroutine},
    authorization_message::{Message, MessageDetails, MessageType},
    domain::Domain,
    function::AtomicFunction,
    msg::ProcessorMessage,
    zk_authorization::ZkMessage,
};
use valence_clearing_queue::msg::{FunctionMsgs, LibraryConfigUpdate};
use valence_coprocessor::Witness;
use valence_library_utils::{msg::ExecuteMsg, LibraryAccountType};

const SCALE_FACTOR: u128 = 100000000;
const CLEARING_QUEUE_LIBRARY_ADDRESS: &str = "neutron...";
const TOKEN_DENOM: &str = "factory...?";

pub fn circuit(witnesses: Vec<Witness>) -> Vec<u8> {
    let withdrawal_request_id = witnesses[0].as_data().unwrap();
    let withdrawal_request_id = <[u8; 8]>::try_from(withdrawal_request_id).unwrap();
    let withdrawal_request_id = u64::from_le_bytes(withdrawal_request_id);

    // HERE WE NEED TO GET THE WITHDRAWAL REQUEST FROM THE VAULT AND VERIFY THE PROOFS
    // Let's assume that we have it, for now.
    let withdrawal_request_recipient = "recipient_address".to_string();
    let withdrawal_request_redemption_rate: u128 = 100000001; // Example redemption rate
    let withdrawal_request_shares_amount: u128 = 100; // Example amount

    // Calculate the amounts to be paid out by doing (shares × current_redemption_rate) / initial_redemption_rate
    let withdrawal_request_amount = (withdrawal_request_shares_amount
        * withdrawal_request_redemption_rate)
        / SCALE_FACTOR;

    let clearing_queue_msg: ExecuteMsg<FunctionMsgs, LibraryConfigUpdate> =
        ExecuteMsg::ProcessFunction(FunctionMsgs::RegisterObligation {
            recipient: withdrawal_request_recipient,
            payout_coins: coins(withdrawal_request_amount, TOKEN_DENOM),
            id: Uint64::from(withdrawal_request_id),
        });
    let processor_msg = ProcessorMessage::CosmwasmExecuteMsg {
        msg: to_json_binary(&clearing_queue_msg).unwrap(),
    };

    let function = AtomicFunction {
        domain: Domain::Main,
        message_details: MessageDetails {
            message_type: MessageType::CosmwasmExecuteMsg,
            message: Message {
                name: "process_function".to_string(),
                params_restrictions: None,
            },
        },
        contract_address: LibraryAccountType::Addr(CLEARING_QUEUE_LIBRARY_ADDRESS.to_string()),
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

    let msg = ZkMessage {
        registry: 1,
        block_number: 0,
        domain: Domain::Main,
        authorization_contract: None,
        message,
    };

    serde_json::to_vec(&msg).unwrap()
}
