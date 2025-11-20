use axum::{routing::get, Router};
use tokio::sync::mpsc;
use std::net::SocketAddr;
use serde::{Deserialize, Serialize};
use chrono;


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

#[tokio::main]
async fn main() {
    
    let app = Router::new().route("/event", get(root));


    let address = SocketAddr::from(([0,0,0,0], 3000));
    let listener = tokio::net::TcpListener::bind(address).await.unwrap();
    axum::serve(listener, router).await.unwrap();

}

async fn event_handler() // axum automatically fills the parameters
