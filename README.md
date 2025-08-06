# Valence co-processor app template

This is a template for a Valence app.

It is configured for an application that leverages ZK storage proofs in order
to proof ERC20 contract storage (balance) entries. Generated proofs are then
used to execute zk-gated functions on the Valence Authorizations contract
which in turn triggers a cw20 contract mint on Neutron.

## Structure

### `./circuits`

The Valence Zero-Knowledge circuits directory.

Inside it you will find `storage_proof` circuit, controller, and core crates that
will perform erc20 storage proofs.

#### Circuit

It serves as a recipient for witness data (state proofs or data) from the associated controller. It carries out assertions based on business logic and outputs a `Vec<u8>`, which is subsequently forwarded to on-chain applications.

#### Controller

Compiled WASM binary that the coprocessor service runs in order to compute the circuit witnesses from given JSON arguments. It features an entrypoint that accommodates user requests; it also receives the result of a proof computation by the service.

#### Core

Core crate will contain any types, methods, or other helpers that may be relevant to both the circuit and controller.

### `./deploy`

Valence Program and Circuit deployment script.

### `./strategist`

Valence Coordinator that submits proof requests to the co-processor, and posts the proofs
to the Valence Authorizations contract on Neutron.

## Requirements

- [Docker](https://docs.docker.com/get-started/)
- [Rust](https://www.rust-lang.org/tools/install)
- (only for manual debugging): [Cargo Valence subcommand](https://github.com/timewave-computer/valence-coprocessor/tree/v0.3.12?tab=readme-ov-file#cli-helper)
- (Optional): [Valence co-processor instance](https://github.com/timewave-computer/valence-coprocessor/tree/v0.3.12?tab=readme-ov-file#local-execution)

## Instructions

There are two ways to interact with your co-processor application:

1. manual approach where you can leverage the `cargo-valence` package
  to deploy and test your circuit via CLI
2. automated approach where `deploy` crate binary will do the deployment
  for you. After that, running the `strategist` crate binary will submit
  the proof requests and verify their results on-chain.

### Automated Instructions

Outlined below are the automated deployment and runtime instructions that
will enable the e2e flow of erc20 -> cw20 ZK-based queries.

#### Mnemonic setup

Full flow will involve transaction execution on Neutron. To enable that,
a mnemonic with available ntrn token balances is needed.

To configure your mnemonic, run the following:

```bash
cp .example.env .env
```

Then open the created `.env` file and replace `todo` with your mnemonic seed phrase.

#### Run the deployment script

`deploy` crate `main.rs` contains an automated script which will perform the
following actions:

1. Fetch the mnemonic from `env`
2. Read the input parameters from `deploy/src/inputs/neutron_inputs.toml`
3. Instantiate the neutron program on-chain
4. Compile and deploy the co-processor application
5. Set up the on-chain authorizations
6. Produce the setup artifacts which will be used as runtime inputs

You can execute the sequence above by running:

```bash
RUST_LOG=info cargo run -p deploy
```

#### Execute the runtime script

After the deployment script produces valid output artifact in `artifacts/neutron_strategy_config.toml`,
you are ready to start the coordinator that will submit the proof requests and post them on-chain.

You can start the coordinator by running:

```bash
RUST_LOG=info cargo run -p strategist
```

### Manual instructions

This section contains the instructions for manual interaction and debugging of a
co-processor app.

#### Install Cargo Valence

A CLI helper is provided to facilitate the use of standard operations like deploying a circuit, proving statements, and retrieving state information.

To install:

```bash
cargo install \
  --git https://github.com/timewave-computer/valence-coprocessor.git \
  --tag v0.3.12 \
  --locked cargo-valence
```

`cargo-valence` supports local development workflows, as well as connecting to the public coprocessor service at http://prover.timewave.computer:37281/

We will be using the public co-processor service. If you prefer to operate your own instance, omit the `--socket` parameter.

#### Deploy

The circuit must be deployed with its controller. The controller is the responsible to compute the circuit witnesses, while the circuit is the responsible to assert the logical statements of the partial program.

```sh
cargo-valence --socket https://service.coprocessor.valence.zone \
  deploy circuit \
  --controller ./circuits/storage_proof/controller \
  --circuit storage-proof-circuit
```

This will output the application id associated with the controller. Let's bind this id to an environment variable, for convenience.

```sh
export CONTROLLER=$(cargo-valence --socket https://service.coprocessor.valence.zone \
  deploy circuit \
  --controller ./circuits/storage_proof/controller \
  --circuit storage-proof-circuit | jq -r '.controller')
```

#### Prove

This command will queue a proof request for this circuit into the co-processor, returning a promise of execution.

```sh
cargo-valence --socket https://service.coprocessor.valence.zone \
  prove -j '{"erc20":"0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48","eth_addr":"0x8d41bb082C6050893d1eC113A104cc4C087F2a2a","neutron_addr": "neutron1m6w8n0hluq7avn40hj0n6jnj8ejhykfrwfnnjh"}' \
  -p /var/share/proof.bin \
  $CONTROLLER
```

The argument `-j '{"erc20":"0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48","eth_addr":"0x8d41bb082C6050893d1eC113A104cc4C087F2a2a","neutron_addr": "neutron1m6w8n0hluq7avn40hj0n6jnj8ejhykfrwfnnjh"}'` will be forwarded to `circuits/storage_proof/controller/src/lib.rs:get_witnesses`. The output of this function will be then forwarded to the circuit for proving.

The command sends a proof request to the coprocessor's worker nodes. Once the proof is ready, it will be delivered to the program's entrypoint. The default implementation will then write the proof to the specified path within the program's virtual filesystem. Note that the virtual filesystem follows a FAT-16 structure, with file extensions limited to 3 characters and case-insensitive paths.

#### Storage

Once the proof is computed by the backend, it will be delivered to the virtual filesystem. We can visualize it via the `storage` command.

```sh
cargo-valence --socket https://service.coprocessor.valence.zone \
  storage \
  -p /var/share/proof.bin \
  $CONTROLLER | jq -r '.data' | base64 -d | jq
```

The output should be similar to the following structure:

```json
{
  "log": [],
  "payload": {
    "cmd": "store",
    "path": "/var/share/proof.bin"
  },
  "proof": "2gFccEZsTVdRU1FBbE94V0hHaTJMd0JsQW4ySXdISmF2L2JCaEgrL0E3SnN3VC9DNythQmdYTXBTUkd2MDM3MjBGMjhhUytuK3VyOXVhTng2QWVxZG5CTWRMTERGZ0ZSOFluNDJ3bWVkbnhpMS9iQ1Y0MkFvNGRUVTN6VjdMSVVDTW1sdENBV0J6cTRqNExCNU9vbHN6MWcxN2U4enRCVEJJd0FZTE8yRytWMlhadktZZFRHWVpINUthM3VtSjNWRVlyU1JYTmREUUxUeEEyWHAzSUVRU3FZRUl5aE8wa0JOanJIeUkwMUVRL1k3U3BISVZQdnFCbW5rOWJBajd3TVVwZENqVnBjQTRZMnZiTlZURkdad2ZSWnFVZDV3SEc1Y2pOY2VBTjg0QmVXQUIyUkJJRkRtU042WW9DaEJjZWN6V0hUSTR5T25mcVlQSkI1R0s5ZkRPU050dC9UV2c92gOMRmFYZm9SZjZrcnRWajFVQjNicm5ScXVtcStSMkQ5VEhGbzltTFphYUdaUjdJbkpsWjJsemRISjVJam93TENKaWJHOWphMTl1ZFcxaVpYSWlPakFzSW1SdmJXRnBiaUk2SW0xaGFXNGlMQ0poZFhSb2IzSnBlbUYwYVc5dVgyTnZiblJ5WVdOMElqcHVkV3hzTENKdFpYTnpZV2RsSWpwN0ltVnVjWFZsZFdWZmJYTm5jeUk2ZXlKcFpDSTZNQ3dpYlhObmN5STZXM3NpWTI5emJYZGhjMjFmWlhobFkzVjBaVjl0YzJjaU9uc2liWE5uSWpvaVpYbEtkR0ZYTlRCSmFuQTNTVzVLYkZreWJIZGhWMVoxWkVOSk5rbHROV3hrV0ZKNVlqSTBlR0pVV2pOUFJ6UjNZVWQ0TVdOVVpHaGtiVFF3VFVkb2NVMUhOREpoYlRWeFQwZFdjV0ZJYkhKYWJrb3pXbTAxZFdGdFoybE1RMHBvWWxjNU1XSnVVV2xQYVVrelRWUnJlRTFxWTNoSmJqRTVJbjE5WFN3aWMzVmljbTkxZEdsdVpTSTZleUpoZEc5dGFXTWlPbnNpWm5WdVkzUnBiMjV6SWpwYmV5SmtiMjFoYVc0aU9pSnRZV2x1SWl3aWJXVnpjMkZuWlY5a1pYUmhhV3h6SWpwN0ltMWxjM05oWjJWZmRIbHdaU0k2SW1OdmMyMTNZWE50WDJWNFpXTjFkR1ZmYlhObklpd2liV1Z6YzJGblpTSTZleUp1WVcxbElqb2liV2x1ZENJc0luQmhjbUZ0YzE5eVpYTjBjbWxqZEdsdmJuTWlPbTUxYkd4OWZTd2lZMjl1ZEhKaFkzUmZZV1JrY21WemN5STZleUo4YkdsaWNtRnllVjloWTJOdmRXNTBYMkZrWkhKOElqb2libVYxZEhKdmJqRTFjelJqZDNKemNYVTJibkF5TWpobU56VTVhMmcxWVhvM1pIVndjelozZVhsaGJteGtZV1JvWkRVeWVqbHNkSGwyY0d0eE1EQnplVEp3SW4xOVhTd2ljbVYwY25sZmJHOW5hV01pT201MWJHd3NJbVY0Y0dseVlYUnBiMjVmZEdsdFpTSTZiblZzYkgxOUxDSndjbWx2Y21sMGVTSTZJbTFsWkdsMWJTSXNJbVY0Y0dseVlYUnBiMjVmZEdsdFpTSTZiblZzYkgxOWZRPT0=",
  "success": true
}
```

#### Public inputs

We can also open the public inputs of the proof via the Valence helper:

```sh
cargo-valence --socket https://service.coprocessor.valence.zone \
  proof-inputs \
  -p /var/share/proof.bin \
  $CONTROLLER | jq -r '.inputs' | base64 -d | hexdump -C
```

Note: The first 32 bytes of the public inputs are reserved for the co-processor root.
