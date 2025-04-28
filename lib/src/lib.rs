#![no_std]

use valence_coprocessor_app::program;
use valence_coprocessor_wasm::abi;

extern crate alloc;

#[unsafe(no_mangle)]
pub extern "C" fn entrypoint() {
    let args = abi::args().unwrap();
    let command = args["command"].as_str().unwrap();

    match command {
        "reset" => abi::set_program_storage(&[]).unwrap(),
        _ => panic!("unknown command"),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn get_witnesses() {
    let args = abi::args().unwrap();
    let witnesses = program::get_witnesses(&args).unwrap();

    abi::ret_witnesses(witnesses).unwrap();
}
