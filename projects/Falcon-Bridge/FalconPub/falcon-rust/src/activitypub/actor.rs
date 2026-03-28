use axum::{extract::Path, Json};
use serde_json::{json, Value};

/// GET /actor/:name
///
/// Returns an ActivityPub Person object.  The public key PEM here is a
/// placeholder — in production you would load the real secp256k1 public key
/// from the database and encode it in PKIX/PEM format.
pub async fn get_actor(Path(name): Path<String>) -> Json<Value> {
    let base = format!("http://localhost:8080/actor/{name}");
    Json(json!({
        "@context": [
            "https://www.w3.org/ns/activitystreams",
            "https://w3id.org/security/v1"
        ],
        "id": base,
        "type": "Person",
        "preferredUsername": name,
        "inbox":  format!("{base}/inbox"),
        "outbox": format!("{base}/outbox"),
        "publicKey": {
            "id":    format!("{base}#main-key"),
            "owner": base,
            // Replace with real PEM encoded secp256k1 public key at runtime
            "publicKeyPem": "-----BEGIN PUBLIC KEY-----\n...\n-----END PUBLIC KEY-----"
        }
    }))
}
