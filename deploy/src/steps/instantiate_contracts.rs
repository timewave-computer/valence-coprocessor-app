use std::time::SystemTime;

use cw20::MinterResponse;
use valence_domain_clients::{
    clients::neutron::NeutronClient,
    cosmos::{base_client::BaseClient, grpc_client::GrpcSigningClient, wasm_client::WasmClient},
};

use crate::steps::read_input::CodeIds;

const VALENCE_NEUTRON_VERIFICATION_GATEWAY: &str =
    "neutron1l3fgzcqse0xw84hdpytg7vcp04kcdm95wes2zd6ap8kpujmv9cwsv45wwk";

pub struct InstantiationOutputs {
    pub cw20: String,
    pub processor: String,
    pub authorizations: String,
}

pub async fn instantiate_contracts(
    neutron_client: &NeutronClient,
    code_ids: CodeIds,
) -> anyhow::Result<InstantiationOutputs> {
    println!("instantiating contracts...");

    let my_address = neutron_client
        .get_signing_client()
        .await?
        .address
        .to_string();

    println!("runner address: {my_address}");

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

    println!("predicted processor addr: {predicted_processor_address}");

    // Owner will initially be the deploy address and eventually will be transferred to the owned address
    let authorization_instantiate_msg = valence_authorization_utils::msg::InstantiateMsg {
        owner: my_address.to_string(),
        sub_owners: vec![],
        processor: predicted_processor_address.clone(),
    };

    println!("instantiating authorization address...");
    let authorization_address = neutron_client
        .instantiate2(
            code_ids.authorizations,
            "authorization".to_string(),
            authorization_instantiate_msg,
            Some(my_address.to_string()),
            salt.clone(),
        )
        .await?;
    println!("Authorization instantiated: {authorization_address}");

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
    println!("Processor instantiated: {processor_address}");

    // Set the verification gateway address on the authorization contract
    let set_verification_gateway_msg =
        valence_authorization_utils::msg::ExecuteMsg::PermissionedAction(
            valence_authorization_utils::msg::PermissionedMsg::SetVerificationGateway {
                verification_gateway: VALENCE_NEUTRON_VERIFICATION_GATEWAY.to_string(),
            },
        );

    let set_verification_gateway_rx = neutron_client
        .execute_wasm(
            &authorization_address,
            set_verification_gateway_msg,
            vec![],
            None,
        )
        .await?;

    neutron_client
        .poll_for_tx(&set_verification_gateway_rx.hash)
        .await?;

    println!("Set verification gateway address to {VALENCE_NEUTRON_VERIFICATION_GATEWAY}");

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

    println!("CW20 Instantiated: {cw20_addr}");

    let outputs = InstantiationOutputs {
        cw20: cw20_addr,
        processor: processor_address,
        authorizations: authorization_address,
    };

    Ok(outputs)
}
