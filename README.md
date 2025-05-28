# Valence co-processor app template

This is a template for a Valence app.

## Requirements

- [Docker](https://docs.docker.com/get-started/)
- [Rust](https://www.rust-lang.org/tools/install)
- [Cargo Valence subcommand](https://github.com/timewave-computer/valence-coprocessor/tree/v0.1.13?tab=readme-ov-file#cli-helper)
- (Optional): [Valence co-processor instance](https://github.com/timewave-computer/valence-coprocessor/tree/v0.1.13?tab=readme-ov-file#local-execution)

## Instructions


We will be using the public co-processor service. If you prefer to operate your own instance, omit the `--socket` parameter.

#### Deploy

The circuit must be deployed with its controller. The controller is the responsible to compute the circuit witnesses, while the circuit is the responsible to assert the logical statements of the partial program.

```sh
cargo-valence --socket prover.timewave.computer:37281 \
  deploy circuit \
  --controller ./crates/controller \
  --circuit valence-coprocessor-app-circuit
```

This will output the circuit id associated with the controller. Let's set it to an environment variable, for convenience.

```sh
export CONTROLLER=$(cargo-valence --socket prover.timewave.computer:37281 \
  deploy circuit \
  --controller ./crates/controller \
  --circuit valence-coprocessor-app-circuit | jq -r '.controller')
```

#### Prove

This command will queue a proof request for this circuit into the co-processor, returning a promise of execution.

```sh
cargo-valence --socket prover.timewave.computer:37281 \
  prove -j '{"value": 42}' \
  -p /var/share/proof.bin \
  $CONTROLLER
```

The argument `-j '{"value": 42}'` will be forwarded to `./crates/controller/src/lib.rs:get_witnesses`. The output of this function will be then forwarded to the circuit for proving.

The command sends a proof request to the coprocessor's worker nodes. Once the proof is ready, it will be delivered to the program's entrypoint. The default implementation will then write the proof to the specified path within the program's virtual filesystem. Note that the virtual filesystem follows a FAT-16 structure, with file extensions limited to 3 characters and case-insensitive paths.

#### Storage

Once the proof is computed by the backend, it will be delivered to the virtual filesystem. We can visualize it via the `storage` command.

```sh
cargo-valence --socket prover.timewave.computer:37281 \
  storage \
  -p /var/share/proof.bin \
  $CONTROLLER | jq -r '.data' | base64 -d | jq
```

The output should be similar to the following structure:

```json
{
  "args": {
    "value": 42
  },
  "log": [
    "received a proof request with arguments {\"value\":42}"
  ],
  "payload": {
    "cmd": "store",
    "path": "/var/share/proof.bin"
  },
  "proof": "2gFcRWJhZ25SQTZYbTBKRWpnSUxyYzl6bEVxT3l4dEJPdHgyU2R0Z3ZqS2pTd2QvQU5MREJYcElZUytLOUo2VXlwK25tMzNCTU8vQkQwOStDZkVZNUhYZytRNDJwRU9SRkdqeVZVUFBoaGU3bXBBY1JYM0lVcnJDRm45VG92MjFzSFg5dFdidmdpeXA4cE43QU9HeHQ2VWFaRHpXVTdCdDZsRzBwSGd6Tm9lR085WkRzU2NER3Z1cnJxWXpJeGVQNGtVRFBsMFZKaWNhTDlhQWRJbXlxb2d5VFFtNWx3Vm00L25qVHBoUDhFNEZMQ3pOWDlnQzduK0Z0SVRiaHFlVndVdU11R0dUQ0xBQjEwV3B6MTluRzZ6L2o4M0VHTnJuNTk2Qkh0RnNEbkFBNnVFZklYREQ4Z3lXTDFuN0RIRVVDek1JKzhCYjJTMS9rOWgzejBmOGxjWEFCTUUzS1E92ThBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBckFBQUFBQUFBQUE9PQ==",
  "success": true
}
```

#### Public inputs

We can also open the public inputs of the proof via the Valence helper:

```sh
cargo-valence --socket 104.171.203.127:37281 \
  proof-inputs \
  -p /var/share/proof.bin \
  $CONTROLLER | jq -r '.inputs' | base64 -d | hexdump -C
```

Note: The first 32 bytes of the public inputs are reserved for the co-processor root.

### Structure

#### `./crates/circuit`

The Valence Zero-Knowledge circuit. It serves as a recipient for witness data (state proofs or data) from the associated controller. It carries out assertions based on business logic and outputs a `Vec<u8>`, which is subsequently forwarded to on-chain applications.

#### `./crates/domain`

A Definition for a domain. This crate will produce state proofs derived from JSON arguments, and validate blocks incorporated within the coprocessor.

#### `./crates/controller`

The Valence controller. It will be used to compute the circuit witnesses from given JSON arguments. It features an entrypoint that accommodates user requests; it also receives the result of a proof computation by the service.
