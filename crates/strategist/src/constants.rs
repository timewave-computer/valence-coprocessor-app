//! Shared constants for Token IBC Eureka transfer system
//! 
//! All hardcoded values are centralized here for use in e2e tests
//! and across the strategist, controller, circuit components.

/// Token being transferred (currently LBTC, but can be changed for other tokens)
pub const TOKEN: &str = "LBTC";

/// Token contract address on Ethereum (currently LBTC)
pub const TOKEN_CONTRACT_ADDRESS: &str = "0x8236a87084f8B84306f72007F36F2618A5634494";

/// Token denomination on Cosmos Hub (IBC denom)
pub const TOKEN_COSMOS_HUB_DENOM: &str = "ibc/DBD9E339E1B093A052D76BECFFDE8435EAC114CF2133346B4D691F3F2068C957";

/// Expected route hash for Token IBC Eureka transfers (SHA3-256)
pub const EXPECTED_ROUTE_HASH: &str = "a041afeb1546e275ec0038183732036ce653b197e8129748da95cf6c7de43abf";

/// Expected destination address for Token transfers (cosmos1...)
pub const EXPECTED_DESTINATION: &str = "cosmos1zxj6y5h3r8k9v7n2m4l1q8w5e3t6y9u0i7o4p2s5d8f6g3h1j4k7l9n2";

/// Fee threshold in token wei (0.0000189 TOKEN = $2.00 equivalent)
pub const FEE_THRESHOLD_TOKEN_WEI: u64 = 1890000000000000;

/// Expected source chain ID (Ethereum)
pub const EXPECTED_SOURCE_CHAIN: &str = "1";

/// Expected destination chain ID (Cosmos Hub)
pub const EXPECTED_DEST_CHAIN: &str = "cosmoshub-4";

/// Expected bridge ID for IBC Eureka transfers
pub const EXPECTED_BRIDGE_ID: &str = "EUREKA";

/// Expected entry contract address (IBCEurekaTransfer)
pub const EXPECTED_ENTRY_CONTRACT: &str = "0xFc2d0487A0ae42ae7329a80dc269916A9184cF7C";

/// Registry ID for Token transfer messages in Valence
pub const TOKEN_TRANSFER_REGISTRY_ID: u64 = 1001;

/// Skip API base URL
pub const SKIP_API_BASE_URL: &str = "https://api.skip.build"; 