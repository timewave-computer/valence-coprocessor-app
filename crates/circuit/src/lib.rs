#![no_std]

extern crate alloc;

use alloc::{vec::Vec, format, string::String};
use valence_coprocessor::Witness;
use alloy_sol_types::{sol, SolValue, SolCall};
use alloy_primitives::{Address, Bytes};

#[cfg(test)]
use alloc::vec;

/// Configuration parameters for the token transfer circuit
#[derive(Debug, Clone)]
pub struct CircuitConfig {
    /// Expected destination address for transfers
    pub expected_destination: String,
    /// Maximum fee threshold in wei
    pub fee_threshold: u64,
    /// Expected source chain ID
    pub expected_source_chain: String,
    /// Expected bridge ID
    pub expected_bridge_id: String,
    /// Expected entry contract address
    pub expected_entry_contract: String,
    /// Token transfer registry ID
    pub token_transfer_registry_id: u64,
}

impl CircuitConfig {
    /// Create a new circuit configuration
    pub fn new(
        expected_destination: String,
        fee_threshold: u64,
        expected_source_chain: String,
        expected_bridge_id: String,
        expected_entry_contract: String,
        token_transfer_registry_id: u64,
    ) -> Self {
        Self {
            expected_destination,
            fee_threshold,
            expected_source_chain,
            expected_bridge_id,
            expected_entry_contract,
            token_transfer_registry_id,
        }
    }
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

    /// Fees structure for IBC Eureka transfer
    struct Fees {
        uint256 relayFee;
        address relayFeeRecipient;
        uint64 quoteExpiry;
    }

    /// Transfer function call for IBC Eureka transfer
    function transfer(Fees calldata fees, string calldata memo) external;
}

/// Validate that route string contains expected components
fn validate_route_components(route_string: &str, config: &CircuitConfig) -> bool {
    route_string.contains(&format!("source_chain:{}", config.expected_source_chain)) &&
    route_string.contains("bridge_type:eureka_transfer") &&
    route_string.contains(&format!("bridge_id:{}", config.expected_bridge_id)) &&
    route_string.contains(&format!("entry_contract:{}", config.expected_entry_contract))
}

/// Generate ZkMessage for Valence Authorization contract
fn generate_zk_message(fee_amount: u64, config: &CircuitConfig) -> ZkMessage {
    // Create the Fees structure for the transfer call
    let fees = Fees {
        relayFee: alloy_primitives::U256::from(fee_amount),
        relayFeeRecipient: Address::ZERO, // Use zero address as default fee recipient
        quoteExpiry: 0, // No expiry for simplicity
    };

    // Create the transfer function call with validated fees and empty memo
    let transfer_call = transferCall {
        fees,
        memo: String::new(), // Empty memo as required
    };

    // ABI encode the transfer call
    let encoded_transfer_call = transfer_call.abi_encode();

    // Create AtomicFunction for IBCEurekaTransfer
    let entry_contract_address = config.expected_entry_contract.parse::<Address>()
        .expect("Invalid entry contract address");
    
    let atomic_function = AtomicFunction {
        contractAddress: entry_contract_address,
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
        registry: config.token_transfer_registry_id,
        blockNumber: 0, // Constant for now
        authorizationContract: Address::ZERO, // Valid for any contract
        processorMessage: processor_message,
    }
}

