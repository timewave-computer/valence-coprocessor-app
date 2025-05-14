# Valence co-processor app template

This is a template for a Valence app.

### Instructions

First, start the co-processor. This can take a couple of minutes to compile. You can check the API interface via http://127.0.0.1:37281/

```sh
cargo run -- coprocessor
```

You can check the status of the application via:

```sh
curl http://127.0.0.1:37281/api/stats
```

### Deploy

Initially, we execute the program, which will build the crates located in `./crates/program` and `./crates/circuit`, followed by submitting these built components to the coprocessor. Ultimately, the assigned Program ID will be returned for your reference.

```sh
cargo run -- deploy program
```

Upon successful deployment, you should observe the generated ID:

```
dc8b02eced17353e42ff11a0fc4aa2b982435735b9b2f24da79a8bcd69792ce6
```

### Prove

We instruct the coprocessor to generate a proof for the program. The default implementation of the program will accept an input value and pass it through the circuit. The circuit will then add `1` to the given value before returning the result as little-endian.

```sh
cargo run -- prove \
  -j '{"value": 42}' \
  -p /var/share/proof.bin \
  dc8b02eced17353e42ff11a0fc4aa2b982435735b9b2f24da79a8bcd69792ce6
```

The command sends a proof request to the coprocessor's worker nodes. Once the proof is ready, it will be delivered to the program's entrypoint. The default implementation will then write the proof to the specified path within the program's virtual filesystem. Note that the virtual filesystem follows a FAT-16 structure, with file extensions limited to 3 characters and case-insensitive paths.

In conclusion, we can retrieve the proof from the virtual filesystem:

```sh
cargo run -- storage \
  -p /var/share/proof.bin \
  dc8b02eced17353e42ff11a0fc4aa2b982435735b9b2f24da79a8bcd69792ce6 \
  | base64 -d
```

You should see the proof that was deployed to the program storage via the entrypoint function:

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
  "proof": "AAAAAAAAAAAAAAAAKAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAACsAAAAAAAAACwAAAAAAAAB2NC4wLjAtcmMuMwA=",
  "success": true
}
```

### Structure

#### `./crates/circuit`

The Valence Zero-Knowledge circuit. It serves as a recipient for witness data (state proofs or data) from the associated program. It carries out assertions based on business logic and outputs a `Vec<u8>`, which is subsequently forwarded to on-chain applications.

#### `./crates/domain`

A Definition for a domain. This crate will produce state proofs derived from JSON arguments, and validate blocks incorporated within the coprocessor.

#### `./crates/program`

The Valence program. It will be used to compute the circuit witnesses from given JSON arguments. It features an entrypoint that accommodates user requests; it also receives the result of a proof computation by the service.

### Requirements

- Docker
- Rust
