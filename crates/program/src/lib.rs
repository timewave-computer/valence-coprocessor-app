#![no_std]

extern crate alloc;

// Global allocator
#[cfg(target_arch = "wasm32")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

// Panic handler
#[cfg(target_arch = "wasm32")]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

use alloc::{string::ToString as _, vec::Vec};
use serde_json::Value;
use valence_coprocessor::Witness;
use valence_coprocessor_wasm::abi;

// Internal logic for get_witnesses
fn internal_get_witnesses(args: Value) -> anyhow::Result<Vec<Witness>> {
    abi::log!(
        "internal_get_witnesses: received a proof request with arguments {}",
        serde_json::to_string(&args).unwrap_or_default()
    )?;

    let value = args
        .get("value")
        .ok_or_else(|| anyhow::anyhow!("Missing 'value' in args"))?
        .as_u64()
        .ok_or_else(|| anyhow::anyhow!("'value' is not a u64"))?;
    let value_bytes = value.to_le_bytes().to_vec();

    Ok([Witness::Data(value_bytes)].to_vec())
}

// Internal logic for entrypoint
fn internal_entrypoint(args: Value) -> anyhow::Result<Value> {
    abi::log!("INTERNAL_ENTRYPOINT: Start processing.")?;

    let log_message_full_args = alloc::format!(
        "INTERNAL_ENTRYPOINT: Received request with full arguments: {}",
        serde_json::to_string(&args).unwrap_or_default()
    );
    abi::log!("{}", log_message_full_args)?;

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
            
            // 1. Normalize: Remove leading `/`, take basename.
            let mut basename_str = if path_from_payload_str.starts_with('/') {
                path_from_payload_str.strip_prefix('/').unwrap_or(path_from_payload_str)
            } else {
                path_from_payload_str
            };
            if let Some(last_slash_idx) = basename_str.rfind('/') {
                basename_str = &basename_str[last_slash_idx + 1..];
            }

            // 2. Stem and Extension
            let (stem_str, ext_str) = match basename_str.rfind('.') {
                Some(idx) if idx > 0 && idx < basename_str.len() - 1 => { // Ensure dot is not first or last char
                    (&basename_str[..idx], &basename_str[idx+1..])
                }
                _ => (basename_str, "") // No extension or path is like ".file" or "file."
            };

            // 3. FAT-16 Stem
            let mut fat16_stem = stem_str.to_uppercase();
            fat16_stem.retain(|c| c.is_ascii_alphanumeric());
            fat16_stem.truncate(8);
            if fat16_stem.is_empty() {
                fat16_stem = "DEFAULT".to_string();
            }

            // 4. FAT-16 Extension
            let mut fat16_ext = ext_str.to_uppercase();
            fat16_ext.retain(|c| c.is_ascii_alphanumeric());
            fat16_ext.truncate(3);
            if fat16_ext.is_empty() { // If original ext was empty or became empty
                fat16_ext = "DAT".to_string(); // Default extension
            }
            
            // 5. Combine
            let final_vfs_path = alloc::format!("{}.{}", fat16_stem, fat16_ext);
            
            // Log the original and final transformed path once.
            abi::log!("WASM: Received path input '{}', transformed to VFS path '{}'", path_from_payload_str, final_vfs_path)?;

            let data_to_store_str = "dynamic_fat16_content_v2"; // Keep consistent marker for now
            abi::log!("WASM: Storing to '{}', Data: '{}', Len: {}.", final_vfs_path, data_to_store_str, data_to_store_str.len())?;

            let bytes_to_store = data_to_store_str.as_bytes();

            match abi::set_storage_file(&final_vfs_path, bytes_to_store) {
                Ok(_) => {
                    abi::log!("WASM: OK write to path: {}", final_vfs_path)?;
                    Ok(Value::Null)
                }
                Err(_raw_err_val_bytes) => {
                    abi::log!("WASM: ERR write to path {} (details in service log)", final_vfs_path)?;
                    Err(anyhow::anyhow!("Failed to set storage file at '{}' (see service logs)", final_vfs_path))
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

// Exported C-ABI compatible get_witnesses
#[no_mangle]
pub extern "C" fn get_witnesses() {
    // Log entry into the C-ABI function
    if let Err(_e) = abi::log!("C-ABI get_witnesses invoked") {
        // Minimal error handling for log failure itself
        // In a real scenario, might want a more robust way to report this.
        // For now, if logging fails, we can't do much other than proceed.
    }

    let result = abi::args().map_err(|e| anyhow::anyhow!("C-ABI get_witnesses: failed to get args: {:?}", e))
        .and_then(internal_get_witnesses);

    match result {
        Ok(witnesses) => {
            if let Err(_e) = abi::log!("C-ABI get_witnesses: success, returning {} witnesses", witnesses.len()) {
                // log failure
            }
            if let Err(e) = abi::ret_witnesses(witnesses) {
                 // Log error during abi::ret_witnesses
                let _ = abi::log!("C-ABI get_witnesses: failed to return witnesses via ABI: {:?}", e);
                // Potentially panic or find another way to signal critical error if ABI return fails.
                // For now, mimicking the example's style of letting it unwrap implicitly or panic if ret fails.
            }
        }
        Err(e) => {
            // Log the error
            let error_message = alloc::format!("C-ABI get_witnesses: error: {}", e.to_string());
            if let Err(_log_e) = abi::log!("{}", error_message) {
                // Double error, log failure itself failed.
            }
            // How to signal error to host? The example `docker/build/program-wasm/src/lib.rs` uses .unwrap()
            // which would panic. If the host expects no return or a status code, this needs adjustment.
            // For now, if `internal_get_witnesses` returns Err, this function will just complete.
            // The host might interpret lack of `abi::ret_witnesses` call as failure.
            // Or, we might need an `abi::error()` function if available.
            // To be safe and explicit about failure propagation if `abi::ret_witnesses` isn't called:
            // Consider adding: abi::panic(error_message); if such a function exists and is appropriate.
            // For now, we will rely on the host to detect no call to abi::ret_witnesses as an issue.
        }
    }
}

// Exported C-ABI compatible entrypoint
#[no_mangle]
pub extern "C" fn entrypoint() {
    if let Err(_e) = abi::log!("C-ABI entrypoint invoked") {
        // log failure
    }

    let result = abi::args().map_err(|e| anyhow::anyhow!("C-ABI entrypoint: failed to get args: {:?}", e))
        .and_then(internal_entrypoint);

    match result {
        Ok(return_value) => {
            if let Err(_e) = abi::log!("C-ABI entrypoint: success, returning value: {}", serde_json::to_string(&return_value).unwrap_or_default()) {
                // log failure
            }
            if let Err(e) = abi::ret(&return_value) {
                let _ = abi::log!("C-ABI entrypoint: failed to return value via ABI: {:?}", e);
            }
        }
        Err(e) => {
            let error_message = alloc::format!("C-ABI entrypoint: error: {}", e.to_string());
            if let Err(_log_e) = abi::log!("{}", error_message) {
                // log failure
            }
            // Similar to get_witnesses, relying on host to detect no abi::ret call.
        }
    }
}
