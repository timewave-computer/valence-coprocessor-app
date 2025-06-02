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
cargo-valence --socket prover.timewave.computer:37281   deploy circuit   --controller ./crates/controller   --circuit valence-coprocessor-app-circuit
```

Upon successful deployment, you should observe the generated ID:

```
96d83b2300d83ecc413687e866338a5cb3522a1007460e7c90121c94a5ecb5e6
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
Validated block root: "ad242daa9f4e7d20187f9122d32a7aa49a3d7bf46ff306b64961e7d21fdd90ee"
Validated block height: 22616191
```

Now we can use this trusted block root and height to prove the program at that point in time:

```sh
cargo-valence --socket prover.timewave.computer:37281 \
  prove -j '{}' \
  -p /var/share/proof.bin \
  96d83b2300d83ecc413687e866338a5cb3522a1007460e7c90121c94a5ecb5e6
```

To get the proof:
```sh
cargo-valence --socket prover.timewave.computer:37281 \
  storage \
  -p /var/share/proof.bin \
  96d83b2300d83ecc413687e866338a5cb3522a1007460e7c90121c94a5ecb5e6 | jq -r '.data' | base64 -d | jq
```

Example Proof:

```json
"proof": "2gFcRWJhZ25TSDBsazFZUjY3ZTlrSzY5Y1NnbXpkTnJkL0tmMllRWDlIdjBZcjc4R0UwSzRnVWpocUsyajJPNzlEOGdMcTdRRjh1N29sOUkyUEFPcFhJdmYwZzcyNEl1ajBmcVpic2NCcmFXYWZFa1VzdW11VVhBTEFFRm5lMTJJVXBzbmN1dVM1aWZMM1phSG43YU9DbUlUVmxKdHhUREg2b2pTRlRnTFJONGkwMDQyUFVEZHl6b0NmT1pZVnRQalpnYm1vTjJPeTh6K1BXTnBiaDl6Y0ozOG9XLytvSUQ2UXN0WlRZTlZRSDQ4Z1lIU28vTDE4aEpCVjR0cW9jRlJ4MlFOUHVzZ3BHcnIwZlJuampPb214NnZERFlKN09zVTFObXMwUDQ3U3k4K3ljaGF1SkpoY2g2REJ6NnVpOUVoQ3IwSFU2N2svNFJndnQvV1dJRTJKSmYxUVplTVE92gQsQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQjdJbmRwZEdoa2NtRjNYM0psY1hWbGMzUnpJanBiZXlKcFpDSTZNQ3dpYjNkdVpYSWlPaUl3ZUdRNVFUSXpZalU0WlRZNE5FSTVPRFZHTmpZeFEyVTNNREExUVVFNFJURXdOak13TVRVd1l6RWlMQ0p5WldSbGJYQjBhVzl1WDNKaGRHVWlPbHN4TURBd01EQXdNREJkTENKemFHRnlaWE5mWVcxdmRXNTBJanBiTVRBd1hTd2ljbVZqWldsMlpYSWlPaUpHNzcrOVJGeDFNREF3TWUrL3ZlKy92ZSsvdlhnbmRlKy92ZSsvdlR4dzc3KzllVngxTURBeFpGeDFNREF4Wm4vdnY3M3Z2NzFjZFRBd01URmQ3Nys5NzcrOVZPKy92ZSsvdmUrL3ZTbnZ2NzNhbXUrL3ZWSXg3Nys5NzcrOVFPKy92ZSsvdmUrL3ZTL3Z2NzFiNzcrOVcrKy92ZFNQYisrL3ZUM3Z2NzN2djcxY2RUQXdNRFR2djczTXFHa2hYSFV3TURBeTc3KzlaM1R2djcxY2REMXhhZSsvdlc1bGRYUnliMjR4Tkcxc2NHUTBPR3MxZG10bGMyVjBOSGczWmpjNGJYbDZSdSsvdlVSY2RUQXdNREh2djczdnY3M3Z2NzE0SjNYdnY3M3Z2NzA4Y08rL3ZYbGNkVEF3TVdSY2RUQXdNV1ovNzcrOTc3KzlYSFV3TURFeFhlKy92ZSsvdlZUdnY3M3Z2NzN2djcwcDc3KzkycHJ2djcxU01lKy92ZSsvdlVEdnY3M3Z2NzN2djcwdjc3KzlXKysvdlZ2dnY3M1VqMi92djcwOTc3Kzk3Nys5WEhVd01EQTA3Nys5ektocElWeDFNREF3TXUrL3ZXZDA3Nys5WEhROWNXbnZ2NzB6YlRRM2FtTmhlRE41YzJwcmNGeDFNREF3TUZ4MU1EQXdNRngxTURBd01GeDFNREF3TUZ4MU1EQXdNRngxTURBd01GeDFNREF3TUZ4MU1EQXdNRngxTURBd01GeDFNREF3TUZ4MU1EQXdNRngxTURBd01GeDFNREF3TUZ4MU1EQXdNRngxTURBd01GeDFNREF3TUZ4MU1EQXdNRngxTURBd01DSjlYU3dpYzNSaGRHVmZjbTl2ZENJNld6WTVMREV3TWl3eU1EQXNNemdzTWpJM0xERXdNeXcxTERFek1Dd3hOVGtzTVRNMUxERTFNQ3d4TVRJc01UZzNMREV5TXl3eE1EY3NNVFUzTERFeU15d3lNRGdzTVRjM0xESXpOeXd4TURBc01UYzVMREUyTkN3eE1ETXNNalF3TERFNE9Td3hNemNzTVRFNExERXpMREU0T1N3NExERTVPRjE5"
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
