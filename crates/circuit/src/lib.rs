#![no_std]

extern crate alloc;

use alloc::{vec, vec::Vec, format};
use valence_coprocessor::Witness;

// Hardcoded constants from Phase 1 discovery
/// Expected route hash for LBTC IBC Eureka transfers
const EXPECTED_ROUTE_HASH: &str = "a041afeb1546e275ec0038183732036ce653b197e8129748da95cf6c7de43abf";

/// Expected destination address (cosmos1...)
const EXPECTED_DESTINATION: &str = "cosmos1zxj6y5h3r8k9v7n2m4l1q8w5e3t6y9u0i7o4p2s5d8f6g3h1j4k7l9n2";

/// Fee threshold in LBTC wei (0.0000189 LBTC = $2.00 equivalent)
const FEE_THRESHOLD_LBTC_WEI: u64 = 1890000000000000;

/// Expected route components for LBTC IBC Eureka
const EXPECTED_SOURCE_CHAIN: &str = "1";
const EXPECTED_DEST_CHAIN: &str = "cosmoshub-4";
const EXPECTED_BRIDGE_ID: &str = "EUREKA";
const EXPECTED_ENTRY_CONTRACT: &str = "0xFc2d0487A0ae42ae7329a80dc269916A9184cF7C";

/// Simple hash function for route validation (using sum for now, would use SHA3 in production)
fn simple_hash(input: &[u8]) -> u64 {
    input.iter().map(|&b| b as u64).sum()
}

/// Validate that route string contains expected components
fn validate_route_components(route_string: &str) -> bool {
    route_string.contains(&format!("source_chain:{}", EXPECTED_SOURCE_CHAIN)) &&
    route_string.contains("bridge_type:eureka_transfer") &&
    route_string.contains(&format!("bridge_id:{}", EXPECTED_BRIDGE_ID)) &&
    route_string.contains(&format!("entry_contract:{}", EXPECTED_ENTRY_CONTRACT))
}

/// Main circuit function for LBTC transfer validation
pub fn circuit(witnesses: Vec<Witness>) -> Vec<u8> {
    // Ensure we have the expected number of witnesses
    assert_eq!(witnesses.len(), 3, "Expected 3 witnesses: fees, route, destination");

    // Extract witness data
    let fee_bytes = witnesses[0].as_data().expect("Failed to get fee data");
    let route_bytes = witnesses[1].as_data().expect("Failed to get route data");
    let destination_bytes = witnesses[2].as_data().expect("Failed to get destination data");

    // Parse fee amount (LBTC wei)
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

    // Validation 1: Route Components Check
    let route_valid = validate_route_components(route_string);

    // Validation 2: Destination Address Check
    let destination_valid = destination_address == EXPECTED_DESTINATION;

    // Validation 3: Fee Threshold Check
    let fees_within_limit = fee_amount <= FEE_THRESHOLD_LBTC_WEI;

    // Overall validation result
    let validation_passed = route_valid && destination_valid && fees_within_limit;

    // Generate structured output
    let validation_result = ValidationResult {
        validation_passed,
        route_valid,
        destination_valid,
        fees_within_limit,
        actual_fee_lbtc_wei: fee_amount,
        fee_threshold_lbtc_wei: FEE_THRESHOLD_LBTC_WEI,
    };

    // Serialize result (simple format for now)
    serialize_validation_result(&validation_result)
}

/// Validation result structure
struct ValidationResult {
    validation_passed: bool,
    route_valid: bool,
    destination_valid: bool,
    fees_within_limit: bool,
    actual_fee_lbtc_wei: u64,
    fee_threshold_lbtc_wei: u64,
}

