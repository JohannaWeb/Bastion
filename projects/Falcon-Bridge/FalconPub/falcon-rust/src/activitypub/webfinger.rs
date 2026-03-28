use axum::{
    extract::Query,
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Deserialize)]
pub struct WebfingerParams {
    pub resource: String,
}

/// GET /.well-known/webfinger?resource=acct:user@domain
///
/// Returns a JRD (JSON Resource Descriptor) allowing other ActivityPub servers
/// to discover this instance's actor endpoint for the given user.
pub async fn webfinger(Query(params): Query<WebfingerParams>) -> Json<Value> {
    // Strip the acct: prefix if present
    let subject = params.resource.clone();
    let username = subject
        .strip_prefix("acct:")
        .and_then(|s| s.split('@').next())
        .unwrap_or(subject.as_str());

    Json(json!({
        "subject": subject,
        "links": [
            {
                "rel":  "self",
                "type": "application/activity+json",
                "href": format!("http://localhost:8080/actor/{username}")
            }
        ]
    }))
}
