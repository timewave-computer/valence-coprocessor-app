# Technical Details

This docs entry provides a deeper dive into the technical implementation of the provisioning process and system architecture.

## On-chain Verification Router

Authorization contracts in co-processor-enabled Valence Programs are extended with an on-chain verification router. This is a key component that enables support for multiple ZK VMs and proving systems.

The verification router is set on the authorization contract as follows (see `provisioner/src/steps/instantiate_contracts.rs` for more details):

```rust
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

The router works by inspecting the `verification_route` parameter of incoming proof verification requests. Verification route is a string that uniquely identifies the ZK VM and proving system used to generate the proof. Based on this information, the router forwards the request to the correct verifier contract.

You can learn more about the ZK integration with Valence on-chain contracts in the [official documentation](https://docs.valence.zone/zk/03_onchain_integration.html).

## ZK Authorization Setup

The `provisioner/src/steps/setup_authorizations.rs` file contains the logic for linking specific on-chain contract execution instructions with the co-processor application. This is achieved by creating a ZK authorization on the authorization contract.

This process involves two main steps:

**1. Getting the verifying key from the co-processor client**

The first step is to retrieve the verifying key (VK) from the co-processor client. This is done by querying the client with the `coprocessor_app_id` that was returned after deploying the ZK app:

```rust
let program_vk = cp_client.get_vk(&cfg.coprocessor_app_id).await?;

// deserialize the resulting bytes
let sp1_program_vk: SP1VerifyingKey = bincode::deserialize(&program_vk)?;
```

**2. Creating the ZK Authorization**

Once the VK is obtained, it can be used to create a ZK authorization.

A crucial element here is the `verification_route` field. In this template, we use a constant value that uniquely identifies the proving system (`groth16`) and the ZK VM (`sp1/5.0.8`) used to compile the circuit:

```rust
const VERIFICATION_ROUTE: &str = "0001/sp1/5.0.8/groth16";
```

With the VK and the verification route, we can create a `ZkAuthorizationInfo` struct and submit it to the Neutron authorizations contract. This binds the co-processor app to the on-chain program. The `ZK_MINT_CW20_LABEL` is used to uniquely identify this authorization, allowing the coordinator to find it during its flow:

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

After the authorization is created, the coordinator can reference it with by `label: ZK_MINT_CW20_LABEL` during its runtime:

```rust
// execute the zk authorization. this will perform the verification
// and, if successful, push the msg to the processor
valence_coordinator_sdk::core::cw::post_zkp_on_chain(
    &self.neutron_client,
    &self.neutron_cfg.authorizations,
    ZK_MINT_CW20_LABEL,
    program_proof,
    program_inputs,
    domain_proof,
)
.await?;
```

See `coordinator/src/engine.rs` and [coordinator docs entry](docs/coordinator.md) for more details.

## Division of Logic: Circuit, Contract, and the Coordinator

To build a coprocessor application, it's helpful to understand how the logic is divided between the three main components: the ZK circuit, the on-chain authorization contract, and the coordinator.

**The ZK Controller (`apps/storage_proof/controller`):**

The controller's job is to gather and prepare the necessary data (witnesses) for the circuit.
It fetches information from external sources and formats it into the exact inputs the circuit requires.

Controller can be thought of as the co-processor "entry point" for the proof request calls made by the coordinator.

**The ZK Circuit (`apps/storage_proof/circuit`):**

The circuit contains the core computation you want to prove.

This logic is executed off-chain in a ZK co-processor.

Because proving is computationally expensive, you should try to minimize the number of variables and dynamic fields in the circuit.

The circuit takes witnesses from its associated controller and produces a cryptographic proof of a specific computational result.

This execution returns a `valence_authorization_utils::zk_authorization::ZkMessage` formatted exactly how the on-chain authorization contract expects it.

**The Authorization Contract (on-chain):**

Authorizations contract can be thought of as the on-chain entry-point.

Having stored the verifying key of the circuit in its state, this contract is able to gate the on-chain execution with a zk-verification step.

While authorization contracts can be configured with a wide array of permissions, in this context its primary job is to receive a proof from the coordinator, verify its validity against the stored key, and, if valid, push the decoded message into its associated processor queue.

Upon successful verification (and decoding) of the proof, this authorizes a corresponding on-chain action (e.g., minting tokens, updating some state) by pushing a message to the processor.

**The Coordinator (off-chain):**

The coordinator acts as the bridge between all parts of the system and orchestrates the entire process.

It triggers the proof generation by sending initial parameters to the controller, receives the final proof from the ZK co-processor, and submits that proof along with any additional parameters to the on-chain authorization contract for verification.
