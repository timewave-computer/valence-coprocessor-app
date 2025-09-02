use std::{fs, path::PathBuf};

use common::NeutronStrategyConfig;
use common::OUTPUTS_DIR;

pub fn write_setup_artifacts(
    cd: PathBuf,
    neutron_cfg: NeutronStrategyConfig,
) -> anyhow::Result<()> {
    println!("writing outputs...");

    // Save the Neutron Strategy Config to a toml file
    let neutron_cfg_toml = toml::to_string(&neutron_cfg)?;

    let target_path = cd.join(format!("{OUTPUTS_DIR}/neutron_strategy_config.toml"));
    println!("writing neutron_strategy_config.toml to: {target_path:?}");

    fs::write(target_path, neutron_cfg_toml)?;

    Ok(())
}
