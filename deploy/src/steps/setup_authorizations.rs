use common::{NeutronStrategyConfig, ZK_MINT_CW20_LABEL};
use cosmwasm_std::Binary;
use sp1_sdk::{HashableKey, SP1VerifyingKey};
use valence_authorization_utils::{
    authorization::{AuthorizationModeInfo, PermissionTypeInfo},
    zk_authorization::ZkAuthorizationInfo,
};
use valence_domain_clients::{
    clients::{coprocessor::CoprocessorClient, neutron::NeutronClient},
    coprocessor::base_client::CoprocessorBaseClient,
    cosmos::{base_client::BaseClient, grpc_client::GrpcSigningClient, wasm_client::WasmClient},
};

pub async fn setup_authorizations(
    neutron_client: &NeutronClient,
    ntrn_strategy_config: &NeutronStrategyConfig,
) -> anyhow::Result<()> {
    println!("setting up authorizations...");
    let my_address = neutron_client
        .get_signing_client()
        .await?
        .address
        .to_string();

    println!("my address: {my_address}");

    let authorization_permissioned_mode =
        AuthorizationModeInfo::Permissioned(PermissionTypeInfo::WithoutCallLimit(vec![
            my_address.to_string(),
        ]));

    // creating cw20 minting zk authorization
    create_zk_cw20_mint_authorization(
        neutron_client,
        ntrn_strategy_config,
        authorization_permissioned_mode,
    )
    .await?;

    Ok(())
}

async fn create_zk_cw20_mint_authorization(
    neutron_client: &NeutronClient,
    cfg: &NeutronStrategyConfig,
    authorization_mode: AuthorizationModeInfo,
) -> anyhow::Result<()> {
    let coprocessor_client = CoprocessorClient::default();
    let program_vk = coprocessor_client.get_vk(&cfg.coprocessor_app_id).await?;

    let sp1_program_vk: SP1VerifyingKey = bincode::deserialize(&program_vk)?;

    let zk_authorization = ZkAuthorizationInfo {
        label: ZK_MINT_CW20_LABEL.to_string(),
        mode: authorization_mode,
        registry: 0,
        vk: Binary::from(sp1_program_vk.bytes32().as_bytes()),
        validate_last_block_execution: false,
    };

    let create_zk_authorization = valence_authorization_utils::msg::ExecuteMsg::PermissionedAction(
        valence_authorization_utils::msg::PermissionedMsg::CreateZkAuthorizations {
            zk_authorizations: vec![zk_authorization],
        },
    );

    println!("creating ZK authorization...");

    let create_zk_auth_rx = neutron_client
        .execute_wasm(&cfg.authorizations, create_zk_authorization, vec![], None)
        .await?;

    neutron_client.poll_for_tx(&create_zk_auth_rx.hash).await?;

    println!("ZK Authorization created successfully");

    Ok(())
}