/// Main circuit function for token transfer validation
pub fn circuit(witnesses: Vec<Witness>, config: &CircuitConfig) -> Vec<u8> {
    // Ensure we have the expected number of witnesses
    assert_eq!(witnesses.len(), 4, "Expected 4 witnesses: fees, route, destination, memo");

    // Extract witness data
    let fee_bytes = witnesses[0].as_data().expect("Failed to get fee data");
    let route_bytes = witnesses[1].as_data().expect("Failed to get route data");
    let destination_bytes = witnesses[2].as_data().expect("Failed to get destination data");
    let memo_bytes = witnesses[3].as_data().expect("Failed to get memo data");

    // Parse fee amount
    let fee_amount = u64::from_le_bytes(
        <[u8; 8]>::try_from(fee_bytes)
            .expect("Fee data must be exactly 8 bytes")
    );

    // Parse route string
    let route_string = core::str::from_utf8(route_bytes)
        .expect("Route data must be valid UTF-8");

    // Parse destination address
    let destination_address = core::str::from_utf8(destination_bytes)
        .expect("Destination data must be valid UTF-8");

    // Parse memo
    let memo = core::str::from_utf8(memo_bytes)
        .expect("Memo data must be valid UTF-8");

    // Validation 1: Route Components Check
    let route_valid = validate_route_components(route_string, config);

    // Validation 2: Destination Address Check
    let destination_valid = destination_address == config.expected_destination;

    // Validation 3: Fee Threshold Check
    let fees_within_limit = fee_amount <= config.fee_threshold;

    // Validation 4: Memo Validation (must be empty)
    let memo_valid = memo.is_empty();

    // Overall validation result
    let validation_passed = route_valid && destination_valid && fees_within_limit && memo_valid;

    // If all validations pass, generate ZkMessage; otherwise return error
    if validation_passed {
        // Generate ZkMessage
        let zk_message = generate_zk_message(fee_amount, config);
        
        // Return ABI-encoded ZkMessage
        zk_message.abi_encode()
    } else {
        // Return validation result for debugging
        let validation_result = ValidationResult {
            validation_passed,
            route_valid,
            destination_valid,
            fees_within_limit,
            memo_valid,
            actual_fee: fee_amount,
            fee_threshold: config.fee_threshold,
        };
        
        serialize_validation_result(&validation_result)
    }
}

/// Validation result structure
struct ValidationResult {
    validation_passed: bool,
    route_valid: bool,
    destination_valid: bool,
    fees_within_limit: bool,
    memo_valid: bool,
    actual_fee: u64,
    fee_threshold: u64,
}

