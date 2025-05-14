# Valence co-processor app template

This is a template for a Valence app.

### Instructions

First, start the co-processor. This can take a couple of minutes to compile. You can check the API interface via http://127.0.0.1:37281/

```sh
make coprocessor
```

You can check the status of the application via:

```sh
curl http://127.0.0.1:37281/api/stats
```

### Build

These commands will build (not deploy) the applications:

```sh
make circuit
make domain
make program
```

Or, to build everything

```sh
make
```

### Deploy

First, we deploy the program. It will compile the crates under `./crates/program` and `./crates/circuit`, and submit them to the co-processor. Finally, the program ID will be returned.

```sh
cargo run -- deploy program
```

You should see the compilation process, and then the generated ID

```
dc8b02eced17353e42ff11a0fc4aa2b982435735b9b2f24da79a8bcd69792ce6
```

Then, we request the co-processor to compute a proof for the program. The default implementation of the program will take a value, and submit it to the circuit. The circuit will then add `1`, and return the value as little-endian.

```sh
cargo run -- prove dc8b02eced17353e42ff11a0fc4aa2b982435735b9b2f24da79a8bcd69792ce6 '{"value": 42}' /var/share/proof.bin
```

The command will submit a proof request to the workers of the co-processor. Once the proof is ready, it will be submitted to the entrypoint of the program. The default implementation will write it to the provided path of the virtual filesystem of the program. Note: the virtual filesystem is a FAT-16, so extensions can have up to 3 characters, and paths are case insensitive.

Finally, we can fetch the proof from the virtual filesystem:

```sh
cargo run -- storage dc8b02eced17353e42ff11a0fc4aa2b982435735b9b2f24da79a8bcd69792ce6 /var/share/proof.bin | base64 -d
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
- Makefile
