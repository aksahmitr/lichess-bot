use anyhow::{Context, Result};
use futures_util::StreamExt;
use lazy_static::lazy_static;
use reqwest::Client;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
struct Event {
    #[serde(rename = "type")]
    event_type: String,
}

fn create_client() -> Result<Client> {
    let access_token = std::env::var("LICHESS_TOKEN").context("LICHESS_TOKEN not found")?;

    let client = reqwest::Client::builder()
        .user_agent("rust-lichess-bot")
        .default_headers({
            let mut headers = reqwest::header::HeaderMap::new();
            headers.insert(
                reqwest::header::AUTHORIZATION,
                reqwest::header::HeaderValue::from_str(&format!("Bearer {}", access_token))
                    .unwrap(),
            );
            headers
        })
        .build()?;
    Ok(client)
}

lazy_static! {
    static ref CLIENT: Client = create_client().unwrap();
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    let mut event_stream = CLIENT
        .get("https://lichess.org/api/stream/event")
        .send()
        .await?
        .bytes_stream();

    while let Some(item) = event_stream.next().await {
        let event_bytes = &item.unwrap();
        let event_str = std::str::from_utf8(event_bytes).unwrap();
        if event_str == "\n" {
            continue;
        }
        let event: Event = serde_json::from_str(event_str)?;
        println!("{:#?}", event);
    }
    //println!("{:#?}", res);
    Ok(())
}
