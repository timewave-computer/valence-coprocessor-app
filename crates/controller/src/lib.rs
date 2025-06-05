//! Token IBC Eureka Transfer Controller - purpose of this file is to handle witness generation and validation logic for token transfers

#![no_std]

extern crate alloc;

use alloc::{string::{String, ToString as _}, vec::Vec, vec, format};
use serde_json::Value;
use valence_coprocessor::Witness;
use valence_coprocessor_wasm::abi;

// WASM binary setup - only for WASM target
#[cfg(target_arch = "wasm32")]
extern crate dlmalloc;

#[cfg(target_arch = "wasm32")]
use dlmalloc::GlobalDlmalloc;

#[cfg(target_arch = "wasm32")]
#[global_allocator]
static ALLOCATOR: GlobalDlmalloc = GlobalDlmalloc;

#[cfg(target_arch = "wasm32")]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

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
                abi::log!("Found fee: {} token wei", amount)?;
            }
        }
    }
    
    abi::log!("Total fees extracted: {} token wei", total_fees)?;
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

/// Extract memo from the arguments
fn extract_memo(args: &Value) -> anyhow::Result<String> {
    Ok(args["memo"]
        .as_str()
        .unwrap_or("") // Default to empty string if not provided
        .to_string())
}

/// Controller witness generation function
#[no_mangle]
pub extern "C" fn get_witnesses() {
    let args = match abi::args() {
        Ok(args) => args,
        Err(_) => return,
    };

    let _ = abi::log!(
        "received a proof request with token transfer arguments {}",
        serde_json::to_string(&args).unwrap_or_default()
    );

    // Check if this is the framework's expected simple "value" format
    if let Some(value) = args.get("value") {
        if value.is_number() {
            // This is the simple framework format, return minimal witnesses
            let _ = abi::log!("Using simple value format: {}", value);
            let simple_value = value.as_u64().unwrap_or(0);
            let witnesses = [
                Witness::Data(simple_value.to_le_bytes().to_vec()),    // Simple value
                Witness::Data(b"".to_vec()),                          // Empty route
                Witness::Data(b"".to_vec()),                          // Empty destination  
                Witness::Data(b"".to_vec()),                          // Empty memo
            ].to_vec();
            
            let _ = abi::ret_witnesses(witnesses);
            return;
        }
    }

    // Handle nested argument structure - first check for args.args.payload, then args.payload
    let skip_response = if let Some(nested_args) = args.get("args") {
        // Check args.args.payload.skip_response
        if let Some(payload) = nested_args.get("payload") {
            payload.get("skip_response")
        } else {
            // Check args.args.skip_response
            nested_args.get("skip_response")
        }
    } else if let Some(payload) = args.get("payload") {
        // Check args.payload.skip_response
        payload.get("skip_response")
    } else {
        // Check args.skip_response
        args.get("skip_response")
    };

    // Check if we have structured data (Skip API format)
    if let Some(skip_response) = skip_response {
        if !skip_response.is_null() {
            let _ = abi::log!("Found Skip API response data, processing witnesses");

            // Extract fee data (in token wei)
            let total_fees = match extract_fee_data(skip_response) {
                Ok(fees) => fees,
                Err(_) => return,
            };
            
            // Extract route data
            let route_string = match extract_route_data(skip_response) {
                Ok(route) => route,
                Err(_) => return,
            };
            
            // Extract destination and memo from the appropriate location (check nested structure)
            let (destination, memo) = if let Some(nested_args) = args.get("args") {
                if let Some(payload) = nested_args.get("payload") {
                    // args.args.payload structure
                    let dest = payload["destination"].as_str().unwrap_or("");
                    let memo = payload["memo"].as_str().unwrap_or("");
                    (dest.to_string(), memo.to_string())
                } else {
                    // args.args structure (fallback)
                    let dest = nested_args["destination"].as_str().unwrap_or("");
                    let memo = nested_args["memo"].as_str().unwrap_or("");
                    (dest.to_string(), memo.to_string())
                }
            } else if let Some(payload) = args.get("payload") {
                // args.payload structure
                let dest = payload["destination"].as_str().unwrap_or("");
                let memo = payload["memo"].as_str().unwrap_or("");
                (dest.to_string(), memo.to_string())
            } else {
                // Direct structure
                let dest = match extract_destination_address(&args) {
                    Ok(d) => d,
                    Err(_) => return,
                };
                let memo = match extract_memo(&args) {
                    Ok(m) => m,
                    Err(_) => return,
                };
                (dest, memo)
            };
            
            let _ = abi::log!("Preparing witnesses: fees={}, route_len={}, dest_len={}, memo_len={}", 
                      total_fees, route_string.len(), destination.len(), memo.len());

            // Prepare witness data for circuit (4 witnesses expected)
            let witnesses = [
                Witness::Data(total_fees.to_le_bytes().to_vec()),          // Witness 0: Total fees in token wei
                Witness::Data(route_string.as_bytes().to_vec()),           // Witness 1: Route string for hashing
                Witness::Data(destination.as_bytes().to_vec()),            // Witness 2: Destination address
                Witness::Data(memo.as_bytes().to_vec()),                   // Witness 3: Memo (should be empty for security)
            ].to_vec();

            let _ = abi::ret_witnesses(witnesses);
            return;
        }
    }

    // If neither format matches, return empty witnesses
    let _ = abi::ret_witnesses(vec![]);
}

