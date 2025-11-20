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
    
    let app = Router::new().route("/event", get(root));


    let address = SocketAddr::from(([0,0,0,0], 3000));
    let listener = tokio::net::TcpListener::bind(address).await.unwrap();

    // we need to make a channel to send events toe 
    axum::serve(listener, router).await.unwrap();

}

async fn event_handler(
    axum::extract::State(state): axum::extract::State<AppState>,
    Query(params): Query<analyticsParams>,
    headers: HeaderMap,
    SecureClientIp(ip): SecureClientIp,

) {} // axum automatically fills the parameters
