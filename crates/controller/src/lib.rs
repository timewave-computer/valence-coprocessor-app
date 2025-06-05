#![no_std]

extern crate alloc;

use alloc::{string::{String, ToString as _}, vec::Vec, format};
use serde_json::Value;
use valence_coprocessor::Witness;
use valence_coprocessor_wasm::abi;

/// Extract fee information from Skip API response
fn extract_fee_data(skip_response: &Value) -> anyhow::Result<u64> {
    abi::log!("Extracting fee data from Skip API response")?;
    
    // Look for estimated_fees array in the response
    let fees = skip_response["estimated_fees"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("No estimated_fees found in Skip API response"))?;
    
    let mut total_fees = 0u64;
    
    for fee in fees {
        if let Some(amount_str) = fee["amount"].as_str() {
            if let Ok(amount) = amount_str.parse::<u64>() {
                total_fees = total_fees.wrapping_add(amount);
                abi::log!("Found fee: {} LBTC wei", amount)?;
            }
        }
    }
    
    abi::log!("Total fees extracted: {} LBTC wei", total_fees)?;
    Ok(total_fees)
}

/// Extract route data from Skip API response
fn extract_route_data(skip_response: &Value) -> anyhow::Result<String> {
    abi::log!("Extracting route data from Skip API response")?;
    
    // Look for operations array and find eureka_transfer operation
    let operations = skip_response["operations"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("No operations found in Skip API response"))?;
    
    for operation in operations {
        if operation["type"].as_str() == Some("eureka_transfer") {
            // Extract key route components
            let from_chain = operation["from_chain_id"].as_str().unwrap_or("");
            let to_chain = operation["to_chain_id"].as_str().unwrap_or("");
            let denom_in = operation["denom_in"].as_str().unwrap_or("");
            let denom_out = operation["denom_out"].as_str().unwrap_or("");
            let bridge_id = operation["bridge_id"].as_str().unwrap_or("");
            let entry_contract = operation["entry_contract_address"].as_str().unwrap_or("");
            
            // Build canonical route string for hashing
            let route_string = format!(
                "source_chain:{}|dest_chain:{}|source_denom:{}|dest_denom:{}|bridge_type:eureka_transfer|bridge_id:{}|entry_contract:{}",
                from_chain, to_chain, denom_in, denom_out, bridge_id, entry_contract
            );
            
            abi::log!("Extracted route: {}", route_string)?;
            return Ok(route_string);
        }
    }
    
    Err(anyhow::anyhow!("No eureka_transfer operation found in Skip API response"))
}

/// Extract destination address from the arguments
fn extract_destination_address(args: &Value) -> anyhow::Result<String> {
    args["destination"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("No destination address found in arguments"))
}

pub fn get_witnesses(args: Value) -> anyhow::Result<Vec<Witness>> {
    abi::log!(
        "received a proof request with LBTC transfer arguments {}",
        serde_json::to_string(&args).unwrap_or_default()
    )?;

    // Extract Skip API response from arguments
    let skip_response = &args["skip_response"];
    if skip_response.is_null() {
        return Err(anyhow::anyhow!("No skip_response found in arguments"));
    }

    // Extract fee data (in LBTC wei)
    let total_fees = extract_fee_data(skip_response)?;
    
    // Extract route data
    let route_string = extract_route_data(skip_response)?;
    
    // Extract destination address
    let destination = extract_destination_address(&args)?;
    
    abi::log!("Preparing witnesses: fees={}, route_len={}, dest_len={}", 
              total_fees, route_string.len(), destination.len())?;

    // Prepare witness data for circuit
    let witnesses = [
        Witness::Data(total_fees.to_le_bytes().to_vec()),           // Witness 0: Total fees in LBTC wei
        Witness::Data(route_string.as_bytes().to_vec()),           // Witness 1: Route string for hashing
        Witness::Data(destination.as_bytes().to_vec()),            // Witness 2: Destination address
    ].to_vec();

    Ok(witnesses)
}

