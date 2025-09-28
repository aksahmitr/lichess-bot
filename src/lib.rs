use anyhow::{anyhow, Context, Result};
use futures_util::StreamExt;
use lazy_static::lazy_static;
use models::{GameStreamEvent, GlobalStreamEvent};
use reqwest::Client;
use std::time::Duration;
use tokio::time::sleep;
use uci::Engine;

mod models;
mod uci;

const RECONNECT_TIME: u64 = 5;

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
    static ref CLIENT: Client = create_client().expect("Failed to create HTTP client");
}

async fn send_move(game_id: &String, bestmove: &String, retries: u8, delay_ms: u64) -> Result<()> {
    for attempt in 0..retries {
        match CLIENT
            .post(format!(
                "https://lichess.org/api/bot/game/{}/move/{}?offeringDraw=false",
                game_id, bestmove
            ))
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => {
                return Ok(());
            }
            Ok(resp) => {
                eprintln!(
                    "invalid move {} for game {}: {:?}",
                    bestmove,
                    game_id,
                    resp.status()
                );
            }
            Err(e) => {
                eprintln!(
                    "error sending move {} for game {}: {}",
                    bestmove, game_id, e
                );
            }
        }

        eprintln!(
            "Retrying move {} for game {} (attempt {}/{})...",
            bestmove,
            game_id,
            attempt + 1,
            retries
        );
        sleep(Duration::from_millis(delay_ms)).await;
    }
    Err(anyhow!(
        "Giving up sending move {} for game {} after {} attempts",
        bestmove,
        game_id,
        retries
    ))
}

async fn setup_game_stream(game_id: String, is_my_turn: bool) -> Result<()> {
    eprintln!("Starting game stream for {}", game_id);

    let resp = CLIENT
        .get(format!(
            "https://lichess.org/api/bot/game/stream/{}",
            game_id
        ))
        .send()
        .await
        .context("game stream request failed")?;

    let mut turn = is_my_turn;

    let path = std::env::var("ENGINE").context("ENGINE not found")?;

    let mut engine = Engine::launch(&path).await?;
    engine.init_uci().await?;

    let mut stream = resp.bytes_stream();
    let mut buf = String::new();

    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(chunk) => {
                let s = std::str::from_utf8(&chunk).unwrap_or("");
                buf.push_str(s);

                while let Some(pos) = buf.find('\n') {
                    let line = buf[..pos].trim().to_string();
                    buf.drain(..=pos);

                    if line.is_empty() {
                        continue;
                    }

                    match serde_json::from_str::<GameStreamEvent>(&line) {
                        Ok(event) => match event {
                            GameStreamEvent::GameFull(full) => {
                                let state = &full.state;

                                engine.set_initial_pos(full.initial_fen);

                                if turn && state.status == "started" {
                                    match engine.handle_game_state(state).await {
                                        Ok(bestmove) => {
                                            send_move(&game_id, &bestmove, 5, 500).await?;
                                        }
                                        Err(e) => {
                                            eprintln!("Engine error: {}", e);
                                        }
                                    }
                                }
                                turn = !turn;
                            }

                            GameStreamEvent::GameState(state) => {
                                if turn && state.status == "started" {
                                    match engine.handle_game_state(&state).await {
                                        Ok(bestmove) => {
                                            send_move(&game_id, &bestmove, 5, 500).await?;
                                        }
                                        Err(e) => {
                                            eprintln!("Engine error: {}", e);
                                        }
                                    }
                                }
                                turn = !turn;
                            }

                            GameStreamEvent::ChatLine(_) => {
                                //ignore
                            }
                        },

                        Err(err) => {
                            eprintln!(
                                "failed to parse GameStreamEvent for game {}: {}\nraw line: {}",
                                game_id, err, line
                            );
                        }
                    }
                }
            }
            Err(err) => {
                eprintln!("game stream error for {}: {}", game_id, err);
                break;
            }
        }
    }
    eprintln!("Game stream for {} ended", game_id);

    engine.kill().await?;

    Ok(())
}

pub async fn setup_event_stream() -> Result<()> {
    loop {
        let res = CLIENT
            .get("https://lichess.org/api/stream/event")
            .send()
            .await;

        match res {
            Ok(r) => {
                eprintln!("started event stream");

                let mut stream = r.bytes_stream();
                let mut buf = String::new();

                while let Some(chunk) = stream.next().await {
                    match chunk {
                        Ok(chunk) => {
                            let s = std::str::from_utf8(&chunk).unwrap_or("");
                            buf.push_str(s);

                            while let Some(pos) = buf.find('\n') {
                                let line = buf[..pos].trim().to_string();
                                buf.drain(..=pos);

                                if line.is_empty() {
                                    continue;
                                }

                                match serde_json::from_str::<GlobalStreamEvent>(&line) {
                                    Ok(event) => match event {
                                        GlobalStreamEvent::Challenge { challenge } => {
                                            let _ = CLIENT
                                                .post(format!(
                                                    "https://lichess.org/api/challenge/{}/accept",
                                                    challenge.id
                                                ))
                                                .send()
                                                .await;
                                            println!(
                                                "accepted Challenge from {}; url: {}",
                                                challenge.challenger.id, challenge.url
                                            );
                                        }
                                        GlobalStreamEvent::GameStart { game } => {
                                            println!(
                                                "received gameStart from {}; id: {}",
                                                game.opponent.id, game.game_id
                                            );
                                            let game_id = game.game_id.clone();
                                            tokio::spawn(async move {
                                                if let Err(e) =
                                                    setup_game_stream(game_id, game.is_my_turn)
                                                        .await
                                                {
                                                    eprintln!(
                                                        "game stream task ended with error: {}",
                                                        e
                                                    );
                                                }
                                            });
                                        }
                                        GlobalStreamEvent::GameFinish { game } => {
                                            println!(
                                                "Received gameFinish from {}; id: {}",
                                                game.opponent.id, game.game_id
                                            );
                                        }
                                    },
                                    Err(e) => {
                                        eprintln!(
                                            "failed to GlobalStreamEvent : {} raw: {}",
                                            e, line
                                        );
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("event stream chunk error: {}", e);
                            break;
                        }
                    }
                }
                eprintln!("event stream ended, reconnecting...");
            }
            Err(e) => {
                eprintln!("failed to connect to event stream: {}", e);
            }
        }
        eprintln!("reconnecting in {}s...", RECONNECT_TIME);
        sleep(Duration::from_secs(RECONNECT_TIME)).await;
    }
}
