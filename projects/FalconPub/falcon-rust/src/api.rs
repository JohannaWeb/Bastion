use axum::{
    routing::{get, post},
    extract::{State, Query},
    Json, Router,
    http::StatusCode,
    response::IntoResponse,
};
use std::sync::Arc;
use sqlx::SqlitePool;
use serde_json::{Value, json};
use crate::models::{Server, Channel, Message};

pub struct AppState {
    pub db: SqlitePool,
}

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        // ActivityPub
        .route("/.well-known/webfinger", get(crate::activitypub::webfinger::webfinger))
        .route("/inbox", post(crate::activitypub::inbox::handle_inbox))
        .route("/actor/:name", get(crate::activitypub::actor::get_actor))
        .route("/actor/:name/outbox", get(crate::activitypub::outbox::handle_outbox))
        // Servers
        .route("/xrpc/app.falcon.server.list", get(list_servers))
        .route("/xrpc/app.falcon.server.get", get(get_server))
        .route("/xrpc/app.falcon.server.create", post(create_server))
        .route("/xrpc/app.falcon.server.invite", post(invite_to_server))
        // Channels
        .route("/xrpc/app.falcon.channel.list", get(list_channels))
        .route("/xrpc/app.falcon.channel.create", post(create_channel))
        .route("/xrpc/app.falcon.channel.postMessage", post(post_message))
        .route("/xrpc/app.falcon.channel.getMessages", get(get_messages))
        // Convos (DMs)
        .route("/xrpc/app.falcon.convo.list", get(list_convos))
        .route("/xrpc/app.falcon.convo.get", get(get_convo))
        .route("/xrpc/app.falcon.convo.getMessages", get(get_convo_messages))
        .route("/xrpc/app.falcon.convo.sendMessage", post(send_convo_message))
        .with_state(state)
}

// ─── Helpers ────────────────────────────────────────────────────────────────

async fn server_to_summary(db: &SqlitePool, server: Server) -> Value {
    let channels = sqlx::query_as::<_, Channel>(
        "SELECT * FROM channels WHERE server_id = ?",
    )
    .bind(server.id)
    .fetch_all(db)
    .await
    .unwrap_or_default();

    let channels_json: Vec<Value> = channels
        .iter()
        .map(|c| json!({ "id": c.id, "name": c.name }))
        .collect();

    json!({
        "id": server.id,
        "name": server.name,
        "ownerDid": server.owner_did,
        "channels": channels_json
    })
}

// ─── Server Endpoints ────────────────────────────────────────────────────────

async fn list_servers(State(state): State<Arc<AppState>>) -> Json<Value> {
    let servers = sqlx::query_as::<_, Server>("SELECT * FROM servers")
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();

    let mut summaries = vec![];
    for s in servers {
        summaries.push(server_to_summary(&state.db, s).await);
    }
    Json(json!(summaries))
}

async fn get_server(
    State(state): State<Arc<AppState>>,
    Query(params): Query<Value>,
) -> impl IntoResponse {
    let server_id = params["serverId"]
        .as_str()
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(0);

    match sqlx::query_as::<_, Server>("SELECT * FROM servers WHERE id = ?")
        .bind(server_id)
        .fetch_one(&state.db)
        .await
    {
        Ok(s) => (StatusCode::OK, Json(server_to_summary(&state.db, s).await)).into_response(),
        Err(_) => (StatusCode::NOT_FOUND, Json(json!({ "error": "Not found" }))).into_response(),
    }
}

