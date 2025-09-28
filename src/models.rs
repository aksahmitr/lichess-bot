#![allow(dead_code)]
use serde::Deserialize;

#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
pub enum GlobalStreamEvent {
    #[serde(rename = "challenge")]
    Challenge { challenge: ChallengeEvent },
    #[serde(rename = "gameStart")]
    GameStart { game: GameStartEvent },
    #[serde(rename = "gameFinish")]
    GameFinish { game: GameFinishEvent },
}

#[derive(Deserialize, Debug)]
pub struct User {
    pub id: String,
    pub rating: u32,
    #[serde(alias = "name")]
    pub username: String,
    pub title: Option<String>,
    pub provisional: Option<bool>,
    pub online: Option<bool>,
    pub lag: Option<u32>,
}

#[derive(Deserialize, Debug)]
pub struct Variant {
    pub key: String,
    pub name: String,
    pub short: String,
}

#[derive(Deserialize, Debug)]
pub struct TimeControl {
    #[serde(rename = "type")]
    pub time_type: String,
    pub limit: Option<u32>,
    pub increment: Option<u32>,
    pub show: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct Perf {
    pub icon: Option<String>,
    pub name: String,
}

#[derive(Deserialize, Debug)]
pub struct ChallengeEvent {
    pub id: String,
    pub url: String,
    pub status: String,
    pub challenger: User,
    #[serde(rename = "destUser")]
    pub dest_user: User,
    pub variant: Variant,
    pub rated: bool,
    pub speed: String,
    #[serde(rename = "timeControl")]
    pub time_control: TimeControl,
    pub color: String,
    #[serde(rename = "finalColor")]
    pub final_color: String,
    pub perf: Perf,
    pub direction: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct GameStartEvent {
    #[serde(rename = "gameId")]
    pub game_id: String,
    #[serde(rename = "isMyTurn")]
    pub is_my_turn: bool,
    pub opponent: User,
}

#[derive(Deserialize, Debug)]
pub struct GameFinishEvent {
    #[serde(rename = "gameId")]
    pub game_id: String,
    #[serde(rename = "isMyTurn")]
    pub is_my_turn: bool,
    pub opponent: User,
}

#[derive(Deserialize, Debug)]
pub struct Clock {
    pub initial: u64,
    pub increment: u64,
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
pub enum GameStreamEvent {
    #[serde(rename = "gameFull")]
    GameFull(GameFullEvent),
    #[serde(rename = "gameState")]
    GameState(GameStateEvent),
    #[serde(rename = "chatLine")]
    ChatLine(ChatLineEvent),
}

#[derive(Deserialize, Debug)]
pub struct ChatLineEvent {
    username: String,
    text: String,
    room: String,
}

#[derive(Deserialize, Debug)]
pub struct GameFullEvent {
    pub id: String,
    pub variant: Variant,
    pub speed: String,
    pub perf: Perf,
    pub rated: bool,
    #[serde(rename = "createdAt")]
    pub created_at: u64,
    pub white: User,
    pub black: User,
    #[serde(rename = "initialFen")]
    pub initial_fen: String,
    pub clock: Option<Clock>,
    pub state: GameStateEvent,
    #[serde(rename = "daysPerTurn")]
    pub days_per_turn: Option<u64>,
    #[serde(rename = "tournamentId")]
    pub tournament_id: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct GameStateEvent {
    pub moves: String,
    pub wtime: u64,
    pub btime: u64,
    pub winc: u64,
    pub binc: u64,
    pub status: String,
    pub winner: Option<String>,
    pub wdraw: Option<bool>,
    pub bdraw: Option<bool>,
    pub wtakeback: Option<bool>,
    pub btakeback: Option<bool>,
}
