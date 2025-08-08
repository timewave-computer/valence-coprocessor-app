use std::env;

use common::NeutronStrategyConfig;
use valence_domain_clients::clients::{coprocessor::CoprocessorClient, neutron::NeutronClient};

pub struct Strategy {
    /// strategy name
    pub label: String,

    /// strategy timeout (sec)
    pub timeout: u64,

    /// source erc20 address
    pub erc20_addr: String,
    pub erc20_balances_storage_index: u64,
    pub erc20_holder_addr: String,

    /// active neutron client and strategy config
    pub(crate) neutron_cfg: NeutronStrategyConfig,
    pub(crate) neutron_client: NeutronClient,

    /// active co-processor client
    pub(crate) coprocessor_client: CoprocessorClient,
}

impl Strategy {
    /// strategy initializer that takes in a `StrategyConfig`, and uses it
    /// to initialize the respective domain clients. prerequisite to starting
    /// the strategist.
    pub async fn new(cfg: NeutronStrategyConfig) -> anyhow::Result<Self> {
        dotenv::dotenv().ok();

        // fetch the env variables used to build the strategy
        let mnemonic = env::var("MNEMONIC")?;
        let label = env::var("LABEL")?;
        let erc20_addr = env::var("ERC20_ADDR")?;
        let strategy_timeout: u64 = env::var("STRATEGY_TIMEOUT")?.parse()?;
        let erc20_balances_storage_index: u64 =
            env::var("ERC20_BALANCES_STORAGE_INDEX")?.parse()?;
        let erc20_src_addr = env::var("ETH_SRC_ADDR")?;

        let neutron_client =
            NeutronClient::new(&cfg.grpc_url, &cfg.grpc_port, &mnemonic, &cfg.chain_id).await?;

        let coprocessor_client = CoprocessorClient::default();

        Ok(Self {
            timeout: strategy_timeout,
            neutron_client,
            label,
            coprocessor_client,
            neutron_cfg: cfg,
            erc20_addr,
            erc20_balances_storage_index,
            erc20_holder_addr: erc20_src_addr,
        })
    }
}
