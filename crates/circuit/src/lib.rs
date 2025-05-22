use common_merkle_proofs::merkle::types::MerkleVerifiable;
use ethereum_merkle_proofs::merkle_lib::{rlp_decode_account, types::EthereumProofType};
use types::CircuitWitness;
use valence_coprocessor::Witness;

pub fn circuit(witnesses: Vec<Witness>) -> Vec<u8> {
    let circuit_input_witness = witnesses.first().unwrap();
    // this macro isn't necessary, but rust analyzer throws a false positive
    #[allow(unused)]
    let mut circuit_input_serialized: Vec<u8> = vec![];
    match circuit_input_witness {
        Witness::Data(data) => {
            circuit_input_serialized = data.clone();
        }
        Witness::StateProof(_) => panic!(
            "Unexpected Input: For this example template we encode all data in a single field."
        ),
    }

    // deserialize the CircuitWitness
    let input: CircuitWitness = serde_json::from_slice(&circuit_input_serialized).unwrap();

    // verify all Ethereum proofs
    for proof in input.state_proofs {
        let proof: EthereumProofType = serde_json::from_slice(&proof.proof).unwrap();
        match &proof {
            EthereumProofType::Account(account_proof) => {
                let decoded_account = rlp_decode_account(&account_proof.value).unwrap();
                println!(
                    "Decoded account state from account proof: {:?}",
                    decoded_account
                );
            }
            _ => {}
        }
        assert!(proof.verify(&input.state_root).unwrap());
    }

    // commit the helios root that we used for verification as an output
    input.state_root.to_vec()
}
