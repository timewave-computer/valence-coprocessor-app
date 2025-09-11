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

The addresses of the instantiated contracts are saved to `artifacts/instantiation_outputs.toml`,
to be consumed by the co-processor deployment step.

For a more in-depth explanation of the on-chain components, see the [technical details](./technical_details.md) document.

### 3. Deploy Co-processor App

This step performs the following actions:

1. **Embeds the CW20 address**: address of the deployed CW20 contract is embedded into the circuit source code
2. **Builds the co-processor app**: runs `./build-circuits.sh` to build the circuit and the controller
3. **Deploys to the co-processor**: compiled circuit and controller are deployed to the co-processor

The ID of the deployed co-processor application is saved to `artifacts/coprocessor_outputs.toml`,
to be consumed by the authorizations setup step.

### 4. Setup Authorizations

This step links the on-chain contracts with the co-processor application by creating a ZK authorization on the authorization contract.
This way, proofs submitted to the authorizations contract will be verified and decoded into the `CosmWasm` message
that will get pushed to the queue of the associated processor.

For a more in-depth explanation of the authorization setup, see the [technical details](./technical_details.md) document.

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