async fn create_server(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<Value>,
) -> Json<Value> {
    let name = payload["name"].as_str().unwrap_or("Untitled Server");
    let owner_did = payload["ownerDid"].as_str().unwrap_or("did:activitypub:anon");

    // Insert server
    let server_id = sqlx::query("INSERT INTO servers (name, owner_did) VALUES (?, ?)")
        .bind(name)
        .bind(owner_did)
        .execute(&state.db)
        .await
        .map(|r| r.last_insert_rowid())
        .unwrap_or(0);

    // Auto-create #general channel
    let channel_id = sqlx::query("INSERT INTO channels (server_id, name) VALUES (?, ?)")
        .bind(server_id)
        .bind("general")
        .execute(&state.db)
        .await
        .map(|r| r.last_insert_rowid())
        .unwrap_or(0);

    // Add owner as first member
    let _ = sqlx::query("INSERT INTO members (server_id, did, handle) VALUES (?, ?, ?)")
        .bind(server_id)
        .bind(owner_did)
        .bind(owner_did)
        .execute(&state.db)
        .await;

    Json(json!({
        "id": server_id,
        "name": name,
        "ownerDid": owner_did,
        "channelId": channel_id
    }))
}

async fn invite_to_server(
    State(state): State<Arc<AppState>>,
    Query(params): Query<Value>,
    Json(payload): Json<Value>,
) -> impl IntoResponse {
    let server_id = params["serverId"]
        .as_str()
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(0);

    let handle = payload["handle"].as_str().unwrap_or("").to_string();
    let did = format!("did:activitypub:{}", handle.replace('.', "-"));

    let exists: bool = sqlx::query_scalar(
        "SELECT COUNT(*) > 0 FROM members WHERE server_id = ? AND did = ?",
    )
    .bind(server_id)
    .bind(&did)
    .fetch_one(&state.db)
    .await
    .unwrap_or(false);

    if !exists {
        let _ = sqlx::query("INSERT INTO members (server_id, did, handle) VALUES (?, ?, ?)")
            .bind(server_id)
            .bind(&did)
            .bind(&handle)
            .execute(&state.db)
            .await;
    }

    (
        StatusCode::OK,
        Json(json!({ "did": did, "handle": handle })),
    )
        .into_response()
}

// ─── Channel Endpoints ───────────────────────────────────────────────────────

async fn list_channels(
    State(state): State<Arc<AppState>>,
    Query(params): Query<Value>,
) -> Json<Value> {
    let server_id = params["serverId"]
        .as_str()
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(0);

    let channels = sqlx::query_as::<_, Channel>("SELECT * FROM channels WHERE server_id = ?")
        .bind(server_id)
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();

    let result: Vec<Value> = channels
        .iter()
        .map(|c| json!({ "id": c.id, "name": c.name, "serverId": server_id }))
        .collect();

    Json(json!(result))
}

async fn create_channel(
    State(state): State<Arc<AppState>>,
    Query(params): Query<Value>,
    Json(payload): Json<Value>,
) -> impl IntoResponse {
    let server_id = params["serverId"]
        .as_str()
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(0);

    let name = payload["name"].as_str().unwrap_or("new-channel");

    match sqlx::query("INSERT INTO channels (server_id, name) VALUES (?, ?)")
        .bind(server_id)
        .bind(name)
        .execute(&state.db)
        .await
    {
        Ok(r) => (
            StatusCode::OK,
            Json(json!({ "id": r.last_insert_rowid(), "name": name, "serverId": server_id })),
        )
            .into_response(),
        Err(_) => (StatusCode::NOT_FOUND, Json(json!({ "error": "Server not found" }))).into_response(),
    }
}

async fn post_message(
    State(state): State<Arc<AppState>>,
    Query(params): Query<Value>,
    Json(payload): Json<Value>,
) -> Json<Value> {
    let channel_id = params["channelId"]
        .as_str()
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(0);
    let content = payload["content"].as_str().unwrap_or("");
    let author_did = payload["authorDid"].as_str().unwrap_or("did:activitypub:anon");
    let author_handle = payload["authorHandle"].as_str().unwrap_or(author_did);

    let id = sqlx::query(
        "INSERT INTO messages (channel_id, author_did, author_handle, content) VALUES (?, ?, ?, ?)",
    )
    .bind(channel_id)
    .bind(author_did)
    .bind(author_handle)
    .bind(content)
    .execute(&state.db)
    .await
    .map(|r| r.last_insert_rowid())
    .unwrap_or(0);

    Json(json!({
        "id": id,
        "content": content,
        "authorDid": author_did,
        "authorHandle": author_handle
    }))
}

