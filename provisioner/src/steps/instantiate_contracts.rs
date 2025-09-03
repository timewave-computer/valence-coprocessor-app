use std::time::SystemTime;

use cw20::MinterResponse;
use log::info;
use valence_domain_clients::{
    clients::neutron::NeutronClient,
    cosmos::{base_client::BaseClient, grpc_client::GrpcSigningClient, wasm_client::WasmClient},
};

use crate::{consts::VALENCE_NEUTRON_VERIFICATION_ROUTER, steps::read_input::CodeIds};

const CONTRACT_DEPLOYMENT: &str = "CONTRACT_DEPLOYMENT";

pub struct InstantiationOutputs {
    pub cw20: String,
    pub processor: String,
    pub authorizations: String,
}

pub async fn instantiate_contracts(
    neutron_client: &NeutronClient,
    code_ids: CodeIds,
) -> anyhow::Result<InstantiationOutputs> {
    info!(target: CONTRACT_DEPLOYMENT, "instantiating contracts...");

    let my_address = neutron_client
        .get_signing_client()
        .await?
        .address
        .to_string();

    info!(target: CONTRACT_DEPLOYMENT, "runner address: {my_address}");

    let now = SystemTime::now();
    let salt_raw = now
        .duration_since(SystemTime::UNIX_EPOCH)?
        .as_secs()
        .to_string();
    let salt = hex::encode(salt_raw.as_bytes());

    let predicted_processor_address = neutron_client
        .predict_instantiate2_addr(code_ids.processor, salt.clone(), my_address.clone())
        .await?
        .address;

    info!(target: CONTRACT_DEPLOYMENT, "predicted processor addr: {predicted_processor_address}");

    // Owner will initially be the deploy address and eventually will be transferred to the owned address
    let authorization_instantiate_msg = valence_authorization_utils::msg::InstantiateMsg {
        owner: my_address.to_string(),
        sub_owners: vec![],
        processor: predicted_processor_address.clone(),
    };

    info!(target: CONTRACT_DEPLOYMENT, "instantiating authorization address...");
    let authorization_address = neutron_client
        .instantiate2(
            code_ids.authorizations,
            "authorization".to_string(),
            authorization_instantiate_msg,
            Some(my_address.to_string()),
            salt.clone(),
        )
        .await?;
    info!(target: CONTRACT_DEPLOYMENT, "Authorization instantiated: {authorization_address}");

    let processor_instantiate_msg = valence_processor_utils::msg::InstantiateMsg {
        authorization_contract: authorization_address.clone(),
        polytone_contracts: None,
    };

    let processor_address = neutron_client
        .instantiate2(
            code_ids.processor,
            "processor".to_string(),
            processor_instantiate_msg,
            Some(my_address.to_string()),
            salt.clone(),
        )
        .await?;
    info!(target: CONTRACT_DEPLOYMENT, "Processor instantiated: {processor_address}");

    // Set the verification gateway address on the authorization contract
    let set_verification_router_msg =
        valence_authorization_utils::msg::ExecuteMsg::PermissionedAction(
            valence_authorization_utils::msg::PermissionedMsg::SetVerificationRouter {
                address: VALENCE_NEUTRON_VERIFICATION_ROUTER.to_string(),
            },
        );

    info!(target: CONTRACT_DEPLOYMENT, "Setting authorizations verification router: {VALENCE_NEUTRON_VERIFICATION_ROUTER}");
    let set_verification_router_rx = neutron_client
        .execute_wasm(
            &authorization_address,
            set_verification_router_msg,
            vec![],
            None,
        )
        .await?;

    neutron_client
        .poll_for_tx(&set_verification_router_rx.hash)
        .await?;

    info!(target: CONTRACT_DEPLOYMENT, "Verification router set!");

    let cw20_init_msg = cw20_base::msg::InstantiateMsg {
        name: "test_playground".to_string(),
        symbol: "CWBASETEST".to_string(),
        decimals: 18,
        initial_balances: vec![],
        mint: Some(MinterResponse {
            minter: processor_address.to_string(),
            cap: None,
        }),
        marketing: None,
    };

    let cw20_addr = neutron_client
        .instantiate(
            code_ids.cw20,
            "mirror_cw20".to_string(),
            cw20_init_msg,
            Some(my_address),
        )
        .await?;

    info!(target: CONTRACT_DEPLOYMENT, "CW20 Instantiated: {cw20_addr}");

    let outputs = InstantiationOutputs {
        cw20: cw20_addr,
        processor: processor_address,
        authorizations: authorization_address,
    };

    Ok(outputs)
}
