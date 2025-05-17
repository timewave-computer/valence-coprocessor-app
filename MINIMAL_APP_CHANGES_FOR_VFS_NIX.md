# Minimal Application Code Changes for v0.1.0 Compatibility

This document outlines the minimal changes required to the `valence-coprocessor-app` (specifically its `crates/program` Rust code, assumed to be at a `v0.1.0` baseline) to make it work correctly with the Nix-based development pipeline and the Valence Coprocessor service's Virtual File System (VFS).

These changes ensure the WASM program builds correctly, interacts with the host environment as expected, and handles VFS path limitations.

## 1. `crates/program/Cargo.toml` Modifications

The primary change to the `Cargo.toml` for the WASM program crate is to ensure it's compiled into the correct crate types.

```diff
--- a/crates/program/Cargo.toml
+++ b/crates/program/Cargo.toml
@@ -X,X +X,X @@
 # ... other dependencies ...
 
 [lib]
-crate-type = ["rlib"] # Or whatever was originally present
+crate-type = ["cdylib", "rlib"]
```

*   **`crate-type = ["cdylib", "rlib"]`**:
    *   `cdylib`: This is essential for producing a WebAssembly binary that can be loaded and called as a dynamic library by the WASM runtime in the coprocessor service.
    *   `rlib`: Often included to allow the crate to also be used as a Rust library dependency if needed, though `cdylib` is the key for WASM execution.

## 2. `crates/program/src/lib.rs` Modifications

Several changes are needed in the main library file for the WASM program.

### 2.1. WASM-Specific Boilerplate

These are common additions for `#![no_std]` WebAssembly programs.

```rust
// Add at the top of the file
#![no_std]

extern crate alloc;

// Global allocator (if not already present)
#[cfg(target_arch = "wasm32")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

// Panic handler (if not already present)
#[cfg(target_arch = "wasm32")]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
```

*   **`wee_alloc`**: A small, efficient allocator suitable for WASM environments where binary size is a concern.
*   **Panic Handler**: A custom panic handler is required in `no_std` environments as the standard library's panic infrastructure is not available. This simple handler loops indefinitely on panic.

### 2.2. Function Signatures and ABI Compliance

The exported functions called by the host (e.g., `entrypoint`, `get_witnesses`) must be correctly defined to be C-ABI compatible and use the host's ABI functions for argument and data passing.

```rust
// Assuming original functions might have looked different, e.g.:
// pub fn entrypoint(args: SomeHostSpecificType) -> SomeHostSpecificType { ... }

// Modified functions:
use valence_coprocessor_wasm::abi; // Ensure this import is present
use serde_json::Value; // Or appropriate type for args/return
use alloc::vec::Vec; // For witness data
use valence_coprocessor::Witness; // For witness data

#[no_mangle]
pub extern "C" fn entrypoint() {
    // Log entry (optional but good for debugging)
    let _ = abi::log!("C-ABI entrypoint invoked");

    // Get arguments from host using ABI
    let result = abi::args().map_err(|e| anyhow::anyhow!("Failed to get args: {:?}", e))
        .and_then(|args_value: Value| { // Explicit type for clarity
            // ... (Original entrypoint logic, now adapted) ...
            // This is where the VFS path transformation will be added (see below)
            // For example, if entrypoint needs to return a value:
            // Ok(some_serde_json_value)
            internal_entrypoint(args_value) // Refactor logic into an internal function
        });

    match result {
        Ok(return_value) => {
            // Return data to host using ABI
            if let Err(e) = abi::ret(&return_value) {
                let _ = abi::log!("C-ABI entrypoint: failed to return value: {:?}", e);
            }
        }
        Err(e) => {
            let error_message = alloc::format!("C-ABI entrypoint: error: {}", e.to_string());
            let _ = abi::log!("{}", error_message);
            // Signal error, perhaps by not calling abi::ret() or using a specific abi::error() if available
        }
    }
}

#[no_mangle]
pub extern "C" fn get_witnesses() {
    let _ = abi::log!("C-ABI get_witnesses invoked");

    let result = abi::args().map_err(|e| anyhow::anyhow!("Failed to get args: {:?}", e))
        .and_then(|args_value: Value| {
            // ... (Original get_witnesses logic, now adapted) ...
            // Ok(vec_of_witnesses)
            internal_get_witnesses(args_value) // Refactor logic into an internal function
        });
    
    match result {
        Ok(witnesses) => {
            if let Err(e) = abi::ret_witnesses(witnesses) {
                let _ = abi::log!("C-ABI get_witnesses: failed to return witnesses: {:?}", e);
            }
        }
        Err(e) => {
            let error_message = alloc::format!("C-ABI get_witnesses: error: {}", e.to_string());
            let _ = abi::log!("{}", error_message);
        }
    }
}

// It's good practice to move the core logic into internal Rust functions:
fn internal_entrypoint(args: Value) -> anyhow::Result<Value> {
    // ... VFS path transformation and main logic here ...
    abi::log!("INTERNAL_ENTRYPOINT: Processing with args: {}", serde_json::to_string(&args).unwrap_or_default())?;
    // Example: Storing a file
    let path_from_payload = args.get("payload").unwrap().get("path").unwrap().as_str().unwrap();
    let transformed_path = transform_to_fat16(path_from_payload);
    abi::set_storage_file(&transformed_path, b"some data")?;
    Ok(Value::Null)
}

fn internal_get_witnesses(args: Value) -> anyhow::Result<Vec<Witness>> {
    // ... logic to generate witnesses ...
    abi::log!("INTERNAL_GET_WITNESSES: Processing with args: {}", serde_json::to_string(&args).unwrap_or_default())?;
    let value_bytes = args.get("value").unwrap().as_u64().unwrap().to_le_bytes().to_vec();
    Ok(vec![Witness::Data(value_bytes)])
}

// Placeholder for the transformation logic function
fn transform_to_fat16(original_path: &str) -> alloc::string::String {
    // Implementation of FAT-16 path transformation (see section 2.3)
    // For brevity, the full logic is in the next section.
    // This function would convert "path/to/my_file.json" to "MYFILE.JSO"
    let mut basename_str = if original_path.starts_with('/') {
        original_path.strip_prefix('/').unwrap_or(original_path)
    } else {
        original_path
    };
    if let Some(last_slash_idx) = basename_str.rfind('/') {
        basename_str = &basename_str[last_slash_idx + 1..];
    }

    let (stem_str, ext_str) = match basename_str.rfind('.') {
        Some(idx) if idx > 0 && idx < basename_str.len() - 1 => {
            (&basename_str[..idx], &basename_str[idx+1..])
        }
        _ => (basename_str, "")
    };

    let mut fat16_stem = stem_str.to_uppercase();
    fat16_stem.retain(|c| c.is_ascii_alphanumeric());
    fat16_stem.truncate(8);
    if fat16_stem.is_empty() {
        fat16_stem = "DEFAULT".to_string();
    }

    let mut fat16_ext = ext_str.to_uppercase();
    fat16_ext.retain(|c| c.is_ascii_alphanumeric());
    fat16_ext.truncate(3);
    if fat16_ext.is_empty() {
        fat16_ext = "DAT".to_string();
    }
    
    alloc::format!("{}.{}", fat16_stem, fat16_ext)
}
```

