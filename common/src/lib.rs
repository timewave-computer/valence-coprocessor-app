use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub const ZK_MINT_CW20_LABEL: &str = "zk_mint_cw20";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeutronStrategyConfig {
    // node info
    pub grpc_url: String,
    pub grpc_port: String,
    pub chain_id: String,

    // contracts
    pub authorizations: String,
    pub processor: String,
    pub cw20: String,

    // coprocessor app id
    pub coprocessor_app_id: String,
}

pub fn workspace_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("failed to cd to workspace root dir")
        .to_path_buf()
}

pub fn artifacts_dir() -> PathBuf {
    workspace_dir().join("artifacts")
}

pub fn provisioner_dir() -> PathBuf {
    workspace_dir().join("provisioner")
}
