#![no_main]
sp1_zkvm::entrypoint!(main);

pub fn main() {
    let w = sp1_zkvm::io::read();
    let ret = valence_coprocessor_app::circuit::run(w);

    sp1_zkvm::io::commit(&ret);
}
