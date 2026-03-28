mod translator;
mod relay;
mod resolver;

use axum::{
    routing::{post, get},
    extract::Json,
    Router,
    http::StatusCode,
    response::IntoResponse,
};
use serde_json::{Value, json};
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use sqlx::sqlite::SqlitePoolOptions;
use std::sync::Arc;

pub struct AppState {
    pub db: sqlx::SqlitePool,
    pub relay: relay::Relay,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .init();

    // ── Database ──────────────────────────────────────────────────────────
    let db_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite:bridge.db?mode=rwc".to_string());

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await?;

    // Run schema migrations
    let schema = include_str!("../bridge_schema.sql");
    for stmt in schema.split(';') {
        let stmt = stmt.trim();
        if !stmt.is_empty() {
            sqlx::query(stmt).execute(&pool).await?;
        }
    }

    let state = Arc::new(AppState { 
        db: pool,
        relay: relay::Relay::new(),
    });

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/translate/atproto-to-ap", post(atproto_to_ap))
        .route("/translate/ap-to-atproto", post(ap_to_atproto))
        .route("/identity/map", post(map_identity))
        .route("/identity/resolve/:did", get(resolve_identity))
        .with_state(state)
        .layer(CorsLayer::permissive());

    let addr = SocketAddr::from(([0, 0, 0, 0], 8081));
    tracing::info!("Falcon Bridge service listening on http://{}", addr);
    
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn health_check() -> impl IntoResponse {
    StatusCode::OK
}

async fn atproto_to_ap(
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
    Json(payload): Json<Value>,
) -> impl IntoResponse {
    let translated = translator::Translator::atproto_to_activitypub(&payload);
    
    // In a live version, we'd look up the target inbox URL.
    // For the demo, we'll simulate resolving the target DID and sending.
    if let Some(did) = payload["targetDid"].as_str() {
        if let Ok(metadata) = resolver::Resolver::resolve(did).await {
            tracing::info!("Resolved target DID {} to metadata", did);
            
            // Load signing key from environment
            let signing_key_pem = std::env::var("BRIDGE_SIGNING_KEY")
                .unwrap_or_else(|_| "DUMMY_KEY_FOR_LOCAL_DEV_ONLY".to_string());
            
            let key_id = "http://localhost:8081/actor/bridge#main-key";
            let target_inbox = "http://localhost:8080/inbox";

            if signing_key_pem != "DUMMY_KEY_FOR_LOCAL_DEV_ONLY" {
                let _ = state.relay.send_activitypub(
                    target_inbox,
                    translated.clone(),
                    &signing_key_pem,
                    key_id
                ).await;
            } else {
                tracing::warn!("No BRIDGE_SIGNING_KEY found, skipping live relay signing");
            }
        }
    }

    Json(translated)
}

async fn ap_to_atproto(Json(payload): Json<Value>) -> impl IntoResponse {
    match translator::Translator::activitypub_to_atproto(&payload) {
        Some(translated) => (StatusCode::OK, Json(translated)).into_response(),
        None => (StatusCode::BAD_REQUEST, Json(json!({ "error": "Invalid ActivityPub activity" }))).into_response(),
    }
}

async fn map_identity(
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
    Json(payload): Json<Value>,
) -> impl IntoResponse {
    let did = payload["did"].as_str().unwrap_or("");
    let actor_uri = payload["actorUri"].as_str().unwrap_or("");
    let protocol = payload["protocol"].as_str().unwrap_or("unknown");

    match sqlx::query(
        "INSERT OR REPLACE INTO protocol_mappings (did, actor_uri, protocol) VALUES (?, ?, ?)"
    )
    .bind(did)
    .bind(actor_uri)
    .bind(protocol)
    .execute(&state.db)
    .await {
        Ok(_) => StatusCode::OK,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

async fn resolve_identity(
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
    axum::extract::Path(did): axum::extract::Path<String>,
) -> impl IntoResponse {
    match sqlx::query_scalar::<_, String>(
        "SELECT actor_uri FROM protocol_mappings WHERE did = ?"
    )
    .bind(did)
    .fetch_optional(&state.db)
    .await {
        Ok(Some(uri)) => (StatusCode::OK, Json(json!({ "actorUri": uri }))).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, Json(json!({ "error": "Not found" }))).into_response(),
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "DB error" }))).into_response(),
    }
}
