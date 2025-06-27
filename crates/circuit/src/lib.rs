#![no_std]

extern crate alloc;

use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use alloy_primitives::{Address, Bytes};
use alloy_sol_types::{sol, SolCall, SolValue};
use valence_coprocessor::Witness;

// The library this will be executed on:
const FORWARDER_LIBRARY_CONTRACT: &str = "0xcF8f8313C587c6Ec6E49Be286942D451D4E0908A";

/// Main circuit function for token transfer validation
pub fn circuit(witnesses: Vec<Witness>) -> Vec<u8> {
    // Ensure we have the expected number of witnesses
    assert_eq!(
        witnesses.len(),
        0,
        "Expected no witnesses"
    );

    // Generate ZkMessage
    let zk_message = generate_zk_message();

    // Return ABI-encoded ZkMessage
    zk_message.abi_encode()
}

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

    /// Transfer function call for IBC Eureka transfer
    function forward() external;
}

/// Generate ZkMessage for Valence Authorization contract
fn generate_zk_message() -> ZkMessage {
    // Create the transfer function call with validated fees and empty memo
    let forward_call = forwardCall {};

    // ABI encode the transfer call
    let encoded_transfer_call = forward_call.abi_encode();

    let atomic_function = AtomicFunction {
        contractAddress: FORWARDER_LIBRARY_CONTRACT.parse().unwrap(),
    };

    // Create retry logic with NoRetry for atomic execution
    let retry_logic = RetryLogic {
        times: RetryTimes {
            retryType: RetryTimesType::NoRetry,
            amount: 0,
        },
        interval: Duration {
            durationType: DurationType::Time,
            value: 0,
        },
    };

    // Create AtomicSubroutine
    let atomic_subroutine = AtomicSubroutine {
        functions: alloc::vec![atomic_function],
        retryLogic: retry_logic,
    };

    // Encode the atomic subroutine
    let encoded_subroutine = atomic_subroutine.abi_encode();

    // Create Subroutine wrapper
    let subroutine = Subroutine {
        subroutineType: SubroutineType::Atomic,
        subroutine: Bytes::from(encoded_subroutine),
    };

    // Create SendMsgs message with the properly encoded transfer call
    let send_msgs = SendMsgs {
        executionId: 1, // Generated execution ID
        priority: Priority::Medium,
        subroutine,
        expirationTime: 0, // No expiration
        messages: alloc::vec![Bytes::from(encoded_transfer_call)],
    };

    // Encode SendMsgs
    let encoded_send_msgs = send_msgs.abi_encode();

    // Create ProcessorMessage
    let processor_message = ProcessorMessage {
        messageType: ProcessorMessageType::SendMsgs,
        message: Bytes::from(encoded_send_msgs),
    };

    // Create final ZkMessage
    ZkMessage {
        registry: 0,                          // Same registry the authorization is created for
        blockNumber: 0,                       // We are not validating it
        authorizationContract: Address::ZERO, // Valid for any contract
        processorMessage: processor_message,
    }
}

#[cfg(test)]
mod tests {
    use alloc::vec;

    use super::*;

    #[test]
    fn test_circuit_valid_transfer() {
        let fee_amount = 957u64; // Valid fee below threshold
        let receiver = "0x33C4DaD158F1E2cCF97bF17d1574d5b7b9f43002";
        let expiration = 1890000000000000u64; // Example expiration timestamp

        let witnesses = vec![
            Witness::Data(fee_amount.to_le_bytes().to_vec()),
            Witness::Data(receiver.as_bytes().to_vec()),
            Witness::Data(expiration.to_le_bytes().to_vec()),
        ];

        let result = circuit(witnesses);

        // Try to decode the ZkMessage to verify it's valid
        let decoded_result = ZkMessage::abi_decode(&result, false);
        assert!(decoded_result.is_ok(), "Should be able to decode ZkMessage");
    }

    #[test]
    #[should_panic(expected = "Fee amount exceeds the maximum allowed limit of 1930")]
    fn test_circuit_excessive_fees() {
        let fee_amount = 2000u64; // Exceeds the maximum allowed fee
        let receiver = "0x33C4DaD158F1E2cCF97bF17d1574d5b7b9f43002";
        let expiration = 1890000000000000u64; // Example expiration timestamp

        let witnesses = vec![
            Witness::Data(fee_amount.to_le_bytes().to_vec()),
            Witness::Data(receiver.as_bytes().to_vec()),
            Witness::Data(expiration.to_le_bytes().to_vec()),
        ];

        circuit(witnesses);
    }
}
