use std::{env, fs, path::PathBuf, process::Command};

use serde_json::json;
use valence_coprocessor::{
    Blake3Context, MemoryBackend, ProgramData, Registry, Witness, mocks::MockZkVM,
};
use valence_coprocessor_wasm::host::ValenceWasm;
use valence_exchange_demo::{Operation, State};

#[test]
fn witnesses_are_computed_in_wasm() {
    let manifest = env!("CARGO_MANIFEST_DIR");
    let manifest = PathBuf::from(manifest).parent().unwrap().to_path_buf();
    let wasm = manifest.join("assets").join("demo.wasm");

    assert!(
        Command::new("make")
            .current_dir(&manifest)
            .arg("wasm")
            .status()
            .unwrap()
            .success()
    );
    assert!(wasm.is_file());

    let wasm = fs::read(wasm).unwrap();

    let data = MemoryBackend::default();
    let registry = Registry::from(data.clone());
    let program = ProgramData::default().with_module(wasm);
    let program = registry.register_program(program).unwrap();

    let capacity = 500;
    let vm = ValenceWasm::new(capacity).unwrap();
    let ctx = Blake3Context::init(program, data, vm, MockZkVM);

    let ret = ctx
        .execute_module(
            &program,
            "get_witnesses",
            json!({
                "mint": {"currency": "usd", "value": 1500.0},
                "exchange": {"from": "usd", "to": "eur", "value": 100.0}
            }),
        )
        .unwrap();

    let mut ret: Vec<Witness> = serde_json::from_value(ret).unwrap();

    let mut state: State = match ret.remove(0) {
        Witness::Data(d) => serde_json::from_slice(&d).unwrap(),
        _ => panic!("unexpected witness"),
    };

    for op in ret {
        let op: Operation = match op {
            Witness::Data(d) => serde_json::from_slice(&d).unwrap(),
            _ => panic!("unexpected witness"),
        };

        state.apply(&op).unwrap();
    }

    assert_eq!(state["usd"], 1400.0);
    assert_ne!(state["eur"], 0.0);
}
