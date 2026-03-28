use serde_json::{json, Value};
use chrono::Utc;

pub struct Translator;

impl Translator {
    /// Translates an AT Protocol message record to an ActivityPub Note activity.
    pub fn atproto_to_activitypub(message: &Value) -> Value {
        let id = message["id"].as_i64().unwrap_or(0);
        let author_handle = message["authorHandle"].as_str().unwrap_or("unknown");
        let content = message["content"].as_str().unwrap_or("");
        let created_at = message["createdAt"].as_str().unwrap_or("");

        json!({
            "@context": "https://www.w3.org/ns/activitystreams",
            "type": "Create",
            "id": format!("http://localhost:8081/bridge/atproto/{}", id),
            "actor": format!("http://localhost:8080/actor/{}", author_handle),
            "object": {
                "id": format!("http://localhost:8081/bridge/atproto/{}/note", id),
                "type": "Note",
                "published": if created_at.is_empty() { Utc::now().to_rfc3339() } else { created_at.to_string() },
                "attributedTo": format!("http://localhost:8080/actor/{}", author_handle),
                "content": content,
                "to": ["https://www.w3.org/ns/activitystreams#Public"]
            }
        })
    }

    /// Translates an ActivityPub Note activity to an AT Protocol message record structure.
    pub fn activitypub_to_atproto(activity: &Value) -> Option<Value> {
        let activity_type = activity["type"].as_str()?;
        if activity_type != "Create" {
            return None;
        }

        let object = &activity["object"];
        let obj_type = object["type"].as_str()?;
        if obj_type != "Note" {
            return None;
        }

        let content = object["content"].as_str().unwrap_or("");
        let author = activity["actor"].as_str().unwrap_or("unknown");
        let external_id = object["id"].as_str().unwrap_or("");

        Some(json!({
            "content": content,
            "authorDid": format!("did:activitypub:{}", author.replace("http://", "").replace("/", "-")),
            "authorHandle": author,
            "externalId": external_id,
            "protocol": "activitypub"
        }))
    }
}
