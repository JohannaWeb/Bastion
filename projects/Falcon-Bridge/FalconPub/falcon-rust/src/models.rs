use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct Server {
    pub id: i64,
    pub name: String,
    pub owner_did: String,
    pub protocol: String, // "atproto" or "activitypub"
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct Channel {
    pub id: i64,
    pub server_id: i64,
    pub name: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct Message {
    pub id: i64,
    pub channel_id: i64,
    pub author_did: String,
    pub author_handle: String,
    pub content: String,
    pub protocol: Option<String>,
    pub external_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct Member {
    pub id: i64,
    pub server_id: i64,
    pub did: String,
    pub handle: String,
    pub joined_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct Conversation {
    pub id: i64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct ConversationParticipant {
    pub id: i64,
    pub conversation_id: i64,
    pub did: String,
    pub handle: String,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct ConversationMessage {
    pub id: i64,
    pub conversation_id: i64,
    pub author_did: String,
    pub author_handle: String,
    pub content: String,
    pub protocol: Option<String>,
    pub external_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct ProtocolMapping {
    pub did: String,
    pub actor_uri: String,
    pub protocol: String,
    pub last_resolved: DateTime<Utc>,
}
