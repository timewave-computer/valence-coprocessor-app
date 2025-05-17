# Nix Development Guide for Valence Coprocessor App

## 1. Introduction

This guide explains how to use the Nix-based development environment for the Valence Coprocessor App. The primary goal of this environment is to provide a reproducible and self-contained setup for building the WASM program and SP1 circuit, deploying them to a local Valence Coprocessor service, and running the full test pipeline. This Nix setup replaces the need for Docker for these development tasks.

This flake is designed to work with the `valence-coprocessor-app` repository, specifically targeting a state compatible with its `v0.1.0` tag, with minimal necessary modifications to the application code for VFS compatibility.

## 2. Prerequisites

*   **Nix**: You must have Nix installed on your system. Follow the instructions at [nixos.org](https://nixos.org/download.html). Ensure that you have enabled support for `flakes` and `nix-command`. This can typically be done by adding/modifying these lines in your `nix.conf` (e.g., in `~/.config/nix/nix.conf` or `/etc/nix/nix.conf`):
    ```
    experimental-features = nix-command flakes
    ```

## 3. `flake.nix` Overview

The `flake.nix` file at the root of this project defines the entire Nix environment. Key components include:

*   **Inputs**: Specifies dependencies like `nixpkgs` (the Nix package collection), `rust-overlay` (for easy access to Rust toolchains), and `flake-parts` (for structuring the flake).
*   **Overlays**: The `rust-overlay` is used to provide specific Rust toolchains.
*   **Packages**:
    *   Custom Rust toolchains (nightly for WASM, SP1's `succinct` toolchain).
    *   SP1 tooling (`sp1-cli` via `cargo-prove`).
    *   Scripts for building, deploying, and testing (e.g., `build-wasm`, `deploy-to-service`, `full-pipeline`). These are packaged as applications runnable with `nix run`.
*   **Development Shells**: Pre-configured environments for specific tasks (`wasm-shell`, `sp1-shell`, and a `default` shell).
*   **Apps**: Exposes the packaged scripts so they can be executed easily with `nix run .#<appName>`.

## 4. Development Shells

You can enter these shells using the `nix develop .#<shellName>` command from the root of the `valence-coprocessor-app` project.

### 4.1. WASM Shell (`wasm-shell`)

*   **Purpose**: Provides an environment specifically for compiling the WASM program (`valence-coprocessor-app-program`).
*   **Entry**: `nix develop .#wasm-shell`
*   **Tools Provided**:
    *   Nightly Rust toolchain with the `wasm32-unknown-unknown` target.
    *   `wasm-bindgen-cli` (though not explicitly used by the current manual build command).
    *   Standard build tools, `curl`, `jq`.
*   **Manual WASM Build**:
    ```bash
    # Inside wasm-shell
    export RUSTFLAGS="--cfg=web_sys_unstable_apis" # Ensure this is set if not by shell hook
    cargo build --target wasm32-unknown-unknown -p valence-coprocessor-app-program --release
    ```
    The output will be in `target/wasm32-unknown-unknown/release/valence_coprocessor_app_program.wasm`. The `build-wasm` script (see below) handles copying this to an `optimized` directory.

### 4.2. SP1 Shell (`sp1-shell`)

*   **Purpose**: Provides an environment for building the SP1 ZK-VM circuit/program.
*   **Entry**: `nix develop .#sp1-shell`
*   **Tools Provided**:
    *   `rustup`: Used to manage Rust toolchains.
    *   SP1 `cargo-prove` CLI (via the `sp1` package, which also makes `cargo-prove` available).
    *   The `succinct` Rust toolchain, installed by `cargo-prove prove install-toolchain` and managed by `rustup`. The shell hook sets `RUSTUP_TOOLCHAIN=succinct` to make this active.
    *   LLVM, Clang, standard build tools, `curl`, `jq`.
*   **Manual SP1 Circuit Build**:
    The SP1 program for this app is typically located in `docker/build/program-circuit/program`.
    ```bash
    # Inside sp1-shell
    # The RUSTUP_TOOLCHAIN=succinct should already be set by the shell.
    # Verify with: rustc --version (should show 1.85.0-dev or similar for succinct toolchain)
    # And: cargo --version (should correspond to the succinct toolchain's cargo)
    cd docker/build/program-circuit/program
    cargo prove build
    ```
    The output ELF file is typically found at `../target/program.elf` (relative to the `program` directory, i.e., `docker/build/program-circuit/target/program.elf`). The `build-wasm` script also handles this and potential fallbacks.
*   **`cargo-prove prove install-toolchain`**: The `sp1-shell` environment automatically runs `cargo-prove prove install-toolchain` if the `succinct` toolchain isn't already installed by `rustup`. This ensures the correct version of Rust (currently a custom 1.85.0-dev via the `succinct` toolchain alias) required by SP1 is available.

### 4.3. Default Shell (`default`)

*   **Purpose**: A general-purpose shell providing access to the packaged helper scripts.
*   **Entry**: `nix develop` (or `nix develop .#default`)
*   **Available Commands**: Provides `menu` to see custom commands, which include:
    *   `build-wasm-cmd`
    *   `deploy-to-service-cmd`
    *   `full-pipeline-cmd`
    *   Shortcuts to enter other shells (`sp1`, `wasm`).

## 5. Core Scripts (Packaged as Nix Apps)

These scripts automate various parts of the development workflow and can be run using `nix run .#<scriptName>`.

### 5.1. `build-wasm` (`nix run .#build-wasm`)

*   **Functionality**:
    1.  Ensures `cargo-prove` is available (installs it if not, using the `install-cargo-prove` script).
    2.  Builds the WASM program (`valence-coprocessor-app-program`) using the `wasm-shell` environment. The output `valence_coprocessor_app_program.wasm` is copied to `target/wasm32-unknown-unknown/optimized/valence_coprocessor_app_lib.wasm`.
    3.  Builds the SP1 circuit (located in `docker/build/program-circuit/program`) using the `sp1-shell` environment.
        *   If successful, the ELF (e.g., `program.elf`) is copied to `target/sp1/optimized/valence-coprocessor-app-circuit`.
        *   If the SP1 build fails, it generates a fallback dummy ELF file at the same location (`target/sp1/optimized/valence-coprocessor-app-circuit`) using the `generate-sp1-elf` script. This allows WASM-only testing in `dev_mode`.
*   **Outputs**:
    *   WASM: `target/wasm32-unknown-unknown/optimized/valence_coprocessor_app_lib.wasm`
    *   SP1 Circuit: `target/sp1/optimized/valence-coprocessor-app-circuit` (either real or dummy)

### 5.2. `deploy-to-service` (`nix run .#deploy-to-service`)

*   **Functionality**:
    1.  Takes the WASM binary produced by `build-wasm` (`target/wasm32-unknown-unknown/optimized/valence_coprocessor_app_lib.wasm`).
    2.  Takes the SP1 circuit ELF (real or dummy) from `target/sp1/optimized/valence-coprocessor-app-circuit`.
    3.  Checks if the Valence Coprocessor service is running (default: `http://localhost:37281`).
    4.  Base64 encodes both the WASM and circuit ELF.
    5.  Constructs a JSON payload: `{"lib": "<wasm_b64>", "circuit": "<circuit_b64>", "dev_mode": true}`.
        *   **Note**: `dev_mode: true` is hardcoded in this script. This is crucial because it instructs the service to use mock proof generation and, importantly for VFS testing, makes the `/prove` endpoint call the WASM program's `entrypoint` function.
    6.  POSTs this payload to the service's `/api/registry/program` endpoint.
    7.  Prints the `program_id` if successful.
*   **Environment Variable**: `VALENCE_SERVICE_URL` can be set to target a different service endpoint (e.g., `http://my-service:1234/api/registry/program`).

### 5.3. `full-pipeline` (`nix run .#full-pipeline`)

*   **Functionality**: Orchestrates the entire build, deploy, and test sequence:
    1.  **Build**: Runs the `build-wasm` script.
    2.  **Deploy**: Runs the `deploy-to-service` script and captures the `program_id`.
    3.  **Prove (Trigger Entrypoint)**: Makes a POST request to `$SERVICE_URL/$PROGRAM_ID/prove`.
        *   The payload is: `{"payload": {"cmd": "store", "path": "/some_dir/long_filename_with_symbols!!.json"}, "dev_mode": true}`.
        *   This `payload` field is passed as arguments to the WASM `entrypoint` function.
        *   `dev_mode: true` ensures `entrypoint` is called.
    4.  **VFS Query**: Waits a few seconds, then makes a POST request to `$SERVICE_HOST/api/registry/program/$PROGRAM_ID/storage/fs`.
        *   The payload for this query is `{"path": "LONGFILE.JSO"}`. This path is the FAT-16 transformed version of the path sent in the prove payload, as processed by the WASM program.
        *   It decodes and prints the retrieved data.
*   **Expected Outcome**: If successful, it will print the content written by the WASM program to the VFS (e.g., `dynamic_fat16_content_v2`).

## 6. Valence Coprocessor Service Interaction Details

This section covers key learnings about how the `valence-coprocessor-app` (specifically its WASM program) interacts with the `valence-coprocessor-service`.

### 6.1. Running the Service

The `valence-coprocessor-service` is an external component that must be running for the `deploy-to-service` and `full-pipeline` scripts to work.
*   Clone the `valence-coprocessor` repository separately.
*   Run the service, typically with a command like:
    ```bash
    # From the root of the valence-coprocessor repository
    RUST_LOG=info cargo run --manifest-path Cargo.toml -p valence-coprocessor-service --profile optimized
    ```
    (Or `RUST_LOG=debug` for more verbose output from the service itself).
    It's recommended to run the service within its own Nix environment if it provides one, to ensure all its dependencies are met.

### 6.2. `dev_mode: true`

The `dev_mode: true` flag is critical for the development and testing workflow:
*   When included in the **deployment payload** (`/api/registry/program`), it registers the program such that subsequent proof requests can also use `dev_mode`.
*   When included in the **prove request payload** (`/api/programs/$PROGRAM_ID/prove`), and if the program was deployed with `dev_mode` support:
    1.  It bypasses actual ZK proof generation by SP1.
    2.  Crucially, it **calls the WASM program's `entrypoint` function**. The `args` field from the prove request payload is passed *within* a larger JSON structure as the `payload` field to the `entrypoint` function.
    3.  If `get_witnesses` is also defined in the WASM, it will be called first, and its output will also be part of the arguments to `entrypoint`.

### 6.3. VFS Path Transformation (Critical for `crates/program/src/lib.rs`)

The Valence Coprocessor service uses `buf-fs` for its WebAssembly Virtual File System (VFS). Observations during development indicate that `buf-fs`, in its default usage by the service for simple file writes via `abi::set_storage_file`, has characteristics similar to a FAT-16 filesystem:

*   **Path Format**: It expects flat paths (no subdirectories are automatically created by `abi::set_storage_file`).
*   **Filename Constraints**:
    *   **Case**: Paths appear to be treated case-insensitively or are canonicalized to uppercase. For reliability, using uppercase is recommended.
    *   **Length**: Adheres to an 8.3 filename convention (e.g., `FILENAME.EXT` where `FILENAME` is up to 8 characters and `EXT` is up to 3 characters).
    *   **Characters**: Standard alphanumeric characters are safe. Special characters might cause issues.
*   **Behavior**: If a non-compliant path (e.g., `"/foo/my long name.json"`) is used with `abi::set_storage_file`, the write might appear to succeed from the WASM program's perspective (i.e., `abi::set_storage_file` returns `Ok`), but the file may not be retrievable via the `/storage/fs` endpoint using that original path, or even a seemingly equivalent one. The file might be written to an unexpected mangled name or not saved correctly.

**Therefore, the WASM program (`crates/program/src/lib.rs`) *must* transform any desired user-friendly path into a compliant FAT-16 like path before calling `abi::set_storage_file`.**

The `internal_entrypoint` function in `crates/program/src/lib.rs` now contains logic to perform this transformation:
1.  **Input**: It expects a path string from the `payload` (e.g., `"path": "/some/directory/my_document.pdf"`).
2.  **Normalization**:
    *   Removes any leading `/`.
    *   Takes only the basename (e.g., `my_document.pdf` from `/some/directory/my_document.pdf`).
3.  **Stem & Extension Extraction**: Splits the basename into a stem (`my_document`) and an extension (`pdf`).
4.  **FAT-16 Conversion**:
    *   **Stem**: Converts to uppercase (`MY_DOCUMENT`), retains only ASCII alphanumeric characters (`MYDOCUMENT`), truncates to 8 characters (`MYDOCUME`). If the result is empty, it defaults to `DEFAULT`.
    *   **Extension**: Converts to uppercase (`PDF`), retains only ASCII alphanumeric characters (`PDF`), truncates to 3 characters (`PDF`). If the result is empty, it defaults to `DAT`.
5.  **Final Path**: Combines them, e.g., `MYDOCUME.PDF`.
    *   Example: `"path": "/some_dir/long_filename_with_symbols!!.json"` becomes `"path": "LONGFILE.JSO"`.

This transformed path is then used for `abi::set_storage_file`. The `full-pipeline` script is also configured to *query* the VFS using this same transformed path.

### 6.4. Querying VFS

The `/api/registry/program/$PROGRAM_ID/storage/fs` endpoint is used to read files from the WASM program's VFS.
*   It's a POST request.
*   The payload is `{"path": "<transformed_vfs_path>"}`.
*   The response (if successful and file exists) is `{"data": "<base64_encoded_content>"}`. If the file doesn't exist or is empty, `data` might be `""` or null.

## 7. Running the Full End-to-End Pipeline

1.  **Start the Valence Coprocessor Service** (see section 6.1). Ensure it's running and accessible (default: `http://localhost:37281`).
2.  **Run the pipeline script**:
    ```bash
    nix run .#full-pipeline -- --verbose
    ```
    The `-- --verbose` part passes the `--verbose` flag to the script itself, not to Nix.
3.  **Interpret Output**:
    *   Successful WASM and SP1 circuit builds.
    *   Successful deployment to the service (a `program_id` will be shown).
    *   The `prove` endpoint call should return `{"status":"received"}`.
    *   The VFS query for the transformed path (e.g., `LONGFILE.JSO`) should return HTTP 200.
    *   The decoded data should match the content written by the WASM program (e.g., `dynamic_fat16_content_v2`).

## 8. Troubleshooting

*   **Port Conflicts**: If the service fails to start with "Address already in use," ensure no other process (including an old instance of the service) is using port 37281. Use `kill $(lsof -t -i:37281)` or similar.
*   **Service Connectivity**: If `deploy-to-service` or `full-pipeline` fails to connect, double-check the service is running and the URL is correct (see `VALENCE_SERVICE_URL`).
*   **Nix Build Errors**: Examine the Nix error output. It might relate to fetching dependencies, Rust compilation errors within the sandboxed Nix build, or issues with the `flake.nix` syntax.
*   **VFS Issues (`{"data": ""}` or file not found)**:
    *   Verify the path transformation logic in `crates/program/src/lib.rs` and ensure the `STORAGE_PATH` in `flake.nix` (for the query) exactly matches the path WASM is writing to.
    *   Check the WASM logs (via service logs if `RUST_LOG=debug` is on for the service) to see the `final_vfs_path` used by `abi::set_storage_file`.
*   **Flake Dirty Warnings**: `warning: Git tree ... is dirty` from Nix is usually benign for local development but indicates uncommitted changes.
*   **Nix Untrusted Substituter Warnings**: These relate to Cachix or other binary caches not being in your user's trusted list. For this project, the flake defines `coffeetables.cachix.org`, but your local Nix setup might restrict its use. This may lead to longer build times as Nix builds more from source. 