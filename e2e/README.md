# End-to-End Production SP1 Proving Flow Test

This directory contains comprehensive end-to-end tests for the Valence Coprocessor App's production SP1 proving pipeline.

## Overview

The production flow test validates the complete pipeline for generating SP1 proofs for token transfer validation using the Valence Coprocessor infrastructure.

### What It Tests

1. **Skip API Integration** - Fetches real route and fee data from Skip Protocol
2. **Controller Deployment** - Verifies WASM controller is deployed to coprocessor service  
3. **Witness Generation** - Generates circuit witnesses from Skip API data
4. **SP1 Proof Generation** - Creates cryptographic proofs via SP1 circuit in production mode
5. **ABI Encoding** - Generates properly formatted Valence Authorization contract messages
6. **Validation** - Verifies all security constraints are met

### Production Flow Validation

The test validates these security constraints:
- **Route Validation**: Correct Ethereum → Cosmos Hub transfer path
- **Fee Validation**: Transfer fees below $1.89 USD threshold  
- **Destination Validation**: Matches expected Cosmos address
- **Memo Validation**: Empty memo as required for security
- **SP1 Proof**: Real cryptographic proof generated successfully
- **ABI Encoding**: Proper ZkMessage structure for Valence Authorization contract

## Quick Start

### Prerequisites

1. **Coprocessor Service Running**:
   ```bash
   # From project root
   nix develop --command valence-coprocessor start --coprocessor-path ./valence-coprocessor-service-0.1.0-x86_64-apple-darwin.tar.gz
   ```

2. **Controller Deployed**:
   ```bash
   # From project root  
   nix develop --command build-wasm
   nix develop --command deploy-to-service
   ```

3. **Internet Access**: For Skip API calls

### Run the Test

```bash
# Quick test - use defaults
cd e2e && cargo run --bin run_production_test

# Run with custom configuration
COPROCESSOR_URL=http://localhost:37281 \
CONTROLLER_ID=2a326a320c2a4269241d2f39a6c8e253ae14b9bccb5e7f141d9d1e4223e485bb \
EXPECTED_DESTINATION=cosmos1zxj6y5h3r8k9v7n2m4l1q8w5e3t6y9u0i7o4p2s5d8f6g3h1j4k7l9n2 \
cargo run --bin run_production_test

# Run as cargo test
cargo test test_production_sp1_proving_flow
```

## Expected Output

### Successful Test Run

```
Starting Production SP1 Proving Flow Test
Configuration:
   Coprocessor: http://localhost:37281
   Controller ID: 2a326a320c2a4269241d2f39a6c8e253ae14b9bccb5e7f141d9d1e4223e485bb
   Destination: cosmos1zxj6y5h3r8k9v7n2m4l1q8w5e3t6y9u0i7o4p2s5d8f6g3h1j4k7l9n2
   Fee Threshold: 1890000000000000 wei

Step 1: Skip API integration successful
Step 2: Coprocessor service available  
Step 3: Controller deployment verified
Step 4: SP1 proof generation successful
Step 5: Proof validation results verified

Production SP1 Proving Flow Test Complete!
Total Duration: 45.234s
Success Rate: 5/5 steps passed
```

### Test Flow Details

#### Step 1: Skip API Integration
**Command equivalent**:
```bash
curl -X POST "https://api.skip.build/v2/fungible/msgs" \
  -H "Content-Type: application/json" \
  -d '{
    "amount_in": "1000000000000000",
    "source_asset_denom": "0x8236a87084f8B84306f72007F36F2618A5634494",
    "source_asset_chain_id": "1",
    "dest_asset_denom": "ibc/DBD9E339E1B093A052D76BECFFDE8435EAC114CF2133346B4D691F3F2068C957", 
    "dest_asset_chain_id": "cosmoshub-4",
    "address_list": ["0x1234567890123456789012345678901234567890", "cosmos1zxj..."]
  }'
```

