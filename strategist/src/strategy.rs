use std::env;

use common::NeutronStrategyConfig;
use valence_domain_clients::clients::{coprocessor::CoprocessorClient, neutron::NeutronClient};

pub struct Strategy {
    /// strategy name
    pub label: String,

    /// strategy timeout (sec)
    pub timeout: u64,

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

        let mnemonic = env::var("MNEMONIC")?;
        let label = env::var("LABEL")?;
        let strategy_timeout: u64 = env::var("STRATEGY_TIMEOUT")?.parse()?;

        let neutron_client =
            NeutronClient::new(&cfg.grpc_url, &cfg.grpc_port, &mnemonic, &cfg.chain_id).await?;

        let coprocessor_client = CoprocessorClient::default();

        Ok(Self {
            timeout: strategy_timeout,
            neutron_client,
            label,
            coprocessor_client,
            neutron_cfg: cfg,
        })
    }
}
