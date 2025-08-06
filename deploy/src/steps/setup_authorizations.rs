use common::{NeutronStrategyConfig, REGULAR_MINT_CW20_LABEL, ZK_MINT_CW20_LABEL};
use cosmwasm_std::Binary;
use sp1_sdk::{HashableKey, SP1VerifyingKey};
use valence_authorization_utils::{
    authorization::{AuthorizationModeInfo, PermissionTypeInfo},
    authorization_message::{Message, MessageDetails, MessageType},
    builders::{AtomicSubroutineBuilder, AuthorizationBuilder},
    domain::Domain,
    function::AtomicFunction,
    zk_authorization::ZkAuthorizationInfo,
};
use valence_domain_clients::{
    clients::{coprocessor::CoprocessorClient, neutron::NeutronClient},
    coprocessor::base_client::CoprocessorBaseClient,
    cosmos::{base_client::BaseClient, grpc_client::GrpcSigningClient, wasm_client::WasmClient},
};
use valence_library_utils::LibraryAccountType;

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

    // creating equivalent variants for cw20 minting for both zk and regular auth mode
    create_regular_cw20_mint_authorization(
        neutron_client,
        ntrn_strategy_config,
        authorization_permissioned_mode.clone(),
    )
    .await?;

    create_zk_cw20_mint_authorization(
        neutron_client,
        ntrn_strategy_config,
        authorization_permissioned_mode,
    )
    .await?;

    Ok(())
}

async fn create_regular_cw20_mint_authorization(
    neutron_client: &NeutronClient,
    cfg: &NeutronStrategyConfig,
    authorization_mode: AuthorizationModeInfo,
) -> anyhow::Result<()> {
    let cw20_mint_function = AtomicFunction {
        domain: Domain::Main,
        message_details: MessageDetails {
            message_type: MessageType::CosmwasmExecuteMsg,
            message: Message {
                name: "mint".to_string(),
                params_restrictions: None,
            },
        },
        contract_address: LibraryAccountType::Addr(cfg.cw20.to_string()),
    };

    let subroutine_mint_cw20 = AtomicSubroutineBuilder::new()
        .with_function(cw20_mint_function)
        .build();

    let authorization_cw20_mint = AuthorizationBuilder::new()
        .with_label(REGULAR_MINT_CW20_LABEL)
        .with_mode(authorization_mode.clone())
        .with_subroutine(subroutine_mint_cw20)
        .build();

    let regular_authorizations = vec![authorization_cw20_mint];

    let create_authorizations = valence_authorization_utils::msg::ExecuteMsg::PermissionedAction(
        valence_authorization_utils::msg::PermissionedMsg::CreateAuthorizations {
            authorizations: regular_authorizations,
        },
    );

    let create_authorizations_rx = neutron_client
        .execute_wasm(&cfg.authorizations, create_authorizations, vec![], None)
        .await?;
    neutron_client
        .poll_for_tx(&create_authorizations_rx.hash)
        .await?;

    println!(
        "Successfully created non-zk cw20 mint authorization with label {REGULAR_MINT_CW20_LABEL}"
    );
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
