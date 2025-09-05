use std::fs;

use common::{artifacts_dir, NeutronStrategyConfig};
use log::info;

const WRITE_OUTPUTS: &str = "WRITE_OUTPUTS";

pub fn write_setup_artifacts(neutron_cfg: NeutronStrategyConfig) -> anyhow::Result<()> {
    info!(target: WRITE_OUTPUTS, "writing outputs...");

    // Save the Neutron Strategy Config to a toml file
    let neutron_cfg_toml = toml::to_string(&neutron_cfg)?;

    let target_path = artifacts_dir().join("neutron_strategy_config.toml");
    info!(target: WRITE_OUTPUTS, "writing neutron_strategy_config.toml to: {target_path:?}");

    fs::write(target_path, neutron_cfg_toml)?;

    Ok(())
}
