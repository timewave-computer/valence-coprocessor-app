# Valence co-processor app template

This is a template for a Valence app.

# Example Application with Ethereum state proofs
This branch contains an example application that verifies Ethereum state proofs 
for both stored values in Smart Contracts and Account data (e.g. ETH Balance, Nonce) on mainnet.

The purpose of this branch is to provide not just a full Ethereum domain implementation, but also
a re-usable example template for developers looking to write Valence ZK apps that target Ethereum.


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
130292224d2c0678d0bafe642d5129d08b3dfd51dd1900a398f27f94a2a6bc77
```

>[!NOTE]
> When updating git dependencies, run `make clean` to ensure all lockfiles are properly updated.


### Prove

We instruct the coprocessor to generate a proof for the program. The default implementation of the program will accept an input value and pass it through the circuit. The circuit will then add `1` to the given value before returning the result as little-endian.

First we need to obtain a light client root and slot from the Co-processor. For testing we can run:

```sh
cd crates/excluded-utils
cargo test test_get_latest_helios_block -- --nocapture
```

Example output, note that it is recommended to obtain a more recent root when testing:
```sh
Validated block root: "aaeaf717f891f01e55287cd09f7291036d1c06a196e53456c5828bcbcb39250d"
Validated block height: 22580997
```

Now we can use this trusted block root and height to prove the program at that point in time:

```sh
cargo-valence --socket prover.timewave.computer:37281 \
  prove -j '{"addresses": ["0xf2B85C389A771035a9Bd147D4BF87987A7F9cf98", "0x07ae8551be970cb1cca11dd7a11f47ae82e70e67"], "keys": ["0xec8156718a8372b1db44bb411437d0870f3e3790d4a08526d024ce1b0b668f6b", ""], "height":22580997, "root":"aaeaf717f891f01e55287cd09f7291036d1c06a196e53456c5828bcbcb39250d"}' \
  -p /var/share/proof.bin \
  528065255d208f5766a8a92259950c103a3513800cdccd066de2d9003fbbfcde
```

To get the proof:
```sh
cargo-valence --socket prover.timewave.computer:37281 \
  storage \
  -p /var/share/proof.bin \
  528065255d208f5766a8a92259950c103a3513800cdccd066de2d9003fbbfcde | jq -r '.data' | base64 -d | jq
```

Example Proof:

```json
"proof": "2gFcRWJhZ25TODZOL0VqcUlJSHVMNzJmUnNVdEFBNmlrODU1QVRjbnluUVVXc1J1RHQ3RTl1WUxXUktCRFdHejA0NDN0Q09wUWU0RnRWR05hL0xSOVNQVEd1Q25Ka0gvT2g2TkZjK29wVEdJVmJBQU1TYWlPbWVyallEbVliSlBwdFg2RFBSNVEvbGRlQ3AxcGxZblkvbm9DSUhoZWlBcDd2dno1T1BXSnJJMlNGdm9LYmREVDZ4OE1iRjFpN1dkZU9XTVBrbW55UFg5eHRSWndyaHRkRTBxdzd0VkNJU1ZwcnJicjdNcUwxWnlIbVdZUk9JSkFRRkFPWS9OVnRjLzlWTjJWODliQTl5RjcreG5xMlNGZk1ITloveEVFNk5rN01SZXVNb00yMERhWWpjL09KckdLVTc4Z1dPd3BYRTFaMDRLVzJrQUxvTS9YQ2lqZ3g5YnBnaHd3SEh1NGM92fhBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFCN0luZHBkR2hrY21GM1gzSmxjWFZsYzNSeklqcGJYU3dpYzNSaGRHVmZjbTl2ZENJNld6RTNNQ3d5TXpRc01qUTNMREl6TERJME9Dd3hORFVzTWpRd0xETXdMRGcxTERRd0xERXlOQ3d5TURnc01UVTVMREV4TkN3eE5EVXNNeXd4TURrc01qZ3NOaXd4TmpFc01UVXdMREl5T1N3MU1pdzROaXd4T1Rjc01UTXdMREV6T1N3eU1ETXNNakF6TERVM0xETTNMREV6WFgwPQ=="
```


Note that in production we will either use the wasm module on the co-processor to obtain that trusted root, or verify the proof in the circuit.
The light client proof verification should ideally always happen in a trustless environment.

The command sends a proof request to the coprocessor's worker nodes. Once the proof is ready, it will be delivered to the program's entrypoint. The default implementation will then write the proof to the specified path within the program's virtual filesystem. Note that the virtual filesystem follows a FAT-16 structure, with file extensions limited to 3 characters and case-insensitive paths.

In conclusion, we can retrieve the proof from the virtual filesystem:

```sh
cargo run -- storage \
  -p /var/share/proof.bin \
  a73509f334b7b5bc8c5921a3f2b45cf5230bdc0f99ff72db2d33716a92bd687b \
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