*   **`#[no_mangle] pub extern "C"`**: Ensures the function names are not mangled by the Rust compiler and use the C calling convention, making them callable from the WASM host.
*   **`abi::args()`**: Fetches arguments passed from the host.
*   **`abi::ret()` / `abi::ret_witnesses()`**: Returns data back to the host.
*   Refactoring core logic into `internal_entrypoint` and `internal_get_witnesses` improves separation of concerns between ABI handling and business logic.

### 2.3. VFS Path Transformation Logic

This is the most significant application-specific change needed for compatibility with the coprocessor service's VFS, which behaves like a FAT-16 filesystem. Before calling `abi::set_storage_file`, any user-provided or generated file path must be transformed.

The `transform_to_fat16` function shown above (and detailed in `NIX_DEVELOPMENT_GUIDE.md`) should be implemented and used within `internal_entrypoint` (or any function that writes to VFS).

**Example Usage within `internal_entrypoint`:**

```rust
fn internal_entrypoint(args: Value) -> anyhow::Result<Value> {
    abi::log!("INTERNAL_ENTRYPOINT: Start processing.")?;

    let payload = args
        .get("payload")
        .ok_or_else(|| anyhow::anyhow!("Missing 'payload' in entrypoint args"))?;

    let cmd = payload
        .get("cmd")
        .ok_or_else(|| anyhow::anyhow!("Missing 'cmd' in payload"))?
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("'cmd' is not a string"))?;

    match cmd {
        "store" => {
            let path_from_payload_str = payload
                .get("path")
                .ok_or_else(|| anyhow::anyhow!("Missing 'path' in store payload"))?
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("'path' is not a string"))?;
            
            // CRITICAL: Transform the path
            let final_vfs_path = transform_to_fat16(path_from_payload_str);
            
            abi::log!("WASM: Received path input '{}', transformed to VFS path '{}'", path_from_payload_str, final_vfs_path)?;

            let data_to_store_str = "content_from_v010_app"; // Example data
            let bytes_to_store = data_to_store_str.as_bytes();

            match abi::set_storage_file(&final_vfs_path, bytes_to_store) {
                Ok(_) => {
                    abi::log!("WASM: OK write to path: {}", final_vfs_path)?;
                    Ok(Value::Null) // Indicate success
                }
                Err(_e) => { // The error from abi::set_storage_file is Vec<u8>
                    abi::log!("WASM: ERR write to path {}", final_vfs_path)?;
                    Err(anyhow::anyhow!("Failed to set storage file at '{}'", final_vfs_path))
                }
            }
        }
        _ => {
            let err_msg_unknown_cmd = alloc::format!("unknown entrypoint command: {}", cmd);
            abi::log!("{}", err_msg_unknown_cmd)?;
            Err(anyhow::anyhow!(err_msg_unknown_cmd))
        }
    }
}
```

**Key aspects of the transformation:**
1.  **Basename**: Only the filename part of the path is used; directory structures are flattened.
2.  **Case**: Converted to uppercase.
3.  **Length**: Stem truncated to 8 characters, extension to 3 characters.
4.  **Characters**: Non-ASCII-alphanumeric characters are typically removed.
5.  **Defaults**: Default stem (`DEFAULT`) and extension (`DAT`) if parts become empty.

## Summary

These changes collectively ensure that the `v0.1.0` application code:
*   Compiles to a valid WASM module.
*   Includes necessary runtime components like an allocator and panic handler.
*   Correctly communicates with the host environment via defined ABI functions.
*   Crucially, interacts with the VFS in a compatible manner by transforming file paths.

Without these modifications, the original `v0.1.0` app would likely fail during WASM compilation, at runtime due to missing components, or when attempting VFS operations with non-compliant paths.
