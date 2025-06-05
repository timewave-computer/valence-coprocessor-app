# Valence Co-processor App Template

This repository serves as a template for building Valence applications with Ethereum state proof verification capabilities.

## Overview

This example application demonstrates how to verify Ethereum state proofs for:
- Stored values in Smart Contracts
- Account data (ETH Balance, Nonce) on mainnet

The template provides a complete Ethereum domain implementation and serves as a reusable foundation for developers building Valence ZK applications that interact with Ethereum.

## Getting Started

### Running the Co-processor

Start the co-processor service (compilation may take a few minutes):

```sh
cargo run -- coprocessor
```

Once running, you can access the API interface at http://127.0.0.1:37281/

Check the application status:

```sh
curl http://127.0.0.1:37281/api/stats
```

### Deployment

Deploy your application by building and submitting the program and circuit components to the coprocessor:

```sh
cargo-valence --socket prover.timewave.computer:37281 deploy circuit --controller ./crates/controller --circuit valence-coprocessor-app-circuit
```

Upon successful deployment, you'll receive a Program ID similar to:
```
98fc5fcaf907e257e65f9c9a3c1b2728b2adde88c9338ff35bea0fdd10daf16d
```

> [!NOTE]
> When updating git dependencies, run `make clean` to ensure all lockfiles are properly updated.

### Generating Proofs

Submit a proof request with JSON arguments:

```sh
cargo-valence --socket prover.timewave.computer:37281 \
  prove -j '{"event_idx":0}' \
  -p /var/share/proof.bin \
  98fc5fcaf907e257e65f9c9a3c1b2728b2adde88c9338ff35bea0fdd10daf16d
```

Retrieve the generated proof:

```sh
cargo-valence --socket prover.timewave.computer:37281 \
  storage \
  -p /var/share/proof.bin \
  98fc5fcaf907e257e65f9c9a3c1b2728b2adde88c9338ff35bea0fdd10daf16d | jq -r '.data' | base64 -d | jq
```

#### How Proof Generation Works

1. The command sends a proof request to the coprocessor's worker nodes
2. Once the proof is ready, it's delivered to the program's entrypoint
3. The default implementation writes the proof to the specified path in the program's virtual filesystem
4. The virtual filesystem uses FAT-16 structure with 3-character file extensions and case-insensitive paths

You can also retrieve proofs directly from the virtual filesystem:

```sh
cargo run -- storage \
  -p /var/share/proof.bin \
  a73509f334b7b5bc8c5921a3f2b45cf5230bdc0f99ff72db2d33716a92bd687b \
  | base64 -d
```

### Example Proof Output

The deployed proof will contain structured data like this:

```json
{
  "args": {
    "event_idx": 0
  },
  "log": [
    "received a proof request with arguments {\"value\":42}"
  ],
  "payload": {
    "cmd": "store",
    "path": "/var/share/proof.bin"
  },
  "proof": "2gFcRWJhZ25RT2JFYWtzbGRHWjR5YncxVjVTMXZ5RCtPdjlmRVZJVDJRdndXcll5VG9lRE1rUllUUWJUSk9LUnZHY1NLdEhKcUJHYS9OTmVmRW0yVWlDU1QzajVSb3BhZjlEcXRQZndOWCtPNmRlN0VHaUVVTlhKSmdtMGdVYWV4QStXQ1RiRkI1WlB2ZHRWa0wrWERnUm5BWjhTZmRRRDlnajJCT2Y4REFNTlo4UXhIcTFLbGNjTXlnNWZQMW05ckdCZUJNWmd2SW1vN1FKclE2MHFDMGFMTUU4dUNJQnZoU1d3TCs2RVUvU1gzT3BZd2xvZ1ppdC9wK1RRRUQvdkt1UUJZN2tqaVBJY3dWaVBsalJoOEcxUkYrRE9EbkhDWlB3M0ZnRDhyS2ZXUEJiTTh5NkI1YkhJdlA3MU5SUmgxcjA0U0w0dHU2T0pSNEZSNlZLTE5KcVBsZHlLOHM92gIAYmpRTG5QK3plcGljcFVUbXUzZ0tMSGlRSFQrek56aDJoUkdqQmhldm9CMTdJbmRwZEdoa2NtRjNYM0psY1hWbGMzUnpJanBiZXlKcFpDSTZNU3dpYjNkdVpYSWlPaUl3ZUdRNVFUSXpZalU0WlRZNE5FSTVPRFZHTmpZeFEyVTNNREExUVVFNFJURXdOak13TVRVd1l6RWlMQ0p5WldSbGJYQjBhVzl1WDNKaGRHVWlPbHN4TURBd01EQXdNREJkTENKemFHRnlaWE5mWVcxdmRXNTBJanBiTlRCZExDSnlaV05sYVhabGNpSTZJbTVsZFhSeWIyNHhiVEpsYldNNU0yMDVaM0IzWjNOeWMyWXlkbmxzZGpsNGRtZHhhRFkxTkRZek1IWjNaR1p5YUhKcmJYSTFjMnhzZVRVemMzQm5PRFYzZGlKOVhTd2ljM1JoZEdWZmNtOXZkQ0k2V3pRMUxERTNOU3d4TURnc01UY3dMREV5T1N3eU1qQXNNVEk0TERFME9Td3hNRGtzTVRFeUxESTBOeXd4TWpVc016UXNPRGdzTVRFMExETXdMREU1TkN3eE9UQXNNVE16TERFek1Dd3lORFFzTVRnMkxEZzRMREkwTWl3M01Dd3lPQ3c0Tnl3eU16WXNNalEwTERJeU5Dd3lOVEFzTnpKZGZRPT0=",
  "success": true
}
```

> [!NOTE]
> In production environments, use either the WASM module on the co-processor to obtain trusted roots, or verify proofs within the circuit. Light client proof verification should always occur in a trustless environment.

## Project Structure

### `./crates/circuit`
Contains the Valence Zero-Knowledge circuit implementation. This component:
- Receives witness data (state proofs or data) from the associated program
- Performs assertions based on business logic
- Outputs a `Vec<u8>` that is forwarded to on-chain applications

### `./crates/domain` 
Defines the domain specification. This crate:
- Generates state proofs from JSON arguments
- Validates blocks integrated within the coprocessor

### `./crates/program`
Houses the main Valence program that:
- Computes circuit witnesses from provided JSON arguments
- Features an entrypoint that handles user requests
- Receives and processes proof computation results from the service

## Development Tools

### Light Client Testing

To test light client functionality and obtain roots and slots from the co-processor:

```sh
cd crates/excluded-utils
cargo test test_get_latest_helios_block -- --nocapture
```

Example output (use a more recent root for actual testing):
```sh
Validated block root: "ad242daa9f4e7d20187f9122d32a7aa49a3d7bf46ff306b64961e7d21fdd90ee"
Validated block height: 22616191
```

## Requirements

- Docker
- Rust