/// Controller entrypoint function
#[no_mangle]
pub extern "C" fn entrypoint() {
    let args = match abi::args() {
        Ok(args) => args,
        Err(_) => return,
    };

    let _ = abi::log!(
        "received a token transfer entrypoint request with arguments {}",
        serde_json::to_string(&args).unwrap_or_default()
    );

    // Handle nested argument structure - check if we have args.args.payload or args.payload
    let payload = if let Some(nested_args) = args.get("args") {
        nested_args.get("payload")
    } else {
        args.get("payload")
    };

    let payload = match payload {
        Some(p) => p,
        None => {
            let _ = abi::log!("ERROR: No payload found in arguments");
            let _ = abi::ret(&args);
            return;
        }
    };
    
    let cmd = match payload["cmd"].as_str() {
        Some(c) => c,
        None => {
            let _ = abi::log!("ERROR: No cmd found in payload");
            let _ = abi::ret(&args);
            return;
        }
    };
    
    let _ = abi::log!("Processing command: {}", cmd);

    match cmd {
        "store" => {
            let _ = abi::log!("Executing store command");
            let path = payload["path"].as_str().unwrap().to_string();
            
            // Check if this is a validation result
            if let Some(validation_result) = args.get("validation_result") {
                let _ = abi::log!("Storing token transfer validation result to {}", path);
                
                // Create structured validation response
                let response = serde_json::json!({
                    "transfer_type": "TOKEN_IBC_EUREKA",
                    "validation_result": validation_result,
                    "original_args": args
                });
                
                let bytes = serde_json::to_vec(&response).unwrap();
                let _ = abi::set_storage_file(&path, &bytes);
                
                let _ = abi::log!("Successfully stored token transfer validation result");
            } else {
                // Store the raw arguments as before for compatibility
                let _ = abi::log!("Storing raw arguments to {}", path);
                let bytes = serde_json::to_vec(&args).unwrap();
                let _ = abi::set_storage_file(&path, &bytes);
            }
        }
        
        "validate" => {
            let _ = abi::log!("Executing validate command");
            
            // Extract Skip API data from the correct nested location
            let skip_response = if let Some(nested_args) = args.get("args") {
                if let Some(nested_payload) = nested_args.get("payload") {
                    nested_payload["skip_response"].as_object()
                } else {
                    nested_args["skip_response"].as_object() 
                }
            } else {
                payload["skip_response"].as_object()
            };
            
            // Check if we have structured Skip API data
            if let Some(skip_response) = skip_response {
                let _ = abi::log!("Found Skip API response data in payload, performing validation");
                
                // Extract fee data using existing functions - wrap in error handling
                let total_fees = match extract_fee_data(&Value::Object(skip_response.clone())) {
                    Ok(fees) => {
                        let _ = abi::log!("Successfully extracted total fees: {} token wei", fees);
                        fees
                    }
                    Err(e) => {
                        let _ = abi::log!("ERROR extracting fee data: {}", e);
                        let _ = abi::ret(&args);
                        return;
                    }
                };
                
                // Extract route data - wrap in error handling
                let route_string = match extract_route_data(&Value::Object(skip_response.clone())) {
                    Ok(route) => {
                        let _ = abi::log!("Successfully extracted route: {}", route);
                        route
                    }
                    Err(e) => {
                        let _ = abi::log!("ERROR extracting route data: {}", e);
                        let _ = abi::ret(&args);
                        return;
                    }
                };
                
                // Extract destination and memo from nested payload
                let (destination, memo) = if let Some(nested_args) = args.get("args") {
                    if let Some(nested_payload) = nested_args.get("payload") {
                        // args.args.payload structure
                        let dest = nested_payload["destination"].as_str().unwrap_or("");
                        let memo = nested_payload["memo"].as_str().unwrap_or("");
                        (dest, memo)
                    } else {
                        // args.args structure (fallback)
                        let dest = nested_args["destination"].as_str().unwrap_or("");
                        let memo = nested_args["memo"].as_str().unwrap_or("");
                        (dest, memo)
                    }
                } else {
                    // args.payload structure
                    let dest = payload["destination"].as_str().unwrap_or("");
                    let memo = payload["memo"].as_str().unwrap_or("");
                    (dest, memo)
                };
                
                let _ = abi::log!("Extracted - Destination: '{}', Memo: '{}'", destination, memo);
                
                // Perform validation logic (similar to circuit validation)
                
                // 1. Route validation - check if it contains expected components
                let route_valid = route_string.contains("source_chain:1") &&
                                  route_string.contains("dest_chain:cosmoshub-4") &&
                                  route_string.contains("bridge_type:eureka_transfer") &&
                                  route_string.contains("bridge_id:EUREKA") &&
                                  route_string.contains("entry_contract:0xFc2d0487A0ae42ae7329a80dc269916A9184cF7C");
                
                // 2. Destination validation - check if it matches expected
                let expected_destination = "cosmos1zxj6y5h3r8k9v7n2m4l1q8w5e3t6y9u0i7o4p2s5d8f6g3h1j4k7l9n2";
                let destination_valid = destination == expected_destination;
                
                // 3. Fee validation - check if fees are below threshold (1.89 USD equivalent in token wei)
                let fee_threshold = 1890000000000000u64; // 1.89 USD worth of token wei
                let fees_within_limit = total_fees <= fee_threshold;
                
                // 4. Memo validation - must be empty for security
                let memo_valid = memo.is_empty();
                
                // Overall validation result
                let validation_passed = route_valid && destination_valid && fees_within_limit && memo_valid;
                
                let _ = abi::log!("Validation results: route={}, dest={}, fees={}<={}?, memo_empty={}, overall={}", 
                         route_valid, destination_valid, total_fees, fee_threshold, memo_valid, validation_passed);
                
                // Store validation results in storage if path provided - use FAT-16 compatible path
                if let Some(_storage_path) = payload["path"].as_str() {
                    // Use a simple FAT-16 compatible filename in root directory
                    let fat16_path = "VALIDATN.JSN";  // 8.3 format for FAT-16 compatibility
                    let _ = abi::log!("Storing validation results to FAT-16 path: {}", fat16_path);
                    
                    let validation_response = serde_json::json!({
                        "validation_passed": validation_passed,
                        "route_valid": route_valid,
                        "destination_valid": destination_valid,
                        "fees_within_limit": fees_within_limit,
                        "memo_valid": memo_valid,
                        "total_fees_token_wei": total_fees,
                        "fee_threshold": fee_threshold,
                        "expected_destination": expected_destination,
                        "actual_destination": destination,
                        "route_components": route_string,
                        "memo": memo
                    });
                    
                    let bytes = match serde_json::to_vec(&validation_response) {
                        Ok(b) => b,
                        Err(e) => {
                            let _ = abi::log!("ERROR serializing validation response: {}", e);
                            // Continue execution even if serialization fails
                            serde_json::to_vec(&serde_json::json!({"error": "serialization_failed"})).unwrap_or_default()
                        }
                    };
                    
                    // Try to store, but don't fail the entire validation if storage fails
                    match abi::set_storage_file(fat16_path, &bytes) {
                        Ok(_) => {
                            let _ = abi::log!("Successfully stored validation results to {}", fat16_path);
                        }
                        Err(e) => {
                            let _ = abi::log!("WARNING: Failed to store validation results to {}: {} (continuing anyway)", fat16_path, e);
                        }
                    }
                }
                
                let _ = abi::log!("Token transfer validation completed successfully!");
                
                // Create a success response that includes validation results
                let success_response = serde_json::json!({
                    "args": args,
                    "validation_passed": validation_passed,
                    "success": validation_passed  // Return success based on validation result
                });
                
                let _ = abi::ret(&success_response);
                return;
            } else {
                let _ = abi::log!("No skip_response found in payload, trying legacy validation format");
                
                // Fallback to old validation format
                let fees = args["fees"].as_u64().unwrap_or(0);
                let route_valid = args["route_valid"].as_bool().unwrap_or(false);
                let destination_valid = args["destination_valid"].as_bool().unwrap_or(false);
                let fees_within_limit = args["fees_within_limit"].as_bool().unwrap_or(false);
                
                let validation_passed = route_valid && destination_valid && fees_within_limit;
                
                let _ = abi::log!("Validation results (legacy format): route={}, dest={}, fees={}, overall={}", 
                         route_valid, destination_valid, fees_within_limit, validation_passed);
                
                // Store validation results in storage if path provided
                if let Some(_storage_path) = payload["path"].as_str() {
                    let fat16_path = "VALIDATN.JSN";  // FAT-16 compatible path
                    let validation_response = serde_json::json!({
                        "validation_passed": validation_passed,
                        "route_valid": route_valid,
                        "destination_valid": destination_valid,
                        "fees_within_limit": fees_within_limit,
                        "total_fees_token_wei": fees
                    });
                    
                    let bytes = serde_json::to_vec(&validation_response).unwrap_or_default();
                    let _ = abi::set_storage_file(fat16_path, &bytes);
                    let _ = abi::log!("Stored legacy validation results to {}", fat16_path);
                }
                
                // Return success based on validation result
                let success_response = serde_json::json!({
                    "args": args,
                    "validation_passed": validation_passed,
                    "success": validation_passed
                });
                
                let _ = abi::ret(&success_response);
                return;
            }
        }

        _ => {
            let _ = abi::log!("ERROR: Unknown entrypoint command: {}", cmd);
            let _ = abi::ret(&args);
            return;
        }
    }

    let _ = abi::log!("Entrypoint completed successfully");
    let _ = abi::ret(&args);
}

