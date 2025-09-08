# Provisioning Setup

This document provides a detailed guide on how to provision the Valence co-processor app.

The provisioning process sets up all the necessary on-chain and off-chain components
and binds them together, preparing the environment for the coordinator operations.

## Overview

The provisioning process is managed by the `provisioner` crate.

It is a command-line tool that executes a series of steps to get the system ready.
The provisioner can be executed in the following ways:

- targeting a specific step
- targeting all steps at once, in order

The provisioning process consists of the following steps:

1. **Read Input**: reads the initial configuration from `provisioner/src/inputs/neutron_inputs.toml`
2. **Instantiate Contracts**: deploys the necessary smart contracts on the Neutron domain
3. **Deploy Co-processor App**: builds and deploys the co-processor application (circuit and controller) to the co-processor
4. **Setup Authorizations**: configures the on-chain authorizations to link the deployed contracts with the co-processor application
5. **Write Output**: writes the final configuration to `artifacts/neutron_strategy_config.toml`, to be consumed by the coordinator

## Prerequisites

Before running the provisioner, make sure you have configured your local development environment:

1. Nix is installed and available (see [environment setup documentation](./environment.md))
2. `.env` file is available with a valid mnemonic

## How to Run

To run the full provisioning process, execute the following:

```sh
RUST_LOG=info cargo run --bin provisioner
```

You can also run a specific step of the provisioning process by using the `--step` flag:

- `--step instantiate-contracts`: instantiates the on-chain contracts
- `--step deploy-coprocessor`: deploys the co-processor application
- `--step authorize`: sets up the on-chain authorizations

For example, to only instantiate the contracts, you would run:

```sh
RUST_LOG=info cargo run --bin provisioner -- --step instantiate-contracts
```

You can execute `cargo run --bin provisioner -- --help` to see the cli tooling commands available.

## Provisioning Steps

### 1. Read Input

This step reads the `provisioner/src/inputs/neutron_inputs.toml` file.
Fields in this file should be sufficient to configure a `NeutronClient` and
the code IDs of the contracts to be deployed.

### 2. Instantiate Contracts

This step deploys the following smart contracts on the Neutron network:

- **Authorization Contract**: manages authorizations for the system
- **Processor Contract**: processes the messages decoded from the verified ZK proofs
- **CW20 Contract**: a token that is minted based on the verified proofs

Authorization contract with its respective processor is the core of all Valence Programs deployed on-chain. You can read more about their design in the [official docs](https://docs.valence.zone/authorizations_processors/_overview.html).

Unique thing about coprocessor-enabled Valence Programs is that authorization contracts are extended with an on-chain verification router.

This is done in `provisioner/src/steps/instantiate_contracts.rs`:

```rust
// Set the verification gateway address on the authorization contract
let set_verification_router_msg =
    valence_authorization_utils::msg::ExecuteMsg::PermissionedAction(
        valence_authorization_utils::msg::PermissionedMsg::SetVerificationRouter {
            address: VALENCE_NEUTRON_VERIFICATION_ROUTER.to_string(),
        },
    );

let set_verification_router_rx = neutron_client
    .execute_wasm(
        &authorization_address,
        set_verification_router_msg,
        vec![],
        None,
    )
    .await?;
```

The purpose of verification router is to route incoming proof verification requests to the correct verifier contract. This is done to enable support for multiple ZK VMs and proving systems, both of which get identified with a string-based `verification_route` parameter which gets set in authorizations setup step. You can read more about the ZK integration with Valence on-chain contracts in the [official documentation](https://docs.valence.zone/zk/03_onchain_integration.html).

After contract instantiation step is complete, addresses of the instantiated contracts are saved to `artifacts/instantiation_outputs.toml`,
to be consumed by the co-processor deployment step.

### 3. Deploy Co-processor App

This step performs the following actions:

1. **Embeds the CW20 address**: address of the deployed CW20 contract is embedded into the circuit source code
2. **Builds the co-processor app**: runs `nix run` to build the circuit and the controller
3. **Deploys to the co-processor**: compiled circuit and controller are deployed to the co-processor

The ID of the deployed co-processor application is saved to `artifacts/coprocessor_outputs.toml`,
to be consumed by the authorizations setup step.

### 4. Setup Authorizations

This step links the on-chain contracts with the co-processor application by creating a ZK authorization on the authorization contract.
This way, proofs submitted to the authorizations contract will be verified and decoded into the `CosmWasm` message
that will get pushed to the queue of the associated processor.

This flow is performed in `provisioner/src/steps/setup_authorizations.rs`, and involves the following steps.

**1. Getting the verifying key from the co-processor client**

Using the `coprocessor_app_id` field returned after deploying our ZK app, we can query the `CoprocessorClient` to get the respective verifying key:

```rust
let program_vk = cp_client.get_vk(&cfg.coprocessor_app_id).await?;

// deserialize the resulting bytes
let sp1_program_vk: SP1VerifyingKey = bincode::deserialize(&program_vk)?;
```

**2. Creating the ZK Authorization**

Using the verifying key from the previous step we can create the authorization.

Another thing to note here is about the `verification_route` field. In this template,
we are passing the following const value which uniquely identifies the proving system (`groth16`) and the ZK VM used to compile the respective circuit (`sp1/5.0.8`):

```rust
const VERIFICATION_ROUTE: &str = "0001/sp1/5.0.8/groth16";
```

With that, we can create the `ZkAuthorizationInfo` and submit it to the Neutron authorizations contract to bind our co-processor app to the on-chain program. We use the `ZK_MINT_CW20_LABEL` label to allow the coordinator to uniquely identify this authorization during its flow:

```rust
let zk_authorization = ZkAuthorizationInfo {
    label: ZK_MINT_CW20_LABEL.to_string(),
    mode: authorization_mode,
    registry: 0,
    vk: Binary::from(sp1_program_vk.bytes32().as_bytes()),
    validate_last_block_execution: false,
    verification_route: VERIFICATION_ROUTE.to_string(),
    metadata_hash: Binary::default(),
};

let create_zk_authorization = valence_authorization_utils::msg::ExecuteMsg::PermissionedAction(
    valence_authorization_utils::msg::PermissionedMsg::CreateZkAuthorizations {
        zk_authorizations: vec![zk_authorization],
    },
);

let create_zk_auth_rx = neutron_client
    .execute_wasm(&cfg.authorizations, create_zk_authorization, vec![], None)
    .await?;
```

### 5. Write Output

This final step generates the `artifacts/neutron_strategy_config.toml` file, which contains all the necessary configuration for the coordinator to run. This configuration includes:

- addresses of the deployed contracts
- ID of the co-processor application
- neutron domain details

## Artifacts

The provisioning process generates the following artifacts in the `artifacts/` directory:

- `instantiation_outputs.toml`: Contains the addresses of the contracts instantiated on-chain.
- `coprocessor_outputs.toml`: Contains the ID of the deployed co-processor application.
- `neutron_strategy_config.toml`: The final configuration file for the coordinator.
- `coprocessor/storage_proof/circuit.bin`: zk circuit binary
- `coprocessor/storage_proof/controller.bin`: zk controller associated with the circuit
