use std::{env, fs, path::PathBuf};

use sp1_sdk::include_elf;

pub const CIRCUIT_ELF: &[u8] = include_elf!("app-circuit");

fn main() {
    let path = env::args().skip(1).next().unwrap();
    let path = PathBuf::from(path);

    fs::write(path, CIRCUIT_ELF).unwrap();
}
