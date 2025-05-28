extern crate alloc;
use alloc::vec::Vec;
use alloy_primitives::Bytes;
use alloy_rlp::decode_exact;
use anyhow::Result;

/// Decodes RLP-encoded bytes into a vector of bytes.
///
/// # Arguments
/// * `bytes` - The RLP-encoded bytes to decode
///
/// # Returns
/// A vector of decoded bytes
///
/// # Panics
/// Panics if the bytes cannot be decoded
pub fn rlp_decode_bytes(bytes: &[u8]) -> Result<Vec<Bytes>> {
    let decoded =
        decode_exact(bytes).map_err(|e| anyhow::anyhow!("Failed to decode RLP bytes: {:?}", e))?;
    Ok(decoded)
}

pub trait RlpDecodable {
    fn rlp_decode(rlp: &[u8]) -> Result<Self>
    where
        Self: Sized;
}
