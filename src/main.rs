use axum::{
    extract::Query,
    http::{HeaderMap, StatusCode},
    routing::get,
    Router,
};

use tokio::sync::mpsc;
use std::net::SocketAddr;
use std::env;
use serde::{Deserialize, Serialize};
use chrono;
use axum_client_ip::{SecureClientIp, SecureClientIpSource};
use convex::ConvexClient;


#[derive(Serialize, Deserialize, Debug, Clone)]
struct analyticsParams {
    event: String, // event type, so click, view etc.
    url: Option<String>, 
    referrer: Option<String>, 
}

#[derive(Debug, Clone)]
struct eventPacket {
    params: analyticsParams,
    timestamp: chrono::DateTime<chrono::Utc>,

}

#[derive(Clone)]
struct AppState {
    sender: mpsc::Sender<eventPacket>,
    ingest_secret: String,
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff: u8 = 0;
    for i in 0..a.len() {
        diff |= a[i] ^ b[i];
    }
    diff == 0
}

fn is_authorized(headers: &HeaderMap, secret: &str) -> bool {
    let Some(auth) = headers.get(axum::http::header::AUTHORIZATION) else {
        return false;
    };
    let Ok(auth_str) = auth.to_str() else {
        return false;
    };
    let Some(token) = auth_str.strip_prefix("Bearer ") else {
        return false;
    };
    constant_time_eq(token.as_bytes(), secret.as_bytes())
}



// structure of the server looks like this:
// ## open port that listens for ev      
//       | 
// ## queue for the BG worker
//      |
// ## BG worker that does the DB actions

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("Fatal error: {e}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    dotenvy::from_filename(".env.local").ok();
    dotenvy::dotenv().ok();

    let ingest_secret = env::var("COREWATCH_INGEST_SECRET")
        .map_err(|_| "COREWATCH_INGEST_SECRET is not set (required for /event auth)")?;

    let deployment_url = env::var("CONVEX_URL")
        .map_err(|_| "CONVEX_URL is not set")?;

    let _client = ConvexClient::new(&deployment_url).await?;

    // This is our queue
    let (tx, mut rx) = mpsc::channel::<eventPacket>(10_000);

    let state = AppState { sender: tx, ingest_secret };
    let app = Router::new().route("/event", get(event_handler))
        .layer(SecureClientIpSource::ConnectInfo.into_extension())
        .with_state(state);

    let address = SocketAddr::from(([0,0,0,0], 6767));
    let listener = tokio::net::TcpListener::bind(address).await?;

    tokio::spawn(async move {
        println!("Background worker started...");

        // Ideally, you would batch these (e.g., wait for 100 items or 1 second)
        // For simplicity, we process them one by one here.
        while let Some(event) = rx.recv().await {
            // SIMULATE DB WRITE
            // In production, use an async client (like sqlx or clickhouse-rs) here
            tracing::info!("PERSISTING EVENT: {:?}", event);
        }
    });

    axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>()).await?;
    Ok(())
}

async fn event_handler(
    axum::extract::State(state): axum::extract::State<AppState>,
    Query(params): Query<analyticsParams>,
    headers: HeaderMap,
    SecureClientIp(_ip): SecureClientIp,

) -> StatusCode {
    // Shared-secret auth (expects: Authorization: Bearer <COREWATCH_INGEST_SECRET>)
    if !is_authorized(&headers, &state.ingest_secret) {
        return StatusCode::UNAUTHORIZED;
    }

    let _user_agent = headers
        .get(axum::http::header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    // Construct the event payload
    let event = eventPacket {
        params,
        timestamp: chrono::Utc::now(),
    };

    // Send to the background worker
    // try_send is non-blocking. If the channel is full, it will fail immediately 
    // (preventing the server from hanging under massive load).
    match state.sender.try_send(event) {
        Ok(_) => {
            // Success: Return 200 OK immediately
            StatusCode::OK 
        }
        Err(_e) => {
            // Queue is full: Return 503 or just drop it to save the server
            tracing::warn!("Queue full, dropping analytics event");
            StatusCode::SERVICE_UNAVAILABLE
        }
    }
} // axum automatically fills the parameters
