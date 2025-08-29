use serde::{Deserialize, Serialize};

pub const MAX_ITEMS: usize = 32;

/// hash type alias
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hash32(pub [u8; 32]);

/// coprocessor metadata type that contains:
/// - developer origin information
/// - build artifact hashes
/// -
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoprocessorMetadata {
    /// ----
    shard_metadata_version: String,

    /// 32 program IDs (e.g., historical upgrades or replicas)
    program_id: [Hash32; 32],
    /// Optional SemVer-style string
    program_version: Option<String>,

    /// Authorization contract address (domain-specific origin)
    shard_id: [Hash32; 32],

    /// Optional SemVer-style string
    shard_version: Option<String>,
    /// Valence domain namespace (e.g., "ethereum") that this hash is anchored into
    anchor_domain: String,

    /// Valence verification gateway version
    valence_version: String,

    registry_id: [Hash32; MAX_ITEMS], // IDs of authorized executions associated with accounts

    /// Should include verifier version
    route: String,

    artifacts: Artifacts,

    origin: Origin,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifacts {
    /// SHA-256 hashes of controller WASM bytecode
    pub controller_wasm_hashes: [Hash32; MAX_ITEMS],
    /// SHA-256 hashes of circuit ELF binaries
    pub circuit_elf_hashes: [Hash32; MAX_ITEMS],
    /// SHA-256 of verification keys (as stored onchain)
    pub verification_keys: [Hash32; MAX_ITEMS],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Origin {
    /// Issued manually
    pub developer_id: Hash32,
    /// Team or individual name
    pub developer_name: Option<String>,

    /// GitHub/GitLab URL (64 is safer here)
    pub repo_url: String,
    /// Git commit (SHA-1 or SHA-256)
    pub repo_commit_hash: Hash32,

    /// Application-defined (consider `Option<Hash32>`)
    pub additional_metadata: Hash32,
}
