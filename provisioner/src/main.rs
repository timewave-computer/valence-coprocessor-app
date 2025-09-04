mod artifacts;
mod consts;
mod steps;

use std::env;

use clap::Parser;
use common::NeutronStrategyConfig;
use valence_domain_clients::clients::{coprocessor::CoprocessorClient, neutron::NeutronClient};

use crate::artifacts::CoprocessorOutputs;

pub(crate) const PROVISIONER: &str = "PROVISIONER";

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// runs the on-chain contract instantiation step as described
    /// in `provisioner/src/steps/instantiate_contracts.rs`.
    ///
    /// prerequisite for the following steps:
    ///
    /// - `deploy_coprocessor`
    /// - `setup_authorizations`
    #[arg(long)]
    instantiate_contracts: bool,

    /// builds and deploys the zk app to the co-processor.
    ///
    /// depends on the following steps:
    ///
    /// - `instantiate_contracts`
    ///
    /// prerequisite for the following steps:
    ///
    /// - `setup_authorizations`
    #[arg(long)]
    deploy_coprocessor: bool,

    /// sets up the on-chain authorizations and associates
    /// the on-chain deployment with the co-processor deployment.
    ///
    /// depends on the following steps:
    ///
    /// - `instantiate_contracts`
    /// - `deploy_coprocessor`
    ///
    /// prerequisite for running the coordinator.
    #[arg(long)]
    setup_authorizations: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    env_logger::init();

    let cli = Cli::parse();
    let current_dir = env::current_dir()?;

    let mnemonic = env::var("MNEMONIC")?;
    let neutron_inputs = steps::read_setup_inputs(current_dir.clone())?;

    let cp_client = CoprocessorClient::default();
    let neutron_client = NeutronClient::new(
        &neutron_inputs.grpc_url,
        &neutron_inputs.grpc_port,
        &mnemonic,
        &neutron_inputs.chain_id,
    )
    .await?;

    // if no flags were specified we do e2e provisioning
    let run_all =
        !cli.instantiate_contracts && !cli.deploy_coprocessor && !cli.setup_authorizations;

    // first step is to instantiate the on-chain contracts
    if run_all || cli.instantiate_contracts {
        let instantiation_outputs =
            steps::instantiate_contracts(&neutron_client, neutron_inputs.code_ids).await?;
        artifacts::write_instantiation_artifacts(instantiation_outputs)?;
    }

    // second step is to build and deploy the coprocessor app.
    // this depends on the first step and can be seen as the second step
    // in a tcp handshake as the cw20 address from step 1 is embedded
    // into our circuits before they get compiled
    if run_all || cli.deploy_coprocessor {
        let instantiation_outputs = artifacts::read_instantiation_artifacts()?;
        let coprocessor_app_id = steps::deploy_coprocessor_app(
            &cp_client,
            current_dir.clone(),
            &instantiation_outputs.cw20,
        )
        .await?;
        artifacts::write_coprocessor_artifacts(CoprocessorOutputs { coprocessor_app_id })?;
    }

    // finally, we set up the on-chain authorizations. this can be seen
    // as the final step in a tcp handshake where our on-chain contracts
    // are made aware of the coprocessor deployment.
    if run_all || cli.setup_authorizations {
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
        steps::setup_authorizations(&neutron_client, &cp_client, &neutron_strategy_config).await?;

        steps::write_setup_artifacts(current_dir, neutron_strategy_config)?;
    }

    Ok(())
}