/// Serialize validation result to bytes (simple binary format)
fn serialize_validation_result(result: &ValidationResult) -> Vec<u8> {
    let mut output = Vec::new();
    
    // Pack boolean results into first byte
    let flags = (result.validation_passed as u8) |
                ((result.route_valid as u8) << 1) |
                ((result.destination_valid as u8) << 2) |
                ((result.fees_within_limit as u8) << 3) |
                ((result.memo_valid as u8) << 4);
    
    output.push(flags);
    
    // Add fee amounts
    output.extend_from_slice(&result.actual_fee.to_le_bytes());
    output.extend_from_slice(&result.fee_threshold.to_le_bytes());
    
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circuit_valid_transfer() {
        let config = CircuitConfig::new(
            String::from("cosmos1zxj6y5h3r8k9v7n2m4l1q8w5e3t6y9u0i7o4p2s5d8f6g3h1j4k7l9n2"),
            1890000000000000,
            String::from("1"),
            String::from("EUREKA"),
            String::from("0xFc2d0487A0ae42ae7329a80dc269916A9184cF7C"),
            1001,
        );
        
        let fee_amount = 957u64; // Valid fee below threshold
        let route_string = "source_chain:1|dest_chain:cosmoshub-4|bridge_type:eureka_transfer|bridge_id:EUREKA|entry_contract:0xFc2d0487A0ae42ae7329a80dc269916A9184cF7C";
        let destination = &config.expected_destination;

        let witnesses = vec![
            Witness::Data(fee_amount.to_le_bytes().to_vec()),
            Witness::Data(route_string.as_bytes().to_vec()),
            Witness::Data(destination.as_bytes().to_vec()),
            Witness::Data(b"".to_vec()),
        ];

        let result = circuit(witnesses, &config);
        
        // When all validations pass, we should get an ABI-encoded ZkMessage (longer than validation result)
        assert!(result.len() > 17, "Should return ABI-encoded ZkMessage, not validation result");
        
        // Try to decode the ZkMessage to verify it's valid
        let decoded_result = ZkMessage::abi_decode(&result, false);
        assert!(decoded_result.is_ok(), "Should be able to decode ZkMessage");
        
        let zk_message = decoded_result.unwrap();
        assert_eq!(zk_message.registry, config.token_transfer_registry_id, "Registry ID should match");
        assert_eq!(zk_message.blockNumber, 0, "Block number should be 0");
        assert_eq!(zk_message.authorizationContract, Address::ZERO, "Authorization contract should be zero");
    }

    #[test]
    fn test_circuit_excessive_fees() {
        let config = CircuitConfig::new(
            String::from("cosmos1zxj6y5h3r8k9v7n2m4l1q8w5e3t6y9u0i7o4p2s5d8f6g3h1j4k7l9n2"),
            1890000000000000,
            String::from("1"),
            String::from("EUREKA"),
            String::from("0xFc2d0487A0ae42ae7329a80dc269916A9184cF7C"),
            1001,
        );
        
        let fee_amount = 2000000000000000u64; // Excessive fee above threshold
        let route_string = "source_chain:1|dest_chain:cosmoshub-4|bridge_type:eureka_transfer|bridge_id:EUREKA|entry_contract:0xFc2d0487A0ae42ae7329a80dc269916A9184cF7C";
        let destination = &config.expected_destination;

        let witnesses = vec![
            Witness::Data(fee_amount.to_le_bytes().to_vec()),
            Witness::Data(route_string.as_bytes().to_vec()),
            Witness::Data(destination.as_bytes().to_vec()),
            Witness::Data(b"".to_vec()),
        ];

        let result = circuit(witnesses, &config);
        
        // Check that validation failed due to excessive fees
        assert_eq!(result[0] & 0x01, 0, "Overall validation should fail");
        assert_eq!(result[0] & 0x08, 0, "Fee validation should fail");
    }

    #[test]
    fn test_circuit_invalid_route() {
        let config = CircuitConfig::new(
            String::from("cosmos1zxj6y5h3r8k9v7n2m4l1q8w5e3t6y9u0i7o4p2s5d8f6g3h1j4k7l9n2"),
            1890000000000000,
            String::from("1"),
            String::from("EUREKA"),
            String::from("0xFc2d0487A0ae42ae7329a80dc269916A9184cF7C"),
            1001,
        );
        
        let fee_amount = 957u64;
        let route_string = "source_chain:INVALID|dest_chain:cosmoshub-4|bridge_type:invalid|bridge_id:INVALID";
        let destination = &config.expected_destination;

        let witnesses = vec![
            Witness::Data(fee_amount.to_le_bytes().to_vec()),
            Witness::Data(route_string.as_bytes().to_vec()),
            Witness::Data(destination.as_bytes().to_vec()),
            Witness::Data(b"".to_vec()),
        ];

        let result = circuit(witnesses, &config);
        
        // Check that validation failed due to invalid route
        assert_eq!(result[0] & 0x01, 0, "Overall validation should fail");
        assert_eq!(result[0] & 0x02, 0, "Route validation should fail");
    }

    #[test]
    fn test_circuit_wrong_destination() {
        let config = CircuitConfig::new(
            String::from("cosmos1zxj6y5h3r8k9v7n2m4l1q8w5e3t6y9u0i7o4p2s5d8f6g3h1j4k7l9n2"),
            1890000000000000,
            String::from("1"),
            String::from("EUREKA"),
            String::from("0xFc2d0487A0ae42ae7329a80dc269916A9184cF7C"),
            1001,
        );
        
        let fee_amount = 957u64;
        let route_string = "source_chain:1|dest_chain:cosmoshub-4|bridge_type:eureka_transfer|bridge_id:EUREKA|entry_contract:0xFc2d0487A0ae42ae7329a80dc269916A9184cF7C";
        let destination = "cosmos1wrongaddress1234567890123456789012345678901234567890";

        let witnesses = vec![
            Witness::Data(fee_amount.to_le_bytes().to_vec()),
            Witness::Data(route_string.as_bytes().to_vec()),
            Witness::Data(destination.as_bytes().to_vec()),
            Witness::Data(b"".to_vec()),
        ];

        let result = circuit(witnesses, &config);
        
        // Check that validation failed due to wrong destination
        assert_eq!(result[0] & 0x01, 0, "Overall validation should fail");
        assert_eq!(result[0] & 0x04, 0, "Destination validation should fail");
    }

    #[test]
    fn test_circuit_non_empty_memo() {
        let config = CircuitConfig::new(
            String::from("cosmos1zxj6y5h3r8k9v7n2m4l1q8w5e3t6y9u0i7o4p2s5d8f6g3h1j4k7l9n2"),
            1890000000000000,
            String::from("1"),
            String::from("EUREKA"),
            String::from("0xFc2d0487A0ae42ae7329a80dc269916A9184cF7C"),
            1001,
        );
        
        let fee_amount = 957u64;
        let route_string = "source_chain:1|dest_chain:cosmoshub-4|bridge_type:eureka_transfer|bridge_id:EUREKA|entry_contract:0xFc2d0487A0ae42ae7329a80dc269916A9184cF7C";
        let destination = &config.expected_destination;
        let memo = "unauthorized_memo";

        let witnesses = vec![
            Witness::Data(fee_amount.to_le_bytes().to_vec()),
            Witness::Data(route_string.as_bytes().to_vec()),
            Witness::Data(destination.as_bytes().to_vec()),
            Witness::Data(memo.as_bytes().to_vec()),
        ];

        let result = circuit(witnesses, &config);
        
        // Check that validation failed due to non-empty memo
        assert_eq!(result[0] & 0x01, 0, "Overall validation should fail");
        assert_eq!(result[0] & 0x10, 0, "Memo validation should fail");
    }

    #[test]
    fn test_zkMessage_abi_encoding() {
        let config = CircuitConfig::new(
            String::from("cosmos1zxj6y5h3r8k9v7n2m4l1q8w5e3t6y9u0i7o4p2s5d8f6g3h1j4k7l9n2"),
            1890000000000000,
            String::from("1"),
            String::from("EUREKA"),
            String::from("0xFc2d0487A0ae42ae7329a80dc269916A9184cF7C"),
            1001,
        );
        
        let fee_amount = 957u64; // Valid fee below threshold
        let route_string = "source_chain:1|dest_chain:cosmoshub-4|bridge_type:eureka_transfer|bridge_id:EUREKA|entry_contract:0xFc2d0487A0ae42ae7329a80dc269916A9184cF7C";
        let destination = &config.expected_destination;

        let witnesses = vec![
            Witness::Data(fee_amount.to_le_bytes().to_vec()),
            Witness::Data(route_string.as_bytes().to_vec()),
            Witness::Data(destination.as_bytes().to_vec()),
            Witness::Data(b"".to_vec()),
        ];

        let result = circuit(witnesses, &config);
        
        // Decode the ZkMessage to verify proper structure
        let decoded_result = ZkMessage::abi_decode(&result, false);
        assert!(decoded_result.is_ok(), "Should be able to decode ZkMessage: {:?}", decoded_result.err());
        
        let zk_message = decoded_result.unwrap();
        
        // Verify ZkMessage fields
        assert_eq!(zk_message.registry, 1001, "Registry ID should match config");
        assert_eq!(zk_message.blockNumber, 0, "Block number should be 0");
        assert_eq!(zk_message.authorizationContract, Address::ZERO, "Authorization contract should be zero");
        
        // Decode the SendMsgs from processor message
        let send_msgs_result = SendMsgs::abi_decode(&zk_message.processorMessage.message, false);
        assert!(send_msgs_result.is_ok(), "Should be able to decode SendMsgs: {:?}", send_msgs_result.err());
        
        let send_msgs = send_msgs_result.unwrap();
        
        // Verify SendMsgs fields
        assert_eq!(send_msgs.executionId, 1, "Execution ID should be 1");
        assert_eq!(send_msgs.expirationTime, 0, "Expiration time should be 0");
        assert_eq!(send_msgs.messages.len(), 1, "Should have exactly one message");
        
        // Decode the AtomicSubroutine
        let atomic_subroutine_result = AtomicSubroutine::abi_decode(&send_msgs.subroutine.subroutine, false);
        assert!(atomic_subroutine_result.is_ok(), "Should be able to decode AtomicSubroutine: {:?}", atomic_subroutine_result.err());
        
        let atomic_subroutine = atomic_subroutine_result.unwrap();
        
        // Verify AtomicSubroutine fields
        assert_eq!(atomic_subroutine.functions.len(), 1, "Should have exactly one function");
        
        // Decode the transfer call from messages
        let transfer_call_result = transferCall::abi_decode(&send_msgs.messages[0], false);
        assert!(transfer_call_result.is_ok(), "Should be able to decode transferCall: {:?}", transfer_call_result.err());
        
        let transfer_call = transfer_call_result.unwrap();
        
        // Verify transfer call fields
        assert_eq!(transfer_call.fees.relayFee, alloy_primitives::U256::from(957), "Relay fee should match");
        assert_eq!(transfer_call.fees.relayFeeRecipient, Address::ZERO, "Fee recipient should be zero");
        assert_eq!(transfer_call.fees.quoteExpiry, 0, "Quote expiry should be 0");
        assert_eq!(transfer_call.memo, "", "Memo should be empty");
    }
}
