use std::time::Duration;
use std::env;

#[tokio::main]
async fn main() {
    let client = reqwest::Client::new();
    let base_url = "http://127.0.0.1:6767/event";
    let secret = env::var("COREWATCH_INGEST_SECRET").unwrap_or_default();

    let events = ["click", "view", "scroll", "hover", "submit"];
    let mut counter = 0;

    println!("Starting event sender... (sending every 1 second)");
    println!("Press Ctrl+C to stop\n");

    loop {
        let event_type = events[counter % events.len()];
        
        let url = format!(
            "{}?event={}&url=https://example.com/page{}&referrer=https://google.com",
            base_url,
            event_type,
            counter
        );

        let req = client
            .get(&url)
            .header("authorization", format!("Bearer {}", secret));

        match req.send().await {
            Ok(response) => {
                println!(
                    "[{}] Sent event '{}' -> Status: {}",
                    counter + 1,
                    event_type,
                    response.status()
                );
            }
            Err(e) => {
                eprintln!("[{}] Failed to send event: {}", counter + 1, e);
            }
        }

        counter += 1;
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

