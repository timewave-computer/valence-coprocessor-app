use alloc::{format, string::ToString as _, vec, vec::Vec};
use serde_json::{Value, json};
use valence_coprocessor::Witness;
use valence_coprocessor_wasm::abi;

extern crate alloc;

pub fn get_witnesses(args: &Value) -> anyhow::Result<Vec<Witness>> {
    let name = args
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("failed to fetch `name` from the provided arguments."))?;

    abi::log!("received name: `{name}`")?;

    let advice = match abi::http(&json!({
        "url": "https://api.adviceslip.com/advice",
        "method": "get",
        "response": "json",
        "headers": {
            "Accept": "application/json"
        }
    })) {
        Ok(v) => v
            .get("body")
            .and_then(|v| v.get("slip"))
            .and_then(|v| v.get("advice"))
            .and_then(Value::as_str)
            .unwrap_or("Oh no! Reply was inconsistent")
            .to_string(),
        Err(_) => "Oh no! HTTP API not available".to_string(),
    };

    abi::log!("received advice: `{advice}`")?;

    let message = format!("Hello, {name}! Here is an advice for you: {advice}");
    let witness = Witness::Data(message.as_bytes().to_vec());

    abi::log!("computed message: `{message}`")?;

    Ok(vec![witness])
}

#[test]
fn message_is_computed_correctly() {
    abi::initialize_default_runtime();

    let args = json!({"name": "Valence"});
    let witnesses = get_witnesses(&args).unwrap();
    let message = witnesses[0].as_data().unwrap().to_vec();
    let message = alloc::string::String::from_utf8(message).unwrap();

    let expected = "Hello, Valence! Here is an advice for you: ";
    let log = abi::runtime().log;

    assert!(message.starts_with(expected));
    assert_eq!(&log[0], "received name: `Valence`");
    assert!(log[1].starts_with("received advice"));
    assert!(log[2].starts_with("computed message"));
}
