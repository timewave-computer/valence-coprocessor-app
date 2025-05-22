#[cfg(test)]
mod tests {
    use valence_coprocessor_app_program::utils::get_state_proof;

    // test an ethereum storage proof
    #[tokio::test]
    async fn test_simple_state_proof() {
        use alloy::providers::{Provider, ProviderBuilder};
        use common_merkle_proofs::merkle::types::MerkleVerifiable;
        use ethereum_merkle_proofs::merkle_lib::types::EthereumProofType;
        use std::str::FromStr;
        use url::Url;

        let sepolia_height = read_sepolia_height().await.unwrap();
        let storage_slot_key = hex::decode(read_ethereum_vault_balances_storage_key()).unwrap();
        let ethereum_url = "https://ethereum-sepolia-rpc.publicnode.com";

        let provider = ProviderBuilder::new().on_http(Url::from_str(ethereum_url).unwrap());
        let block = provider
            .get_block_by_number(alloy::eips::BlockNumberOrTag::Number(sepolia_height))
            .await
            .unwrap()
            .unwrap();

        // Get state proof using our function
        let state_proof = get_state_proof(
            &read_ethereum_vault_contract_address(),
            ethereum_url,
            sepolia_height,
            Some(alloy::hex::encode(&storage_slot_key).as_str()),
        )
        .await
        .unwrap();

        // Deserialize the proof bytes into EthereumProofType
        let proof_type: EthereumProofType = serde_json::from_slice(&state_proof.proof).unwrap();

        // Match on the proof type and verify
        match proof_type {
            EthereumProofType::Simple(simple_proof) => {
                assert!(simple_proof
                    .verify(block.header.state_root.as_slice())
                    .unwrap());
            }
            EthereumProofType::Account(_account_proof) => {
                panic!("Expected Simple proof but got Account proof");
            }
            _ => {
                panic!("Unsupported EthereumProofType: The MVP only supports SimpleProof and AccountProof");
            }
        }
    }

    async fn read_sepolia_height() -> Result<u64, anyhow::Error> {
        use alloy::providers::{Provider, ProviderBuilder};
        use std::str::FromStr;
        use url::Url;

        let ethereum_url = "https://ethereum-sepolia-rpc.publicnode.com";
        let provider = ProviderBuilder::new().on_http(Url::from_str(ethereum_url)?);
        let block = provider
            .get_block_by_number(alloy::eips::BlockNumberOrTag::Latest)
            .await?
            .expect("Failed to get Block!");
        Ok(block.header.number)
    }

    fn read_ethereum_vault_balances_storage_key() -> String {
        "0x0000000000000000000000000000000000000000000000000000000000000001".to_string()
    }

    fn read_ethereum_vault_contract_address() -> String {
        "0x8Fbd2549Dc447d229813ef5139b1aee8a9012eb3".to_string()
    }
}
