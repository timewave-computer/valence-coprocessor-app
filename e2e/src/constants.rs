//! Shared constants for token IBC Eureka transfer system
//!
//! All hardcoded values are centralized here for use in e2e tests
//! and across the strategist, controller, circuit components.

/// Token being transferred (currently LBTC, but can be changed for other tokens)
pub const TOKEN: &str = "LBTC";

/// Token contract address on Ethereum (currently LBTC)
pub const TOKEN_CONTRACT_ADDRESS: &str = "0x8236a87084f8B84306f72007F36F2618A5634494";

/// Token denomination on Cosmos Hub (IBC denom)
pub const TOKEN_COSMOS_HUB_DENOM: &str =
    "ibc/DBD9E339E1B093A052D76BECFFDE8435EAC114CF2133346B4D691F3F2068C957";

/// Expected route hash for token IBC Eureka transfers (SHA3-256)
pub const EXPECTED_ROUTE_HASH: &str =
    "a041afeb1546e275ec0038183732036ce653b197e8129748da95cf6c7de43abf";

/// Expected destination address for token transfers (cosmos1...)
pub const EXPECTED_DESTINATION: &str =
    "cosmos1zxj6y5h3r8k9v7n2m4l1q8w5e3t6y9u0i7o4p2s5d8f6g3h1j4k7l9n2";

/// Fee threshold in token wei (0.0000189 LBTC = $2.00 equivalent)
pub const FEE_THRESHOLD_TOKEN_WEI: u64 = 1890000000000000;

/// Expected source chain ID (Ethereum)
pub const EXPECTED_SOURCE_CHAIN: &str = "1";

/// Expected destination chain ID (Cosmos Hub)
pub const EXPECTED_DEST_CHAIN: &str = "cosmoshub-4";

/// Expected bridge ID for IBC Eureka transfers
pub const EXPECTED_BRIDGE_ID: &str = "EUREKA";

/// Expected entry contract address (IBCEurekaTransfer)
pub const EXPECTED_ENTRY_CONTRACT: &str = "0xFc2d0487A0ae42ae7329a80dc269916A9184cF7C";

/// Registry ID for token transfer messages in Valence
pub const TOKEN_TRANSFER_REGISTRY_ID: u64 = 1001;

/// Skip API base URL
pub const SKIP_API_BASE_URL: &str = "https://api.skip.build";

/// Default coprocessor URL for local development
pub const LOCAL_COPROCESSOR_URL: &str = "http://localhost:37281";

/// Public coprocessor URL  
pub const PUBLIC_COPROCESSOR_URL: &str = "http://prover.timewave.computer:37281";

/// Default Ethereum RPC URL for local development (Anvil)
pub const LOCAL_ETHEREUM_RPC_URL: &str = "http://127.0.0.1:8545";

/// Ethereum mainnet RPC URL template (replace with actual API key)
pub const MAINNET_ETHEREUM_RPC_URL_TEMPLATE: &str = "https://mainnet.infura.io/v3/{API_KEY}";

/// Test mnemonic for local development (DO NOT USE IN PRODUCTION)
pub const TEST_MNEMONIC: &str = "test test test test test test test test test test test junk";

/// LBTC decimal places (18 decimal places like ETH)
pub const LBTC_DECIMALS: u8 = 18;

/// Maximum acceptable proof generation time (seconds)
pub const MAX_PROOF_GENERATION_TIME_SECONDS: u64 = 30;

/// Maximum acceptable API response time (seconds)
pub const MAX_API_RESPONSE_TIME_SECONDS: u64 = 5;

/// Maximum acceptable end-to-end flow time (seconds)
pub const MAX_END_TO_END_TIME_SECONDS: u64 = 60;

/// Real Ethereum mainnet RPC URLs for testing (no fallbacks)
pub const ETHEREUM_RPC_URLS: &[&str] = &[
    "https://rpc.ankr.com/eth",
    "https://eth.public-rpc.com",
    "https://ethereum.publicnode.com",
    "https://rpc.payload.de",
];

/// Real Cosmos Hub RPC URLs for testing (no fallbacks)
pub const COSMOS_HUB_RPC_URLS: &[&str] = &[
    "https://cosmos-rpc.polkachu.com:443",
    "https://rpc-cosmoshub.keplr.app:443",
    "https://cosmos-rpc.stakely.io:443",
    "https://rpc.cosmos.directory/cosmoshub",
];

/// Environment variable names for configuration
pub const ENV_ETHEREUM_RPC_URL: &str = "ETHEREUM_RPC_URL";
pub const ENV_COPROCESSOR_URL: &str = "COPROCESSOR_URL";
pub const ENV_MNEMONIC: &str = "MNEMONIC";
pub const ENV_SKIP_API_KEY: &str = "SKIP_API_KEY";

/// Test transfer amounts in token wei for e2e testing
pub const TEST_TRANSFER_AMOUNTS: &[u64] = &[
    1000000000000000, // 0.001 LBTC (below threshold - should pass)
    1800000000000000, // 0.0018 LBTC (below threshold - should pass)
    1890000000000000, // 0.0189 LBTC (at threshold - should pass)
    2000000000000000, // 0.002 LBTC (above threshold - should fail)
    5000000000000000, // 0.005 LBTC (well above threshold - should fail)
];
