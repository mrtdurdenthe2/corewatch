use axum::{
    extract::{Query, Request},
    http::{HeaderMap, StatusCode},
    routing::get,
    Router,
};

use tokio::sync::mpsc;
use std::net::SocketAddr;
use serde::{Deserialize, Serialize};
use chrono;
use axum_client_ip::{SecureClientIp, SecureClientIpSource};


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
}



// structure of the server looks like this:
// ## open port that listens for ev      
//       | 
// ## queue for the BG worker
//      |
// ## BG worker that does the DB actions

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    dotenvy::from_filename(".env.local").ok();
    dotenvy::dotenv().ok();

    let deployment_url = env::var("CONVEX_URL").unwrap();

    let mut client = ConvexClient::new(&deployment_url).await.unwrap();

    // This is our queue
    let (tx, mut rx) = mpsc::channel::<eventPacket>(10_000);

    let state = AppState { sender: tx };
    let app = Router::new().route("/event", get(event_handler))
    .layer(SecureClientIpSource::ConnectInfo.into_extension())
    .with_state(state);


    let address = SocketAddr::from(([0,0,0,0], 3000));
    let listener = tokio::net::TcpListener::bind(address).await.unwrap();

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

    // we need to make a channel to send events toe 
    axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>()).await.unwrap();

}

async fn event_handler(
    axum::extract::State(state): axum::extract::State<AppState>,
    Query(params): Query<analyticsParams>,
    headers: HeaderMap,
    SecureClientIp(ip): SecureClientIp,

) -> StatusCode {

    let user_agent = headers
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
