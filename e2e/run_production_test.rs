//! Simple runner for the production SP1 proving flow test
//!
//! This can be run independently to test the complete pipeline:
//! ```bash
//! cd e2e && cargo run --bin run_production_test
//! ```

use anyhow::Result;
use std::env;

mod test_production_sp1_proving_flow;
use test_production_sp1_proving_flow::{ProductionFlowConfig, ProductionSP1ProvingTest};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("Valence Coprocessor Production SP1 Proving Flow Test");
    println!("=====================================================\n");

    // Create configuration (can be customized via environment variables)
    let mut config = ProductionFlowConfig::default();

    // Allow overriding via environment variables
    if let Ok(coprocessor_url) = env::var("COPROCESSOR_URL") {
        config.coprocessor_url = coprocessor_url;
    }

    if let Ok(controller_id) = env::var("CONTROLLER_ID") {
        config.controller_id = controller_id;
    }

    if let Ok(destination) = env::var("EXPECTED_DESTINATION") {
        config.expected_destination = destination;
    }

    println!("Test Configuration:");
    println!("   Coprocessor URL: {}", config.coprocessor_url);
    println!("   Controller ID: {}", config.controller_id);
    println!("   Expected Destination: {}", config.expected_destination);
    println!("   Fee Threshold: {} wei", config.fee_threshold);
    println!();

    // Clone needed values before moving config
    let controller_id_for_error = config.controller_id.clone();
    let coprocessor_url_for_error = config.coprocessor_url.clone();

    // Create and run the test
    let test = ProductionSP1ProvingTest::new(config);

    match test.run_complete_flow().await {
        Ok(results) => {
            println!("\nTest Completed Successfully!");
            println!("Results Summary:");
            println!(
                "   Skip API Integration: {}",
                if results.skip_api_response.is_some() {
                    "PASS"
                } else {
                    "FAIL"
                }
            );
            println!(
                "   Controller Deployment: {}",
                if results.controller_deployed {
                    "PASS"
                } else {
                    "FAIL"
                }
            );
            println!(
                "   Witnesses Generated: {}",
                if results.witnesses_generated {
                    "PASS"
                } else {
                    "FAIL"
                }
            );
            println!(
                "   SP1 Proof Generated: {}",
                if results.sp1_proof_generated {
                    "PASS"
                } else {
                    "FAIL"
                }
            );
            println!(
                "   Validation Passed: {}",
                if results.validation_passed {
                    "PASS"
                } else {
                    "FAIL"
                }
            );
            println!(
                "   ABI Message Generated: {}",
                if results.abi_encoded_message.is_some() {
                    "PASS"
                } else {
                    "FAIL"
                }
            );
            println!("   Total Duration: {:?}", results.total_duration);

            if !results.errors.is_empty() {
                println!("\nErrors Encountered:");
                for (i, error) in results.errors.iter().enumerate() {
                    println!("   {}. {}", i + 1, error);
                }
                println!("\nTest completed with {} error(s)", results.errors.len());
                std::process::exit(1);
            } else {
                println!("\nAll steps completed successfully!");
                println!("The valence-coprocessor-app is production-ready!");
            }
        }
        Err(e) => {
            println!("\nTest Failed: {}", e);
            println!("Check that:");
            println!(
                "   1. Coprocessor service is running on {}",
                coprocessor_url_for_error
            );
            println!("   2. Controller {} is deployed", controller_id_for_error);
            println!("   3. Skip API is accessible");
            println!("   4. Internet connection is available");
            std::process::exit(1);
        }
    }

    Ok(())
}
