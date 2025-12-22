use std::time::Duration;

#[tokio::main]
async fn main() {
    let client = reqwest::Client::new();
    let base_url = "http://127.0.0.1:3000/event";

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

        match client.get(&url).send().await {
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
        tokio::time::sleep(Duration::from_millis(2)).await;
    }
}

