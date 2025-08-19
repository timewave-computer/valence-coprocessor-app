use valence_coprocessor::Witness;

pub fn circuit(witnesses: Vec<Witness>) -> anyhow::Result<Vec<u8>> {
    let value = witnesses[0]
        .as_data()
        .ok_or_else(|| anyhow::anyhow!("failed to extract witness data"))?;

    let value = <[u8; 8]>::try_from(value)?;
    let value = u64::from_le_bytes(value);
    let value = value.wrapping_add(1);

    Ok(value.to_le_bytes().to_vec())
}
