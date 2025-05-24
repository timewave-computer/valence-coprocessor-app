#![no_std]
extern crate alloc;
use valence_coprocessor_wasm::abi;

#[no_mangle]
pub extern "C" fn get_witnesses() {
    let args = abi::args().unwrap();
    let ret = valence_coprocessor_app_program::get_witnesses(args).unwrap();

    abi::ret_witnesses(ret).unwrap();
}

#[no_mangle]
pub extern "C" fn entrypoint() {
    let args = abi::args().unwrap();
    let ret = valence_coprocessor_app_program::entrypoint(args).unwrap();

    abi::ret(&ret).unwrap();
}