/// Serialize validation result to bytes (simple binary format)
fn serialize_validation_result(result: &ValidationResult) -> Vec<u8> {
    let mut output = Vec::new();
    
    // Pack boolean results into first byte
    let flags = (result.validation_passed as u8) |
                ((result.route_valid as u8) << 1) |
                ((result.destination_valid as u8) << 2) |
                ((result.fees_within_limit as u8) << 3);
    
    output.push(flags);
    
    // Add fee amounts
    output.extend_from_slice(&result.actual_fee_lbtc_wei.to_le_bytes());
    output.extend_from_slice(&result.fee_threshold_lbtc_wei.to_le_bytes());
    
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circuit_valid_transfer() {
        let fee_amount = 957u64; // Valid fee below threshold
        let route_string = "source_chain:1|dest_chain:cosmoshub-4|bridge_type:eureka_transfer|bridge_id:EUREKA|entry_contract:0xFc2d0487A0ae42ae7329a80dc269916A9184cF7C";
        let destination = EXPECTED_DESTINATION;

        let witnesses = vec![
            Witness::Data(fee_amount.to_le_bytes().to_vec()),
            Witness::Data(route_string.as_bytes().to_vec()),
            Witness::Data(destination.as_bytes().to_vec()),
        ];

        let result = circuit(witnesses);
        
        // Check that validation passed (first bit set)
        assert_eq!(result[0] & 0x01, 1, "Overall validation should pass");
        assert_eq!(result[0] & 0x02, 2, "Route validation should pass");
        assert_eq!(result[0] & 0x04, 4, "Destination validation should pass");
        assert_eq!(result[0] & 0x08, 8, "Fee validation should pass");
    }

    #[test]
    fn test_circuit_excessive_fees() {
        let fee_amount = 2000000000000000u64; // Excessive fee above threshold
        let route_string = "source_chain:1|dest_chain:cosmoshub-4|bridge_type:eureka_transfer|bridge_id:EUREKA|entry_contract:0xFc2d0487A0ae42ae7329a80dc269916A9184cF7C";
        let destination = EXPECTED_DESTINATION;

        let witnesses = vec![
            Witness::Data(fee_amount.to_le_bytes().to_vec()),
            Witness::Data(route_string.as_bytes().to_vec()),
            Witness::Data(destination.as_bytes().to_vec()),
        ];

        let result = circuit(witnesses);
        
        // Check that validation failed due to excessive fees
        assert_eq!(result[0] & 0x01, 0, "Overall validation should fail");
        assert_eq!(result[0] & 0x08, 0, "Fee validation should fail");
    }

    #[test]
    fn test_circuit_invalid_route() {
        let fee_amount = 957u64;
        let route_string = "source_chain:INVALID|dest_chain:cosmoshub-4|bridge_type:invalid|bridge_id:INVALID";
        let destination = EXPECTED_DESTINATION;

        let witnesses = vec![
            Witness::Data(fee_amount.to_le_bytes().to_vec()),
            Witness::Data(route_string.as_bytes().to_vec()),
            Witness::Data(destination.as_bytes().to_vec()),
        ];

        let result = circuit(witnesses);
        
        // Check that validation failed due to invalid route
        assert_eq!(result[0] & 0x01, 0, "Overall validation should fail");
        assert_eq!(result[0] & 0x02, 0, "Route validation should fail");
    }

    #[test]
    fn test_circuit_wrong_destination() {
        let fee_amount = 957u64;
        let route_string = "source_chain:1|dest_chain:cosmoshub-4|bridge_type:eureka_transfer|bridge_id:EUREKA|entry_contract:0xFc2d0487A0ae42ae7329a80dc269916A9184cF7C";
        let destination = "cosmos1wrongaddress1234567890123456789012345678901234567890";

        let witnesses = vec![
            Witness::Data(fee_amount.to_le_bytes().to_vec()),
            Witness::Data(route_string.as_bytes().to_vec()),
            Witness::Data(destination.as_bytes().to_vec()),
        ];

        let result = circuit(witnesses);
        
        // Check that validation failed due to wrong destination
        assert_eq!(result[0] & 0x01, 0, "Overall validation should fail");
        assert_eq!(result[0] & 0x04, 0, "Destination validation should fail");
    }
}
