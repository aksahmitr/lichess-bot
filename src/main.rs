use anyhow::{Context, Result};
use futures_util::StreamExt;
use lazy_static::lazy_static;
use reqwest::Client;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
struct Event {
    #[serde(rename = "type")]
    type_string: String,
    #[serde(flatten)]
    event_type: EventType,
}

#[derive(Deserialize, Debug)]
enum EventType {
    #[serde(rename = "challenge")]
    ChallengeEvent(ChallengeEvent),
    #[serde(rename = "game")]
    GameEvent(GameEvent),
}

#[derive(Deserialize, Debug)]
struct User {
    id: String,
    #[serde(alias = "username")]
    name: String,
    rating: u32,
    title: Option<String>,
    provisional: Option<bool>,
    online: Option<bool>,
    lag: Option<u32>,
}

#[derive(Deserialize, Debug)]
struct Variant {
    key: String,
    name: String,
    short: String,
}

#[derive(Deserialize, Debug)]
struct TimeControl {
    #[serde(rename = "type")]
    time_type: String,
    limit: Option<u32>,
    increment: Option<u32>,
    show: Option<String>,
}

#[derive(Deserialize, Debug)]
struct Perf {
    icon: String,
    name: String,
}

#[derive(Deserialize, Debug)]
struct ChallengeEvent {
    id: String,
    url: String,
    status: String,
    challenger: User,
    destUser: User,
    variant: Variant,
    rated: bool,
    speed: String,
    timeControl: TimeControl,
    color: String,
    finalColor: String,
    perf: Perf,
    direction: Option<String>,
}

#[derive(Deserialize, Debug)]
struct GameEvent {
    #[serde(rename = "gameId")]
    game_id: String,
    opponent: User,
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

async fn setup_event_stream() -> Result<()> {
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

        match event.event_type {
            EventType::ChallengeEvent(challenge) => {
                if challenge.status == "created" {
                    let _res = CLIENT
                        .post(format!(
                            "https://lichess.org/api/challenge/{}/accept",
                            challenge.id
                        ))
                        .send()
                        .await?;
                    println!(
                        "Accepted Challenge from {}; url: {}",
                        challenge.challenger.id, challenge.url
                    );
                } else {
                    let _res = CLIENT
                        .post(format!(
                            "https://lichess.org/api/challenge/{}/decline",
                            challenge.id
                        ))
                        .send()
                        .await?;
                    println!("Declined Challenge from {}", challenge.challenger.id);
                }
            }
            EventType::GameEvent(game) => {
                if event.type_string == "gameStart" {
                    println!(
                        "Received gameStart from {}; id: {}",
                        game.opponent.id, game.game_id
                    );
                } else {
                    //run game stream
                    println!(
                        "Received {} from {}; id: {}",
                        event.type_string, game.opponent.id, game.game_id
                    );
                }
            }
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    setup_event_stream().await?;
    Ok(())
}
