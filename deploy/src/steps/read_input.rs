use common::INPUTS_DIR;
use serde::Deserialize;
use std::{fs, path::PathBuf};

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

pub fn read_setup_inputs(cd: PathBuf) -> anyhow::Result<NeutronInputs> {
    println!("reading inputs...");

    let input_dir = cd.join(format!("{INPUTS_DIR}/neutron_inputs.toml"));
    let parameters = fs::read_to_string(input_dir)?;

    let neutron_inputs: NeutronInputs = toml::from_str(&parameters)?;

    println!("neutron inputs from step: {neutron_inputs:?}");

    Ok(neutron_inputs)
}