pub fn entrypoint(args: Value) -> anyhow::Result<Value> {
    abi::log!(
        "received an LBTC transfer entrypoint request with arguments {}",
        serde_json::to_string(&args).unwrap_or_default()
    )?;

    let cmd = args["payload"]["cmd"].as_str().unwrap();

    match cmd {
        "store" => {
            let path = args["payload"]["path"].as_str().unwrap().to_string();
            
            // Check if this is a validation result
            if let Some(validation_result) = args.get("validation_result") {
                abi::log!("Storing LBTC transfer validation result to {}", path)?;
                
                // Create structured validation response
                let response = serde_json::json!({
                    "transfer_type": "LBTC_IBC_EUREKA",
                    "timestamp": "2025-01-31T00:00:00Z", // Placeholder - in real implementation would use actual time
                    "validation_result": validation_result,
                    "original_args": args
                });
                
                let bytes = serde_json::to_vec(&response).unwrap();
                abi::set_storage_file(&path, &bytes).unwrap();
                
                abi::log!("Successfully stored LBTC transfer validation result")?;
            } else {
                // Store the raw arguments as before for compatibility
                abi::log!("Storing raw arguments to {}", path)?;
                let bytes = serde_json::to_vec(&args).unwrap();
                abi::set_storage_file(&path, &bytes).unwrap();
            }
        }
        
        "validate" => {
            abi::log!("Processing LBTC transfer validation request")?;
            
            // Extract validation inputs
            let fees = args["fees"].as_u64().unwrap_or(0);
            let route_valid = args["route_valid"].as_bool().unwrap_or(false);
            let destination_valid = args["destination_valid"].as_bool().unwrap_or(false);
            let fees_within_limit = args["fees_within_limit"].as_bool().unwrap_or(false);
            
            let validation_passed = route_valid && destination_valid && fees_within_limit;
            
            abi::log!("Validation results: route={}, dest={}, fees={}, overall={}", 
                     route_valid, destination_valid, fees_within_limit, validation_passed)?;
            
            // Store validation results in storage if path provided
            if let Some(storage_path) = args["payload"]["path"].as_str() {
                let validation_response = serde_json::json!({
                    "validation_passed": validation_passed,
                    "route_valid": route_valid,
                    "destination_valid": destination_valid,
                    "fees_within_limit": fees_within_limit,
                    "total_fees_lbtc_wei": fees,
                    "timestamp": "2025-01-31T00:00:00Z"
                });
                
                let bytes = serde_json::to_vec(&validation_response).unwrap();
                abi::set_storage_file(storage_path, &bytes).unwrap();
                abi::log!("Stored validation results to {}", storage_path)?;
            }
        }

        _ => {
            abi::log!("Unknown entrypoint command: {}", cmd)?;
            return Err(anyhow::anyhow!("unknown entrypoint command: {}", cmd));
        }
    }

    Ok(args)
}

/// Generate mock Skip API response for testing
pub fn generate_mock_skip_response(fees_lbtc_wei: u64, use_valid_route: bool) -> Value {
    let (source_denom, dest_denom, bridge_id, entry_contract) = if use_valid_route {
        // Valid LBTC Eureka route from Phase 1
        (
            "0x8236a87084f8B84306f72007F36F2618A5634494",
            "ibc/DBD9E339E1B093A052D76BECFFDE8435EAC114CF2133346B4D691F3F2068C957",
            "EUREKA",
            "0xFc2d0487A0ae42ae7329a80dc269916A9184cF7C"
        )
    } else {
        // Invalid route for testing
        (
            "0x1234567890123456789012345678901234567890",
            "ibc/INVALID123456789012345678901234567890123456789012345678901234567890",
            "INVALID",
            "0x0000000000000000000000000000000000000000"
        )
    };

    serde_json::json!({
        "operations": [
            {
                "type": "evm_swap",
                "input_token": source_denom,
                "amount_in": "1000000",
                "amount_out": "999000",
                "denom_in": source_denom,
                "denom_out": "0xBf45A5029d081333407Cc52a84BE5ed40e181C46"
            },
            {
                "type": "eureka_transfer",
                "from_chain_id": "1",
                "to_chain_id": "ledger-mainnet-1",
                "denom_in": "0xBf45A5029d081333407Cc52a84BE5ed40e181C46",
                "denom_out": "ibc/EB19395F41C98C5F53420B7F8A96A02D075F86E5E8B90B88EE0D6C63A32F9040",
                "bridge_id": bridge_id,
                "entry_contract_address": entry_contract,
                "smart_relay": true
            },
            {
                "type": "swap",
                "chain_id": "ledger-mainnet-1",
                "denom_in": "ibc/EB19395F41C98C5F53420B7F8A96A02D075F86E5E8B90B88EE0D6C63A32F9040",
                "denom_out": "lbtc"
            },
            {
                "type": "transfer",
                "from_chain_id": "ledger-mainnet-1",
                "to_chain_id": "cosmoshub-4",
                "denom_in": "lbtc",
                "denom_out": dest_denom,
                "bridge_id": "IBC"
            }
        ],
        "estimated_route_duration_seconds": 120,
        "estimated_fees": [
            {
                "fee_type": "smart_relay",
                "bridge_id": bridge_id,
                "amount": fees_lbtc_wei.to_string(),
                "chain_id": "1"
            }
        ]
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_fee_data() {
        let mock_response = generate_mock_skip_response(957, true);
        let fees = extract_fee_data(&mock_response).unwrap();
        assert_eq!(fees, 957);
    }

    #[test]
    fn test_extract_route_data() {
        let mock_response = generate_mock_skip_response(957, true);
        let route_string = extract_route_data(&mock_response).unwrap();
        assert!(route_string.contains("source_chain:1"));
        assert!(route_string.contains("bridge_type:eureka_transfer"));
        assert!(route_string.contains("bridge_id:EUREKA"));
    }

    #[test]
    fn test_get_witnesses_valid() {
        let mock_response = generate_mock_skip_response(957, true);
        let args = serde_json::json!({
            "skip_response": mock_response,
            "destination": "cosmos1zxj6y5h3r8k9v7n2m4l1q8w5e3t6y9u0i7o4p2s5d8f6g3h1j4k7l9n2"
        });
        
        let witnesses = get_witnesses(args).unwrap();
        assert_eq!(witnesses.len(), 3);
        
        // Check fee witness
        let fee_bytes = witnesses[0].as_data().unwrap();
        let fees = u64::from_le_bytes(<[u8; 8]>::try_from(fee_bytes).unwrap());
        assert_eq!(fees, 957);
    }
}
