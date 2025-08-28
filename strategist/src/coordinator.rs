use async_trait::async_trait;
use common::ZK_MINT_CW20_LABEL;
use cw20::{BalanceResponse, Cw20QueryMsg};
use log::info;
use valence_coordinator_sdk::coordinator::ValenceCoordinator;
use valence_domain_clients::{
    coprocessor::base_client::{Base64, CoprocessorBaseClient, Proof},
    cosmos::{grpc_client::GrpcSigningClient, wasm_client::WasmClient},
};

use crate::strategy::Strategy;

const COORDINATOR_LOG_TARGET: &str = "COORDINATOR";

// implement the ValenceCoordinator trait for the Strategy struct.
// This trait defines the main loop of the strategy and inherits
// the default implementation for spawning the coordinator.
#[async_trait]
impl ValenceCoordinator for Strategy {
    fn get_name(&self) -> String {
        format!("Valence Coprocessor App: {}", self.label)
    }

    async fn cycle(&mut self) -> anyhow::Result<()> {
        info!(target: COORDINATOR_LOG_TARGET, "{}: Starting cycle...", self.get_name());

        let ntrn_addr = self
            .neutron_client
            .get_signing_client()
            .await?
            .address
            .to_string();

        let circuit_inputs = storage_proof_core::ControllerInputs {
            erc20_addr: self.erc20_addr.to_string(),
            eth_addr: self.erc20_holder_addr.to_string(),
            neutron_addr: ntrn_addr.to_string(),
            erc20_balances_map_storage_index: 9, // usdc is 9
        };

        let proof_request = serde_json::to_value(circuit_inputs)?;
        info!(target: COORDINATOR_LOG_TARGET, "posting proof request: {proof_request}");

        // submit the proof request to the coprocessor
        let resp = self
            .coprocessor_client
            .prove(&self.neutron_cfg.coprocessor_app_id, &proof_request)
            .await?;

        info!(target: COORDINATOR_LOG_TARGET, "received zkp: {resp:?}");

        // extract the program and domain parameters by decoding the zkp
        let program_proof = decode(resp.program)?;
        let domain_proof = decode(resp.domain)?;

        let cw20_bal_query = Cw20QueryMsg::Balance {
            address: ntrn_addr.to_string(),
        };
        let cw20_balance: BalanceResponse = self
            .neutron_client
            .query_contract_state(&self.neutron_cfg.cw20, &cw20_bal_query)
            .await?;
        info!(target: COORDINATOR_LOG_TARGET, "cw20 balance pre-proof: {cw20_balance:?}");

        // execute the zk authorization. this will perform the verification
        // and, if successful, push the msg to the processor
        info!(target: COORDINATOR_LOG_TARGET, "posting zkp to the authorizations contract");
        valence_coordinator_sdk::core::cw::post_zkp_on_chain(
            &self.neutron_client,
            &self.neutron_cfg.authorizations,
            ZK_MINT_CW20_LABEL,
            program_proof,
            domain_proof,
        )
        .await?;

        // tick the processor
        info!(target: COORDINATOR_LOG_TARGET, "ticking the processor...");
        valence_coordinator_sdk::core::cw::tick(&self.neutron_client, &self.neutron_cfg.processor)
            .await?;

        let cw20_balance: BalanceResponse = self
            .neutron_client
            .query_contract_state(&self.neutron_cfg.cw20, cw20_bal_query)
            .await?;
        info!(target: COORDINATOR_LOG_TARGET, "cw20 balance post-proof: {cw20_balance:?}");

        Ok(())
    }
}

fn decode(a: Proof) -> anyhow::Result<(Vec<u8>, Vec<u8>)> {
    let proof = Base64::decode(&a.proof)?;
    let inputs = Base64::decode(&a.inputs)?;

    Ok((proof, inputs))
}
