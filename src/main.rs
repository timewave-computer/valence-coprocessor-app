async fn get_state_proof_handler(Json(payload): Json<StateProofRequest>) -> impl IntoResponse {
    match get_state_proof(
        &payload.address,
        &payload.ethereum_url,
        payload.height,
        payload.key.as_deref(),
    )
    .await
    {
        Ok(proof) => (StatusCode::OK, Json(proof)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Error getting state proof: {}", e),
        )
            .into_response(),
    }
} 