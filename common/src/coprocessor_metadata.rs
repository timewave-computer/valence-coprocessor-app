use serde::{Deserialize, Serialize};

pub const MAX_ITEMS: usize = 32;

/// hash type alias
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hash32(pub [u8; 32]);

/// valence program shard metadata type that contains:
/// - developer origin information
/// - build artifact hashes
/// - program that the shard belongs to
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardMetadata {
    /// shared metadata version
    shard_metadata_version: String,
    /// top level program information
    program_info: Program,
    /// Authorization contract address (domain-specific origin)
    shard_id: [Hash32; MAX_ITEMS],
    /// Optional SemVer-style string
    shard_version: Option<String>,
    /// Valence domain namespace (e.g., "ethereum") that this hash is anchored into
    anchor_domain: String,
    /// Valence verification gateway version
    valence_version: String,

    /// compiled artifact hashes and vks
    artifacts: Artifacts,

    /// developer origin information
    origin: Origin,
}

/// information about the program which this shard belongs to.
/// if a given Valence Program spans over multiple domains, all
/// of those shards should have an identical program object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Program {
    /// program IDs (e.g., historical upgrades or replicas)
    pub program_id: [Hash32; MAX_ITEMS],
    /// Optional SemVer-style string
    pub program_version: Option<String>,
}

/// hashes of the compiled elf binaries and verification
/// keys stored on-chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifacts {
    /// SHA-256 hashes of controller WASM bytecode
    pub controller_wasm_hashes: [Hash32; MAX_ITEMS],
    /// SHA-256 hashes of circuit ELF binaries
    pub circuit_elf_hashes: [Hash32; MAX_ITEMS],

    /// SHA-256 of verification keys (as stored onchain)
    pub verification_keys: [Hash32; MAX_ITEMS],

    /// IDs of authorized executions associated with accounts
    pub registry_id: [Hash32; MAX_ITEMS],

    /// Should include verifier version
    pub route: String,
}

/// metadata about the Valence Program developer
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

    /// Application-defined extra metadata
    pub additional_metadata: Option<Hash32>,
}
