mod artifacts;
mod steps;

use std::env;

use clap::Parser;
use common::NeutronStrategyConfig;
use valence_domain_clients::clients::{coprocessor::CoprocessorClient, neutron::NeutronClient};

use crate::artifacts::CoprocessorOutputs;

use clap::ValueEnum;

pub(crate) const PROVISIONER: &str = "PROVISIONER";

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// which step to run. Defaults to `all`.
    #[arg(long, value_enum, default_value_t = Step::All)]
    step: Step,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
enum Step {
    /// run all steps in sequence
    All,
    /// runs the on-chain contract instantiation step as described
    /// in `provisioner/src/steps/instantiate_contracts.rs`.
    /// prerequisite for `deploy_coprocessor` and `setup_authorizations`.
    InstantiateContracts,
    /// builds and deploys the zk app to the co-processor.
    /// depends on `instantiate_contracts` step.
    /// prerequisite for `setup_authorizations` step.
    DeployCoprocessor,
    /// sets up the on-chain authorizations and associates
    /// the on-chain deployment with the co-processor deployment.
    /// depends on `instantiate_contracts`, `deploy_coprocessor`
    /// steps. prerequisite for running the coordinator.
    Authorize,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    env_logger::init();

    let cli = Cli::parse();

    let mnemonic = env::var("MNEMONIC")?;
    let neutron_inputs = steps::read_setup_inputs("neutron_inputs.toml")?;

    let cp_client = CoprocessorClient::default();
    let neutron_client = NeutronClient::new(
        &neutron_inputs.grpc_url,
        &neutron_inputs.grpc_port,
        &mnemonic,
        &neutron_inputs.chain_id,
    )
    .await?;

    // first step is to instantiate the on-chain contracts
    match cli.step {
        Step::All | Step::InstantiateContracts => {
            let instantiation_outputs =
                steps::instantiate_contracts(&neutron_client, neutron_inputs.code_ids).await?;
            artifacts::write_instantiation_artifacts(instantiation_outputs)?;
        }
        _ => {}
    };

    // second step is to build and deploy the coprocessor app.
    // this depends on the first step and can be seen as the second step
    // in a tcp handshake as the cw20 address from step 1 is embedded
    // into our circuits before they get compiled
    // if run_all || cli.deploy_coprocessor {
    match cli.step {
        Step::All | Step::DeployCoprocessor => {
            let instantiation_outputs = artifacts::read_instantiation_artifacts()?;
            let coprocessor_app_id =
                steps::deploy_coprocessor_app(&cp_client, &instantiation_outputs.cw20).await?;
            artifacts::write_coprocessor_artifacts(CoprocessorOutputs { coprocessor_app_id })?;
        }
        _ => {}
    };

    // finally, we set up the on-chain authorizations. this can be seen
    // as the final step in a tcp handshake where our on-chain contracts
    // are made aware of the coprocessor deployment.
    match cli.step {
        Step::All | Step::Authorize => {
            let instantiation_outputs = artifacts::read_instantiation_artifacts()?;
            let coprocessor_outputs = artifacts::read_coprocessor_artifacts()?;
            let neutron_strategy_config = NeutronStrategyConfig {
                grpc_url: neutron_inputs.grpc_url.clone(),
                grpc_port: neutron_inputs.grpc_port.clone(),
                chain_id: neutron_inputs.chain_id.clone(),
                authorizations: instantiation_outputs.authorizations,
                processor: instantiation_outputs.processor,
                cw20: instantiation_outputs.cw20,
                coprocessor_app_id: coprocessor_outputs.coprocessor_app_id,
            };
            steps::setup_authorizations(&neutron_client, &cp_client, &neutron_strategy_config)
                .await?;

            steps::write_setup_artifacts(neutron_strategy_config)?;
        }
        _ => {}
    };

    Ok(())
}
