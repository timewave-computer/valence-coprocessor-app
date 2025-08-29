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
    /// top level program information. this can be shared
    /// across multiple shards.
    pub program_info: Program,

    /// target shard information
    pub shard: Shard,

    /// compiled artifact hashes and vks
    pub artifacts: Artifacts,

    /// developer origin information
    pub dev_origin: Origin,

    /// Application-defined extra metadata
    pub additional_metadata: Option<Hash32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Shard {
    /// shared metadata version
    pub shard_metadata_version: String,
    /// hash of the authorization contract address (domain-specific)
    pub shard_id: Hash32,
    /// Optional SemVer-style string
    pub shard_version: Option<String>,

    /// Valence domain namespace (e.g., "ethereum") that this hash is anchored into
    pub anchor_domain: String,

    /// Valence verification gateway version
    pub valence_version: String,
}

/// information about the program which this shard belongs to.
/// if a given Valence Program spans over multiple domains, all
/// of those shards should have an identical program object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Program {
    /// program ID
    pub program_id: Hash32,
    /// Optional SemVer-style string
    pub program_version: Option<String>,
}

/// hashes of the compiled circuits and verification
/// keys stored on-chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifacts {
    /// SHA-256 hashes of controller WASM bytecode
    pub controller_wasm_hashes: [Hash32; MAX_ITEMS],
    /// SHA-256 hashes of circuit ELF binaries
    pub circuit_elf_hashes: [Hash32; MAX_ITEMS],

    /// SHA-256 of verification keys (as stored onchain)
    pub verification_keys: [Hash32; MAX_ITEMS],

    /// labels associated with authorized executions
    // TODO: do we want to hash these?
    // also is this the right place for auth labels and route?
    pub authorization_labels: [String; MAX_ITEMS],

    /// Route used by the verification gateway, e.g.
    /// - ethereum/1.0.0/sp1/2.0.0
    pub verification_route: String,
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
}
