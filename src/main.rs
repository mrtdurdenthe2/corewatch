use axum::{
    extract::Query,
    http::{HeaderMap, StatusCode},
    routing::get,
    Router,
};

use tokio::sync::mpsc;
use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::env;
use serde::{Deserialize, Serialize};
use chrono;
use axum_client_ip::{SecureClientIp, SecureClientIpSource};
use convex::{ConvexClient, Value, FunctionResult};


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
    user_agent: Option<String>,
    ip: String,

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

    // One Convex client connection shared by the background worker.
    let convex_client = ConvexClient::new(&deployment_url).await?;

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
        let mut client = convex_client;
        while let Some(event) = rx.recv().await {
            // Persist to Convex via mutation: "events:addEvent"
            let mut args: BTreeMap<String, Value> = BTreeMap::new();
            args.insert("event".to_string(), event.params.event.clone().into());
            if let Some(url) = event.params.url.clone() {
                args.insert("url".to_string(), url.into());
            }
            if let Some(referrer) = event.params.referrer.clone() {
                args.insert("referrer".to_string(), referrer.into());
            }
            if let Some(ua) = event.user_agent.clone() {
                args.insert("userAgent".to_string(), ua.into());
            }
            args.insert("ip".to_string(), event.ip.clone().into());

            // Basic retry to smooth over transient network hiccups.
            let mut attempt: u32 = 0;
            loop {
                attempt += 1;
                match client.mutation("events:addEvent", args.clone()).await {
                    Ok(FunctionResult::Value(v)) => {
                        tracing::info!("PERSISTED EVENT: {:?}", v);
                        break;
                    }
                    Ok(other) => {
                        tracing::warn!("Convex returned non-value result: {:?}", other);
                        break;
                    }
                    Err(e) => {
                        if attempt >= 3 {
                            tracing::warn!("Failed to persist event after {attempt} attempts: {e}");
                            break;
                        }
                        // 50ms, 150ms
                        let backoff_ms = 50u64 * (attempt as u64) * (attempt as u64);
                        tokio::time::sleep(std::time::Duration::from_millis(backoff_ms)).await;
                    }
                }
            }
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

    let user_agent = headers
        .get(axum::http::header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    // Construct the event payload
    let event = eventPacket {
        params,
        timestamp: chrono::Utc::now(),
        user_agent,
        ip: _ip.to_string(),
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