async fn get_messages(
    State(state): State<Arc<AppState>>,
    Query(params): Query<Value>,
) -> Json<Value> {
    let channel_id = params["channelId"]
        .as_str()
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(0);

    let messages = sqlx::query_as::<_, Message>(
        "SELECT * FROM messages WHERE channel_id = ? ORDER BY created_at ASC",
    )
    .bind(channel_id)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    Json(json!(messages))
}

// ─── Convo (DM) Endpoints ────────────────────────────────────────────────────

async fn list_convos(State(state): State<Arc<AppState>>) -> Json<Value> {
    let convos = sqlx::query_as::<_, crate::models::Conversation>("SELECT * FROM conversations")
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();
    Json(json!({ "convos": convos }))
}

async fn get_convo(
    State(state): State<Arc<AppState>>,
    Query(params): Query<Value>,
) -> impl IntoResponse {
    let convo_id = params["convoId"]
        .as_str()
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(0);

    match sqlx::query_as::<_, crate::models::Conversation>(
        "SELECT * FROM conversations WHERE id = ?",
    )
    .bind(convo_id)
    .fetch_one(&state.db)
    .await
    {
        Ok(c) => (StatusCode::OK, Json(json!(c))).into_response(),
        Err(_) => (StatusCode::NOT_FOUND, Json(json!({ "error": "Not found" }))).into_response(),
    }
}

async fn get_convo_messages(
    State(state): State<Arc<AppState>>,
    Query(params): Query<Value>,
) -> Json<Value> {
    let convo_id = params["convoId"]
        .as_str()
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(0);

    let messages = sqlx::query_as::<_, crate::models::ConversationMessage>(
        "SELECT * FROM conversation_messages WHERE conversation_id = ? ORDER BY created_at ASC",
    )
    .bind(convo_id)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    Json(json!({ "messages": messages }))
}

async fn send_convo_message(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<Value>,
) -> Json<Value> {
    let content = payload["content"].as_str().unwrap_or("");
    let author_did = payload["authorDid"].as_str().unwrap_or("did:activitypub:anon");
    let author_handle = payload["authorHandle"].as_str().unwrap_or(author_did);

    // Get or create conversation
    let final_convo_id: i64 = if let Some(id) = payload["convoId"].as_i64() {
        id
    } else {
        // Create new conversation
        let id = sqlx::query("INSERT INTO conversations DEFAULT VALUES")
            .execute(&state.db)
            .await
            .map(|r| r.last_insert_rowid())
            .unwrap_or(0);

        // Add participants (sender + any listed members)
        let mut participants = vec![author_did.to_string()];
        if let Some(members) = payload["members"].as_array() {
            for m in members {
                if let Some(did) = m.as_str() {
                    if did != author_did {
                        participants.push(did.to_string());
                    }
                }
            }
        }
        for did in &participants {
            let _ = sqlx::query(
                "INSERT INTO conversation_participants (conversation_id, did, handle) VALUES (?, ?, ?)",
            )
            .bind(id)
            .bind(did)
            .bind(did)
            .execute(&state.db)
            .await;
        }
        id
    };

    let msg_id = sqlx::query(
        "INSERT INTO conversation_messages (conversation_id, author_did, author_handle, content) VALUES (?, ?, ?, ?)",
    )
    .bind(final_convo_id)
    .bind(author_did)
    .bind(author_handle)
    .bind(content)
    .execute(&state.db)
    .await
    .map(|r| r.last_insert_rowid())
    .unwrap_or(0);

    Json(json!({
        "id": msg_id,
        "content": content,
        "authorDid": author_did,
        "authorHandle": author_handle,
        "convoId": final_convo_id
    }))
}