**Expected Response**:
- operations: Array with eureka_transfer operation
- estimated_fees: Array with fee amounts < threshold  
- entry_contract_address: 0xFc2d0487A0ae42ae7329a80dc269916A9184cF7C

#### Step 2: Coprocessor Service Check
**Command equivalent**:
```bash
curl -s http://localhost:37281/api/status
```

**Expected Response**: HTTP 200 indicating service is running

#### Step 3: Controller Deployment Verification  
**Command equivalent**:
```bash
curl -s http://localhost:37281/api/registry/controller/{controller_id}/dev \
  -H "Content-Type: application/json" \
  -d '{"args": {"payload": {"cmd": "validate", ...}}}'
```

**Expected Response**: Successful controller execution

#### Step 4: SP1 Proof Generation (Production Mode)
**Command equivalent**:
```bash  
curl -X POST "http://localhost:37281/api/registry/controller/{controller_id}/prove" \
  -H "Content-Type: application/json" \
  -d '{
    "args": {
      "payload": {
        "cmd": "validate",
        "destination": "cosmos1zxj...",
        "memo": "", 
        "path": "/tmp/validation_result.json",
        "skip_response": { /* Skip API response */ }
      }
    }
  }'
```

**Expected Flow**:
1. Controller generates witnesses from Skip API data
2. SP1 circuit proves the witnesses meet security constraints
3. Returns success=true with base64-encoded SP1 proof  
4. Controller processes proof and generates ABI-encoded ZkMessage

#### Step 5: Proof Validation Results
**Command equivalent**:
```bash
curl -s http://localhost:37281/api/registry/controller/{controller_id}/storage/raw | \
  jq -r '.data' | base64 -d | strings | grep '{"actual_destination"' | jq
```

**Expected Validation Results**:
- route_validation: true (correct Ethereum → Cosmos Hub path)
- destination_validation: true (matches expected cosmos address) 
- fee_validation: true (957 wei < 1.89 USD threshold)
- memo_validation: true (empty memo as required)
- overall_validation_passed: true

## Configuration

### Environment Variables

- `COPROCESSOR_URL`: Coprocessor service URL (default: http://localhost:37281)
- `CONTROLLER_ID`: Deployed controller program ID  
- `EXPECTED_DESTINATION`: Expected Cosmos destination address
- `SKIP_API_KEY`: Skip API key (if required)

### Default Values

```rust
ProductionFlowConfig {
    coprocessor_url: "http://localhost:37281",
    skip_api_url: "https://api.skip.build", 
    controller_id: "2a326a320c2a4269241d2f39a6c8e253ae14b9bccb5e7f141d9d1e4223e485bb",
    expected_destination: "cosmos1zxj6y5h3r8k9v7n2m4l1q8w5e3t6y9u0i7o4p2s5d8f6g3h1j4k7l9n2",
    fee_threshold: 1890000000000000, // 0.00189 LBTC = ~$1.89 USD
    proof_timeout: Duration::from_secs(60),
}
```

## Integration with Valence Authorization Contract

The test validates that the circuit correctly generates ABI-encoded `ZkMessage` structures compatible with the Valence Authorization contract. This includes:

### ZkMessage Structure
```solidity
struct ZkMessage {
    uint64 registryId;
    uint64 blockNumber; 
    address authorizationContract;
    bytes processorMessage;
}
```

### ProcessorMessage (SendMsgs)
```solidity  
struct SendMsgs {
    uint256 executionId;
    Priority priority;
    Subroutine subroutine;
    bytes[] messages;
    uint256 expiration;
}
```

### Eureka Transfer Call
```solidity
function transfer(Fees calldata fees, string calldata memo) external;

struct Fees {
    uint256 relayFee;
    address relayFeeRecipient; 
    uint256 quoteExpiry;
}
```

The circuit ensures proper ABI encoding of these structures for seamless integration with the Valence protocol.
