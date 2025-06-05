//! Skip API client for route discovery and message construction

use anyhow::{Result, anyhow};
use reqwest::Client;
use serde::Serialize;
use tracing::{info, warn, error, debug};

use crate::constants::*;
use crate::types::{TransferRequest, SkipApiResponse};

/// Skip API client for route and message discovery
pub struct SkipApiClient {
    client: Client,
    base_url: String,
    _api_key: Option<String>,
}

/// Skip API route request
#[derive(Debug, Serialize)]
struct RouteRequest {
    amount_in: String,
    source_asset_denom: String,
    source_asset_chain_id: String,
    dest_asset_denom: String,
    dest_asset_chain_id: String,
}

/// Skip API messages request  
#[derive(Debug, Serialize)]
struct MessagesRequest {
    amount_in: String,
    source_asset_denom: String,
    source_asset_chain_id: String,
    dest_asset_denom: String,
    dest_asset_chain_id: String,
    address_list: Vec<String>,
}

impl SkipApiClient {
    /// Creates a new Skip API client with configuration
    pub fn new(base_url: &str, api_key: Option<&str>) -> Result<Self> {
        let mut client_builder = Client::builder();
        
        // Add API key to default headers if provided
        if let Some(key) = api_key {
            let mut headers = reqwest::header::HeaderMap::new();
            headers.insert(
                reqwest::header::HeaderName::from_static("x-api-key"),
                reqwest::header::HeaderValue::from_str(key)
                    .map_err(|e| anyhow!("Invalid API key format: {}", e))?,
            );
            client_builder = client_builder.default_headers(headers);
        }

        Ok(Self {
            client: client_builder.build()
                .map_err(|e| anyhow!("Failed to create HTTP client: {}", e))?,
            base_url: base_url.to_string(),
            _api_key: api_key.map(|s| s.to_string()),
        })
    }

    /// Get messages for token transfer (this is what we'll actually use)
    pub async fn get_messages(&self, request: &TransferRequest) -> Result<SkipApiResponse> {
        info!("Requesting Skip API messages for token transfer");

        let messages_request = MessagesRequest {
            amount_in: request.amount.to_string(),
            source_asset_denom: TOKEN_CONTRACT_ADDRESS.to_string(),
            source_asset_chain_id: EXPECTED_SOURCE_CHAIN.to_string(),
            dest_asset_denom: TOKEN_COSMOS_HUB_DENOM.to_string(),
            dest_asset_chain_id: EXPECTED_DEST_CHAIN.to_string(),
            address_list: vec![
                request.source_address.clone(),
                request.destination.clone(),
            ],
        };

        debug!("Messages request: {:?}", messages_request);

        let url = format!("{}/v2/fungible/msgs", self.base_url);
        let response = self.client
            .post(&url)
            .json(&messages_request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            error!("Skip API error {}: {}", status, error_text);
            return Err(anyhow!("Skip API request failed: {} - {}", status, error_text));
        }

        let messages: SkipApiResponse = response.json().await?;
        
        // Validate that we have a eureka_transfer operation
        if !messages.has_eureka_transfer() {
            warn!("No eureka_transfer operation found in Skip API response");
            return Err(anyhow!("Response does not contain eureka_transfer operation"));
        }

        info!("Successfully retrieved {} operations from Skip API", messages.operations.len());
        Ok(messages)
    }

    /// Get route information (for discovery/validation purposes)
    pub async fn get_route(&self, request: &TransferRequest) -> Result<SkipApiResponse> {
        info!("Requesting Skip API route for token transfer");

        let route_request = RouteRequest {
            amount_in: request.amount.to_string(),
            source_asset_denom: TOKEN_CONTRACT_ADDRESS.to_string(),
            source_asset_chain_id: EXPECTED_SOURCE_CHAIN.to_string(),
            dest_asset_denom: TOKEN_COSMOS_HUB_DENOM.to_string(),
            dest_asset_chain_id: EXPECTED_DEST_CHAIN.to_string(),
        };

        debug!("Route request: {:?}", route_request);

        let url = format!("{}/v2/fungible/route", self.base_url);
        let response = self.client
            .post(&url)
            .json(&route_request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            error!("Skip API route error {}: {}", status, error_text);
            return Err(anyhow!("Skip API route request failed: {} - {}", status, error_text));
        }

        let route: SkipApiResponse = response.json().await?;
        
        info!("Successfully retrieved route with {} operations", route.operations.len());
        Ok(route)
    }

    /// Validate that a route matches our hardcoded expectations
    pub fn validate_route(&self, response: &SkipApiResponse) -> Result<()> {
        // Check for eureka_transfer operation
        if !response.has_eureka_transfer() {
            return Err(anyhow!("Route does not contain eureka_transfer operation"));
        }

        // Extract route data and validate hash
        let route_data = crate::types::RouteData::from_skip_response(response)?;
        let calculated_hash = route_data.generate_hash();
        
        if calculated_hash != EXPECTED_ROUTE_HASH {
            warn!("Route hash mismatch: expected {}, got {}", EXPECTED_ROUTE_HASH, calculated_hash);
            return Err(anyhow!("Route hash validation failed"));
        }

        // Check fee threshold
        let total_fees = response.total_fees();
        if total_fees > FEE_THRESHOLD_TOKEN_WEI {
            warn!("Fees exceed threshold: {} > {}", total_fees, FEE_THRESHOLD_TOKEN_WEI);
            return Err(anyhow!("Fees exceed maximum threshold"));
        }

        info!("Route validation passed: hash matches and fees within limits");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_skip_api_client_creation() {
        let client = SkipApiClient::new(SKIP_API_BASE_URL, None).unwrap();
        assert_eq!(client.base_url, SKIP_API_BASE_URL);
        assert!(client._api_key.is_none());

        let client_with_key = SkipApiClient::new(SKIP_API_BASE_URL, Some("test-key")).unwrap();
        assert_eq!(client_with_key._api_key, Some("test-key".to_string()));
    }

    // Note: These tests would require network access and valid API responses
    // For now, we're focusing on structure rather than full integration testing
} 