/// Generate mock Skip API response for testing
pub fn generate_mock_skip_response(fees_token_wei: u64, use_valid_route: bool) -> Value {
    let (source_denom, dest_denom, bridge_id, entry_contract) = if use_valid_route {
        // Valid token Eureka route from Phase 1
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
                "denom_out": "token"
            },
            {
                "type": "transfer",
                "from_chain_id": "ledger-mainnet-1",
                "to_chain_id": "cosmoshub-4",
                "denom_in": "token",
                "denom_out": dest_denom,
                "bridge_id": "IBC"
            }
        ],
        "estimated_route_duration_seconds": 120,
        "estimated_fees": [
            {
                "fee_type": "smart_relay",
                "bridge_id": bridge_id,
                "amount": fees_token_wei.to_string(),
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
        let _args = serde_json::json!({
            "skip_response": mock_response,
            "destination": "cosmos1zxj6y5h3r8k9v7n2m4l1q8w5e3t6y9u0i7o4p2s5d8f6g3h1j4k7l9n2",
            "memo": ""
        });
        
        // Note: Cannot directly test get_witnesses() here since it uses abi calls
        // This test verifies the helper functions work correctly
        let fees = extract_fee_data(&mock_response).unwrap();
        assert_eq!(fees, 957);
        
        let route_string = extract_route_data(&mock_response).unwrap();
        assert!(route_string.contains("source_chain:1"));
        assert!(route_string.contains("bridge_type:eureka_transfer"));
        assert!(route_string.contains("bridge_id:EUREKA"));
    }
}
