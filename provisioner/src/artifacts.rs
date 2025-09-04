use std::fs;

use anyhow::anyhow;
use common::artifacts_dir;
use log::info;
use serde::{Deserialize, Serialize};

use crate::PROVISIONER;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InstantiationOutputs {
    pub authorizations: String,
    pub processor: String,
    pub cw20: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CoprocessorOutputs {
    pub coprocessor_app_id: String,
}

pub(crate) fn write_instantiation_artifacts(outputs: InstantiationOutputs) -> anyhow::Result<()> {
    let path = artifacts_dir().join("instantiation_outputs.toml");
    info!(target: PROVISIONER, "writing on-chain instantiation artifacts to {}", path.display());
    fs::write(path, toml::to_string(&outputs)?)?;
    Ok(())
}

pub(crate) fn read_instantiation_artifacts() -> anyhow::Result<InstantiationOutputs> {
    let path = artifacts_dir().join("instantiation_outputs.toml");
    let content = fs::read_to_string(path).map_err(|_| {
        anyhow!(
            "on-chain instantiation artifacts not found. run --instantiate-contracts step first."
        )
    })?;
    toml::from_str(&content)
        .map_err(|e| anyhow::anyhow!("failed to reconstruct instantiation outputs: {e}"))
}

pub(crate) fn write_coprocessor_artifacts(outputs: CoprocessorOutputs) -> anyhow::Result<()> {
    let path = artifacts_dir().join("coprocessor_outputs.toml");
    info!(target: PROVISIONER, "writing co-processor deployment artifacts to {}", path.display());
    fs::write(path, toml::to_string(&outputs)?)?;
    Ok(())
}

pub(crate) fn read_coprocessor_artifacts() -> anyhow::Result<CoprocessorOutputs> {
    let path = artifacts_dir().join("coprocessor_outputs.toml");
    let content = fs::read_to_string(path).map_err(|_| {
        anyhow!("co-processor artifacts not found. run --deploy-coprocessor step first.")
    })?;
    toml::from_str(&content)
        .map_err(|e| anyhow::anyhow!("failed to reconstruct coprocessor step outputs: {e}"))
}
