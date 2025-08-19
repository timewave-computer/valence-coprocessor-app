use serde_json::Value;
use valence_coprocessor::Witness;
use valence_coprocessor_wasm::abi;

mod valence;

pub fn get_witnesses(args: Value) -> anyhow::Result<Vec<Witness>> {
    abi::log!(
        "received a proof request with arguments {}",
        serde_json::to_string(&args).unwrap_or_default()
    )?;

    let value = args["value"]
        .as_u64()
        .ok_or_else(|| anyhow::anyhow!("unexpected value"))?;
    let value = value.to_le_bytes().to_vec();

    Ok([Witness::Data(value)].to_vec())
}

pub fn entrypoint(args: Value) -> anyhow::Result<Value> {
    abi::log!(
        "received an entrypoint request with arguments {}",
        serde_json::to_string(&args).unwrap_or_default()
    )?;

    let cmd = args["payload"]["cmd"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("undefined command"))?;

    match cmd {
        "store" => {
            let path = args["payload"]["path"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("unexpected input"))?
                .to_string();
            let bytes = serde_json::to_vec(&args)?;

            abi::set_storage_file(&path, &bytes)?;
        }

        _ => anyhow::bail!("unknown entrypoint command"),
    }

    Ok(args)
}
