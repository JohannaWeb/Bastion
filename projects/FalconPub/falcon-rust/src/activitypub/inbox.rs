use axum::{http::StatusCode, Json};
use serde::Deserialize;
use serde_json::{json, Value};
use tracing::info;

/// Any incoming ActivityPub activity share the common fields we care about.
#[derive(Debug, Deserialize)]
pub struct Activity {
    #[serde(rename = "type")]
    pub activity_type: Option<String>,
    pub actor: Option<String>,
    pub object: Option<Value>,
}

/// POST /inbox
///
/// Receives an ActivityPub activity sent by a remote server.
/// In production you would verify the HTTP Signature here using the
/// sender's public key fetched via WebFinger / actor endpoint.
pub async fn handle_inbox(Json(activity): Json<Activity>) -> StatusCode {
    info!(
        r#type = activity.activity_type.as_deref().unwrap_or("unknown"),
        actor = activity.actor.as_deref().unwrap_or("unknown"),
        "received ActivityPub activity"
    );
    StatusCode::ACCEPTED
}
