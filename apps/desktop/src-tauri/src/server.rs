use crate::monitoring::CollectorState;
use axum::{
    extract::State,
    http::{header, HeaderValue, Method, StatusCode},
    response::{IntoResponse, Json},
    routing::get,
    Router,
};
use serde_json::json;
use std::sync::Arc;
use tower_http::cors::CorsLayer;

pub fn start_pairing_server(state: Arc<CollectorState>) {
    tauri::async_runtime::spawn(async move {
        let cors = CorsLayer::new()
            .allow_origin([
                "https://kernova.click".parse::<HeaderValue>().unwrap(),
                "http://localhost:1420".parse::<HeaderValue>().unwrap(),
                "http://localhost:5173".parse::<HeaderValue>().unwrap(),
            ])
            .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
            .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION]);

        let app = Router::new()
            .route("/api/v1/ping", get(ping_handler))
            .route("/api/v1/snapshot", get(snapshot_handler))
            .layer(cors)
            .with_state(state);

        let listener = match tokio::net::TcpListener::bind("127.0.0.1:45120").await {
            Ok(l) => l,
            Err(e) => {
                eprintln!("[TBK Core] Failed to bind local agent server on 127.0.0.1:45120: {}", e);
                return;
            }
        };

        println!("[TBK Core] Local pairing server listening on http://127.0.0.1:45120");
        let _ = axum::serve(listener, app).await;
    });
}

async fn ping_handler() -> impl IntoResponse {
    Json(json!({
        "status": "connected",
        "app": "TBK Core",
        "version": "0.1.0",
        "target": "https://kernova.click/"
    }))
}

async fn snapshot_handler(State(state): State<Arc<CollectorState>>) -> Result<impl IntoResponse, StatusCode> {
    let mut collector = state
        .collector
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let snapshot = collector.take_snapshot();
    Ok(Json(snapshot))
}
