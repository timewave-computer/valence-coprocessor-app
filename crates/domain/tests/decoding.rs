#[cfg(test)]
mod tests {
    use types::CircuitOutput;

    #[test]
    fn test_decode_proof() {
        let proof_base64 = "AAAAAAAAAAAAAAAAvQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAHsid2l0aGRyYXdfcmVxdWVzdHMiOltdLCJzdGF0ZV9yb290IjpbMjQzLDE1Myw3NSw0NiwxNDksMTc2LDEzOCwxMjYsMjE1LDQwLDIwNCwyNDQsMjM4LDIwOCwxOCwyNTQsMTMzLDczLDIxMiw5Miw5MSwyMzgsMTU5LDIwNywxOTQsMTczLDk0LDExMCwxMSwxNjUsMjU0LDc0XX0LAAAAAAAAAHY0LjAuMC1yYy4zAA==";
        let proof = valence_coprocessor::Proof::try_from_base64(proof_base64)
            .expect("Failed to decode proof");
        let (_, proof_outputs) = proof.decode().unwrap();
        let outputs: CircuitOutput = serde_json::from_slice(&proof_outputs).unwrap();
    }
}
