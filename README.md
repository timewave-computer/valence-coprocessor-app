# Valence co-processor app template

This is a template for a Valence co-processor app.

## Overview

This repository showcases a minimal, co-processor enabled Valence Program.

After system is provisioned, a coordinator will generate ZK storage
proofs of a given Ethereum Externally-owned Account (EOA) address for a
selected ERC20 contract address on Ethereum mainnet. The balance
observed during the proving process will be used to mint the equivalent
amount of CW20 tokens on Neutron mainnet for a given address.

> Note: currently the flow is uni-directional in that the proofs are only
> generated for EVM state and verified on Neutron.
>
> We are planning to extend this example with Tendermint proofs being
> verified on EVM in the near future.

### Project filetree

```md
├── apps            # TODO: describe
│   └── ...
├── artifacts       # TODO: describe
│   └── ...
├── Cargo.nix       # TODO: describe
├── Cargo.toml      # TODO: describe
├── common          # TODO: describe
│   └── ...
├── coordinator     # TODO: describe
│   └── ...
├── flake.nix       # TODO: describe
├── provisioner     # TODO: describe
│   └── ...
└── valence.toml    # TODO: describe
```

## Provisioning

Multiple steps are involved in getting the system to the point where
the on-and-off-chain components are tied together and ready to start
processing the requests as expected.

System setup is managed by the **provisioner**.

End to end provisioning can be performed with the following command:

```sh
RUST_LOG=info cargo run --bin provisioner
```

To see more details about Valence co-processor app provisioning and
what steps are involved in the e2e provisioning (above), see the
[setup docs](docs/setup.md).

## Runtime

After the system is fully bootstrapped, it is ready to enter the
runtime phase.

Program runtime is managed by the **coordinator** and can be started
by running the following command:

```sh
RUST_LOG=info cargo run --bin coordinator
```

To see more details about the coordinator runtime and overview, see
the [coordinator docs](docs/coordinator.md).
