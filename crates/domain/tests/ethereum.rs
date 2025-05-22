#[cfg(feature = "dev")]
#[cfg(test)]
mod tests {
    use valence_coprocessor_app_domain::validate;

    // test the helios wrapper proof verification
    #[test]
    fn test_validate_block() {
        let fixture = get_fixture();
        let vk_str =
            std::str::from_utf8(&fixture.vk_bytes).expect("Failed to convert vk bytes to string");
        let valid_block = validate(&fixture.proof_bytes, &fixture.public_values_bytes, vk_str)
            .expect("Failed to validate block");
        println!("Validated block: {:?}", valid_block);
    }

    struct Fixture {
        proof_bytes: Vec<u8>,
        public_values_bytes: Vec<u8>,
        vk_bytes: Vec<u8>,
    }

    fn get_fixture() -> Fixture {
        let proof_bytes = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/fixture/proof.bin"))
            .expect("Failed to read proof file");
        let public_values_bytes = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/fixture/public_outputs.bin"
        ))
        .unwrap();
        let vk_bytes = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/fixture/vk.bin"))
            .expect("Failed to read vk file");
        Fixture {
            proof_bytes,
            public_values_bytes,
            vk_bytes,
        }
    }
}
