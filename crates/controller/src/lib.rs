#![no_std]
extern crate alloc;
use alloc::{string::ToString as _, vec::Vec};
use alloy_sol_types::sol;
use ibc_eureka::types::*;
use serde_json::Value;
use valence_coprocessor::Witness;
use valence_coprocessor_wasm::abi;

// constants that we assert in the circuit
const EXPECTED_ENTRY_CONTRACT: &str = "0x0000000000000000000000000000000000000000";
const EXPECTED_ROOT_HASH: &str = "e274283da590c2e015040a412a73159db7d14b30e8c9dd9a92ccd9463f297230";

// Define Valence contract types using alloy-sol-types
sol! {
    /// Duration type for Valence messages
    enum DurationType {
        Height,
        Time
    }

    /// Duration structure
    struct Duration {
        DurationType durationType;
        uint64 value;
    }

    /// Retry times type
    enum RetryTimesType {
        NoRetry,
        Indefinitely,
        Amount
    }

    /// Retry times structure
    struct RetryTimes {
        RetryTimesType retryType;
        uint64 amount;
    }

    /// Retry logic structure
    struct RetryLogic {
        RetryTimes times;
        Duration interval;
    }

    /// Atomic function structure
    struct AtomicFunction {
        address contractAddress;
    }

    /// Atomic subroutine structure
    struct AtomicSubroutine {
        AtomicFunction[] functions;
        RetryLogic retryLogic;
    }

    /// Subroutine type
    enum SubroutineType {
        Atomic,
        NonAtomic
    }

    /// Subroutine structure
    struct Subroutine {
        SubroutineType subroutineType;
        bytes subroutine;
    }

    /// Priority enum
    enum Priority {
        Medium,
        High
    }

    /// SendMsgs structure
    struct SendMsgs {
        uint64 executionId;
        Priority priority;
        Subroutine subroutine;
        uint64 expirationTime;
        bytes[] messages;
    }

    /// ProcessorMessage type enum
    enum ProcessorMessageType {
        Pause,
        Resume,
        EvictMsgs,
        SendMsgs,
        InsertMsgs
    }

    /// ProcessorMessage structure
    struct ProcessorMessage {
        ProcessorMessageType messageType;
        bytes message;
    }

    /// ZkMessage structure for Valence Authorization
    struct ZkMessage {
        uint64 registry;
        uint64 blockNumber;
        address authorizationContract;
        ProcessorMessage processorMessage;
    }

    /// Fees structure for IBC Eureka transfer
    struct Fees {
        uint256 relayFee;
        address relayFeeRecipient;
        uint64 quoteExpiry;
    }

    /// Transfer function call for IBC Eureka transfer
    function transfer(Fees calldata fees, string calldata memo) external;
}

pub fn get_witnesses(args: Value) -> anyhow::Result<Vec<Witness>> {
    abi::log!(
        "received a proof request with arguments {}",
        serde_json::to_string(&args).unwrap_or_default()
    )?;

    let route_request = RouteRequest {
        amount_in: "1000000000".to_string(),
        source_asset_denom: "0x8236a87084f8B84306f72007F36F2618A5634494".to_string(),
        source_asset_chain_id: "1".to_string(),
        dest_asset_denom: "ibc/DBD9E339E1B093A052D76BECFFDE8435EAC114CF2133346B4D691F3F2068C957"
            .to_string(),
        dest_asset_chain_id: "cosmoshub-4".to_string(),
    };

    let state_proof_request = serde_json::json!({
        "method": "POST",
        "url": "https://api.skip.build/v2/fungible/route",
        "headers": {
            "Content-Type": "application/json"
        },
        "json": route_request
    });
    let response_json = abi::http(&state_proof_request)?;
    let body_bytes: Vec<u8> = response_json["body"]
        .as_array()
        .ok_or("body not an array")
        .unwrap()
        .iter()
        .map(|v| Ok::<u8, &str>(v.as_u64().unwrap() as u8))
        .collect::<Result<Vec<u8>, _>>()
        .unwrap();
    let skip_api_response: SkipApiResponse = serde_json::from_slice(&body_bytes)?;
    abi::log!("skip_api_response: {:?}", skip_api_response)?;
    let value = args["value"].as_u64().unwrap();
    let value = value.to_le_bytes().to_vec();
    Ok([Witness::Data(value)].to_vec())
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
