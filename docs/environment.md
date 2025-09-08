# Environment Setup

This document provides a detailed guide on how to set up your environment for developing and
running the Valence co-processor app.

## On-Chain Interactions

To interact with the Neutron domain, you need to provide a mnemonic seed phrase for an account which holds `untrn` tokens.
This will be needed in order to cover the gas fees during both provisioning, and runtime coordination stages.

First, create your own environment file from the provided example by running:

```sh
cp .example.env .env
```

After that, open the newly created `.env` file and replace the placeholder `todo` with your mnemonic seed phrase.

```sh
MNEMONIC="your mnemonic seed phrase here"
```

This mnemonic will be used by the provisioner and the coordinator to sign transactions.

## Local Development Environment

We use [Nix](https://nixos.org/) to provide a reproducible development environment.
In the future, Docker-based builds will be added for convenience.

The `flake.nix` file at the root of the project defines all the dependencies and tools required to build and run the project.

### Nix Installation

If you don't have Nix installed, you can install it using one of the following methods:

- **Determinate Systems Installer**: [https://docs.determinate.systems/getting-started/](https://docs.determinate.systems/getting-started/)
- **Nix Package Manager**: [https://nixos.org/download/](https://nixos.org/download/)

### Development Shell

Once Nix is installed, you can enter the development shell by running the following command at the root of the project:

```sh
nix develop
```

This will download all of the required dependencies and drop you into a shell with those dependencies available.

**Note:** Explicitly entering the development shell is not a necessity for developing co-processor apps.
It is meant to serve the situations where more fine-grained control is needed for debugging or other purposes.

### Building the Code

To manually build all the circuits and controllers, run the following command:

```sh
nix run
```

This command executes the `build-circuits` script defined in `flake.nix`, which does the following:

1. reads the `valence.toml` file in the project root
2. builds each circuit and controller
3. places the resulting artifacts in the `artifacts/coprocessor` directory
