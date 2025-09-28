use anyhow::Result;

use lichess_bot::setup_event_stream;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    setup_event_stream().await?;
    Ok(())
}
