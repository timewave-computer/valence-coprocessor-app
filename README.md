TODO: rewrite this according to the latest format

# Valence co-processor app template

This is a template for a Valence co-processor app.

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
- [Valence domain clients co-processor utils](https://github.com/timewave-computer/valence-domain-clients?tab=readme-ov-file#cli)

## Deploy

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

The build process expects the following toolchains to be installed:

- [wasm32-unknown-unknown](https://doc.rust-lang.org/rustc/platform-support/wasm32-unknown-unknown.html)
- [SP1](https://docs.succinct.xyz/docs/sp1/getting-started/install)

```shell
valence-coprocessor build
```

#### Nix

Alternatively to manually installing build toolchains, we can use Nix. Notably, we endorse and facilitate Nix builds in our system. Currently, our platform exclusively supports WebAssembly (WASM) builds using Nix. As such, having SP1 installed is a necessity; we however plan to add SP1 Nix support.

#### Deploy

The circuit must be deployed with its controller. The controller is the responsible to compute the circuit witnesses, while the circuit is the responsible to assert the logical statements of the partial program.

```sh
cargo-valence --socket https://service.coprocessor.valence.zone \
  deploy circuit \
  --controller ./circuits/storage_proof/controller \
  --circuit storage-proof-circuit
```shell
nix develop --command valence-coprocessor build --only-controller
```

#### Publish

Executing this command initiates the build process, which constructs all artifacts listed within the `valence.toml` configuration file. The resulting folder for these artifacts is designated in `valence.artifacts`, and it will house the runtime binary data along with the circuit files tailored for the ZkVM. These constructed elements will be delivered to the co-processor service upon deployment.

```shell
valence-coprocessor deploy
```

This command deploys all circuits defined in the 'valence.toml' file. The circuit IDs are subsequently listed.

```json
[
  {
    "id": "308ce5062e87628f50a0b0a3f4e8f8b66640a96c0983e35a44652e40b7809278",
    "name": "app"
  }
]
```

## Debug

The Valence Co-Processor Toolkit enables the inspection of pre-computation methods for witness sets in the context of circuit proofs.

```shell
valence-coprocessor witnesses \
  --circuit 308ce5062e87628f50a0b0a3f4e8f8b66640a96c0983e35a44652e40b7809278 \
  --args '{"value": 42}'
```

The output is the controller logs, and the computed witnesses:

```json
{
  "log": [
    "received a proof request with arguments {\"value\":42}"
  ],
  "witnesses": {
    "proofs": [],
    "root": [222,236,233,6,92,167,87,33,245,88,93,74,74,188,123,188,206,93,80,59,251,190,92,36,98,1,50,145,105,110,104,118],
    "witnesses": [
      {
        "Data": [42,0,0,0,0,0,0,0]
      }
    ]
  }
}
```

## Prove

The Valence Co-Processor Toolkit enables proving a circuit with a given set of arguments.

```shell
valence-coprocessor prove \
  --circuit 308ce5062e87628f50a0b0a3f4e8f8b66640a96c0983e35a44652e40b7809278 \
  --args '{"value": 42}'
```

The generated artifact is a cryptographically secure proof document suitable for submission to an on-chain verifier.

```json
{
  "domain": {
    "inputs": "3uzpBlynVyH1WF1KSrx7vM5dUDv7vlwkYgEykWluaHY=",
    "proof": "pFlMWRfN3PMSmiEcGYzUgdNxnHVmuwOln72TOouvU0gsIoZeERCKbfy8bA4B27F8o2XIxFP
QOP+KikbfluhH8BnEyT4sLNAY6rAw2Wc22UcIpsQkabZbdDhkxeFdtR5goZMVVRkvDSVKoklJPPYjvRMjo5/8
+NN/Bqk8dCV0tttnjTMVExR9d3Y+QhnwfXDitLsmjebAFcL0+b6+rgjdwQXuF+8iEnx7i7F37SExZ8JLLKrqo
CqSQ9PGWIO9I2mOG0YtDxklwPeCEfgtDf+rZ9JyVsAzxb3o/RxVDAq4+/NdGFNyKNjkhHxe/lhItek6Kdlohh
uyNQZ0izQcXJ2IaZyhopw="
  },
  "program": {
    "inputs": "3uzpBlynVyH1WF1KSrx7vM5dUDv7vlwkYgEykWluaHYrAAAAAAAAAA==",
    "proof": "pFlMWSUQMaQnsht+AgKaa+5QXcP+01iQTwK6mrVC7gaZ67VqEM0F5i0lWKbTiXbJU+oyqPY
cLph78lZ5GY2Ulu3110QSxdy6C4AHGKoUxiah7UXxk6mA8WCH2Fl1OuiD47QSGCQeieUsOc4EFOoqRJs2enYH
UZWe9Z30HP6HwQqCQb8VGz2nCKHEXVhvMqPFxNqBpB9z9gx1iuxYSZnTO8zy9CAJRIhxZv3JGCN72jzG4etAt
0u658nRwCcAyk0REs0n2iWf3P8+I1Fcj7JQdxReR0p5W5kpNxHKeifPZpuiysb2BEH6hcfmQqZErWKFlz6z+x
i7sAEo7xO+WieBxOVbg0E="
  }
}
```
