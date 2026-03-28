use axum::{http::StatusCode, Json, extract::State};
use serde_json::Value;
use tracing::info;

/// POST /inbox
///
/// Receives an ActivityPub activity sent by a remote server.
/// In production you would verify the HTTP Signature here using the
/// sender's public key fetched via WebFinger / actor endpoint.
pub async fn handle_inbox(
    State(state): State<std::sync::Arc<crate::api::AppState>>,
    Json(activity): Json<Value>
) -> StatusCode {
    info!(
        r#type = activity["type"].as_str().unwrap_or("unknown"),
        actor = activity["actor"].as_str().unwrap_or("unknown"),
        "received ActivityPub activity"
    );

    if activity["type"].as_str() == Some("Create") {
        let client = reqwest::Client::new();
        let _ = client.post("http://localhost:8081/translate/ap-to-atproto")
            .json(&activity)
            .send()
            .await;
        tracing::info!("Relayed ActivityPub activity to external bridge service");
    }

    StatusCode::ACCEPTED
}
