use axum::{extract::Path, Json};
use serde_json::{json, Value};

/// GET /actor/:name/outbox
///
/// Returns an ActivityPub OrderedCollection for the actor's outbox.
/// In production you would paginate activities from the database.
pub async fn handle_outbox(Path(name): Path<String>) -> Json<Value> {
    Json(json!({
        "@context": "https://www.w3.org/ns/activitystreams",
        "id": format!("http://localhost:8080/actor/{name}/outbox"),
        "type": "OrderedCollection",
        "totalItems": 0,
        "orderedItems": []
    }))
}
