use axum::{routing::get, Router};
use tokio::sync::mpsc;
use std::net::SocketAddr;
use serde::{Deserialize, Serialize};



#[derive(Serialize, Deserialize, Debug, Clone)]
struct analyticsParams {
    event: String, // event type, so click, view etc.
    url: Option<String>, 
    referrer: Option<String>, 
}
struct eventPacket {
    event_type: String,
    event_data: String,

}

#[tokio::main]
async fn main() {
    println!("Hello, world!");

    let address = SocketAddr::from(([0,0,0,0], 3000));

}
