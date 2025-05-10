# Valence Coprocessor App Workflow

This document explains the workflow for building, deploying, and using the Valence coprocessor application.

## Workflow Diagram

```mermaid
graph TD
    A[Start] --> B[Build WASM]
    B --> C[Deploy to Service]
    C --> D[Generate Proof]
    D --> E[Verify Proof]
    
    subgraph "Scripts"
        F[build-wasm.sh] -.- B
        G[build-wasm-wrapper.sh<br/>friendly wrapper] -.- B
        H[deploy-to-service.sh] -.- C
        I[full-pipeline.sh] -.- J[Runs all steps]
    end
    
    J --> B
    J --> C
    J --> D
    J --> E
    
    subgraph "Tools Used"
        K[Nix WASM Shell] -.- B
        L[curl API Calls] -.- C
        L -.- D
        L -.- E
    end
    
    subgraph "Alternative Legacy CLI"
        M[cargo run -- deploy] -.->|May fail on<br/>some platforms| N[Build and Deploy]
        O[cargo run -- prove] -.-> P[Generate Proof]
        Q[cargo run -- verify] -.-> R[Verify Proof]
    end
```

## Script Functionality

| Script | Purpose | Description |
|--------|---------|-------------|
| `build-wasm.sh` | Build WASM | Builds the WASM binary using Nix wasm-shell |
| `build-wasm-wrapper.sh` | User-friendly wrapper | A more user-friendly wrapper around build-wasm.sh |
| `deploy-to-service.sh` | Deploy to service | Deploys the WASM binary to the co-processor service |
| `full-pipeline.sh` | Complete workflow | Runs the entire pipeline from build to verification |
| `install-cargo-prove.sh` | Install cargo-prove | Utility to install the cargo-prove binary |

## Recommended Usage

For most users, especially on macOS with Apple Silicon, we recommend using our shell scripts rather than the legacy Cargo CLI:

```bash
# Run the entire pipeline with one command
./scripts/full-pipeline.sh

# Or run each step individually
./scripts/build-wasm.sh
./scripts/deploy-to-service.sh
# Then use curl for proof generation and verification
``` 