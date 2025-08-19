use valence_coprocessor_wasm::abi;

#[unsafe(no_mangle)]
pub extern "C" fn get_witnesses() {
    let args = abi::args().unwrap();
    let ret = super::get_witnesses(args).unwrap();

    abi::ret_witnesses(ret).unwrap();
}

#[unsafe(no_mangle)]
pub extern "C" fn entrypoint() {
    let args = abi::args().unwrap();
    let ret = super::entrypoint(args).unwrap();

    abi::ret(&ret).unwrap();
}
