pub mod engine;
pub mod strategy;

use std::fs;

use common::NeutronStrategyConfig;
use common::OUTPUTS_DIR;
use dotenv::dotenv;
use log::{info, warn};
use strategy::Strategy;
use valence_coordinator_sdk::coordinator::ValenceCoordinator;

const RUNNER: &str = "runner";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // load environment variables
    dotenv().ok();

    // initialize the logger
    valence_coordinator_sdk::telemetry::setup_logging(None)?;

    info!(target: RUNNER, "starting the coordinator runner");

    let neutron_cfg_path = format!("{OUTPUTS_DIR}/neutron_strategy_config.toml");

    info!(target: RUNNER, "Using ntrn config: {neutron_cfg_path}");

    let parameters = fs::read_to_string(neutron_cfg_path)?;

    let neutron_cfg: NeutronStrategyConfig = toml::from_str(&parameters)?;

    let strategy = Strategy::new(neutron_cfg).await?;

    info!(target: RUNNER, "strategy initialized");
    info!(target: RUNNER, "starting the coordinator");

    let coordinator_join_handle = strategy.start();

    // join here will wait for the coordinator thread to finish which should never happen
    // in practice since it runs an infinite stayalive loop
    match coordinator_join_handle.join() {
        Ok(t) => warn!(target: RUNNER, "coordinator thread completed: {t:?}"),
        Err(e) => warn!(target: RUNNER, "coordinator thread completed with error: {e:?}"),
    }

    Ok(())
}
