use common::provisioner_dir;
use log::info;
use serde::Deserialize;
use std::fs;

#[derive(Debug, Clone, Deserialize)]
pub struct NeutronInputs {
    pub grpc_url: String,
    pub grpc_port: String,
    pub chain_id: String,
    pub code_ids: CodeIds,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CodeIds {
    pub authorizations: u64,
    pub processor: u64,
    pub cw20: u64,
}

const READ_INPUTS: &str = "READ_INPUTS";

pub fn read_setup_inputs(input_file: &str) -> anyhow::Result<NeutronInputs> {
    let input_dir = provisioner_dir()
        .join("src")
        .join("inputs")
        .join(input_file);
    info!(target: READ_INPUTS, "reading inputs from {}...", input_dir.display());

    let parameters = fs::read_to_string(input_dir)?;

    let neutron_inputs: NeutronInputs = toml::from_str(&parameters)?;

    info!(target: READ_INPUTS, "neutron inputs from step: {neutron_inputs:?}");

    Ok(neutron_inputs)
}
