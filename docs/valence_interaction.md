# Valence Protocol Integration

This document describes how the Token Transfer system integrates with the Valence Authorization contract to execute cross-chain transfers.

## Overview

The ZK circuit generates ABI-encoded `ZkMessage` structures that are compatible with the Valence Authorization contract. These messages contain validated transfer instructions that trigger IBCEurekaTransfer calls on Ethereum.

## ZkMessage Structure

The circuit outputs a complete `ZkMessage` ready for submission to the Valence Authorization contract:

```solidity
struct ZkMessage {
    uint64 registry;                    // TOKEN_TRANSFER_REGISTRY_ID (1001)
    uint64 blockNumber;                 // Current block number (0 for testing)
    address authorizationContract;      // address(0) for permissionless execution
    ProcessorMessage processorMessage;  // The actual transfer instruction
}
```

## ProcessorMessage Structure

The `ProcessorMessage` contains a `SendMsgs` instruction for immediate execution:

```solidity
struct ProcessorMessage {
    ProcessorMessageType messageType;   // SendMsgs (immediate execution)
    bytes message;                      // ABI-encoded SendMsgs struct
}

struct SendMsgs {
    uint64 executionId;                 // Generated execution identifier
    Priority priority;                  // Medium priority
    Subroutine subroutine;             // AtomicSubroutine with transfer function
    uint64 expirationTime;             // 0 (no expiration)
    bytes[] messages;                   // ABI-encoded transfer call
}
```

## Subroutine Structure

The transfer uses an `AtomicSubroutine` with a single function call to the IBCEurekaTransfer contract:

```solidity
struct AtomicSubroutine {
    AtomicFunction[] functions;         // Single function: IBCEurekaTransfer
    RetryLogic retryLogic;             // NoRetry configuration
}

struct AtomicFunction {
    address contractAddress;            // 0xFc2d0487A0ae42ae7329a80dc269916A9184cF7C
}

struct RetryLogic {
    RetryTimes times;                   // NoRetry
    Duration interval;                  // Height-based, value 0
}
```

## Transfer Function Call

The actual transfer call with validated parameters:

```solidity
function transfer(Fees calldata fees, string calldata memo) external;

struct Fees {
    uint256 relayFee;                  // Validated fee from Skip API (< threshold)
    address relayFeeRecipient;         // Fee recipient address
    uint64 quoteExpiry;                // Quote expiration timestamp
}
```

**Security Note:** The `memo` parameter is enforced to be an empty string by the ZK circuit. This prevents unauthorized execution of arbitrary logic on the destination chain.

## Circuit Implementation

The current circuit implementation in `crates/circuit/src/lib.rs` generates these structures:

### 1. Fee Validation
```rust
let fee_amount = u64::from_le_bytes(fee_bytes);
let fees_within_limit = fee_amount <= config.fee_threshold;
```

### 2. Transfer Call Generation
```rust
let transfer_call = vec![
    fee_amount.to_be_bytes().to_vec(),  // fees as bytes
    vec![]                              // empty memo (enforced)
];
```

### 3. AtomicFunction Creation
```rust
let atomic_function = AtomicFunction {
    contractAddress: config.expected_entry_contract.parse::<Address>()?,
};
```

### 4. Complete ZkMessage Assembly
```rust
let zk_message = ZkMessage {
    registry: config.token_transfer_registry_id,
    blockNumber: 0,
    authorizationContract: Address::ZERO,
    processorMessage: processor_message,
};

// Return ABI-encoded message
zk_message.abi_encode()
```

## Integration Flow

1. **Circuit Validation**: Validates route, destination, fees, and memo
2. **ZkMessage Generation**: Creates ABI-encoded message with transfer instructions
3. **Strategist Submission**: Submits ZkMessage to Valence Authorization contract
4. **Valence Processing**: Authorization contract verifies proof and executes transfer
5. **IBCEureka Execution**: Transfer function called with validated parameters

## Configuration Constants

The system uses these constants for Valence integration:

```rust
/// Registry ID for token transfer messages in Valence
pub const TOKEN_TRANSFER_REGISTRY_ID: u64 = 1001;

/// Expected entry contract address (IBCEurekaTransfer)
pub const EXPECTED_ENTRY_CONTRACT: &str = "0xFc2d0487A0ae42ae7329a80dc269916A9184cF7C";
```

## Testing

The implementation can be tested with the e2e test suite:

```bash
cd e2e && cargo test test_production_sp1_proving_flow
```

This validates:
- Correct ZkMessage structure generation
- ABI encoding compatibility
- Integration with coprocessor service
- End-to-end proof generation and validation

## Security Considerations

1. **Empty Memo Enforcement**: Circuit ensures memo is always empty
2. **Fee Validation**: Fees must be below configured threshold
3. **Route Validation**: Only predetermined routes are allowed
4. **Destination Validation**: Only configured destination addresses accepted
5. **Proof Integrity**: ZK proofs ensure all validations passed before execution 