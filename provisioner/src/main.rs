mod consts;
mod steps;

use std::env;

use common::NeutronStrategyConfig;
use valence_domain_clients::clients::{coprocessor::CoprocessorClient, neutron::NeutronClient};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    let mnemonic = env::var("MNEMONIC")?;
    let current_dir = env::current_dir()?;

    let neutron_inputs = steps::read_setup_inputs(current_dir.clone())?;

    let cp_client = CoprocessorClient::default();
    let neutron_client = NeutronClient::new(
        &neutron_inputs.grpc_url,
        &neutron_inputs.grpc_port,
        &mnemonic,
        &neutron_inputs.chain_id,
    )
    .await?;

    let instantiation_outputs =
        steps::instantiate_contracts(&neutron_client, neutron_inputs.code_ids).await?;

    let coprocessor_app_id =
        steps::deploy_coprocessor_app(&cp_client, current_dir.clone(), &instantiation_outputs.cw20)
            .await?;

    let neutron_strategy_config = NeutronStrategyConfig {
        grpc_url: neutron_inputs.grpc_url,
        grpc_port: neutron_inputs.grpc_port,
        chain_id: neutron_inputs.chain_id,
        authorizations: instantiation_outputs.authorizations,
        processor: instantiation_outputs.processor,
        cw20: instantiation_outputs.cw20,
        coprocessor_app_id,
    };

    println!("neutron strategy config: {neutron_strategy_config:?}");

    steps::setup_authorizations(&neutron_client, &cp_client, &neutron_strategy_config).await?;

    steps::write_setup_artifacts(current_dir, neutron_strategy_config)?;

    Ok(())
}
