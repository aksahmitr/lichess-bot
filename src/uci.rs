use crate::models::GameStateEvent;
use anyhow::{anyhow, Context, Result};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::time::{timeout, Duration};

pub struct Engine {
    initial_pos: String,
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl Engine {
    pub async fn launch(path: &str) -> Result<Self> {
        let mut cmd = Command::new(path);
        cmd.stdin(Stdio::piped()).stdout(Stdio::piped());

        let mut child = cmd.spawn()?;
        let stdin = child.stdin.take().context("failed to open child stdin")?;
        let stdout = child.stdout.take().context("failed to open child stdout")?;

        Ok(Self {
            initial_pos: "startpos".to_string(),
            child,
            stdin,
            stdout: BufReader::new(stdout),
        })
    }

    pub async fn send_command(&mut self, cmd: &str) -> Result<()> {
        self.stdin.write_all(cmd.as_bytes()).await?;
        self.stdin.write_all(b"\n").await?;
        self.stdin.flush().await?;
        Ok(())
    }

    pub fn set_initial_pos(&mut self, pos: String) {
        self.initial_pos = pos;
    }

    pub async fn init_uci(&mut self) -> Result<()> {
        self.send_command("uci").await?;

        let mut line = String::new();

        timeout(Duration::from_secs(5), async {
            loop {
                line.clear();

                let n = self.stdout.read_line(&mut line).await?;
                if n == 0 {
                    return Err(anyhow!("engine closed before sending 'uciok'"));
                }

                let trimmed = line.trim();

                if trimmed == "uciok" {
                    return Ok(());
                }
            }
        })
        .await
        .context("timeout waiting for 'uciok'")??;

        Ok(())
    }

    async fn read_bestmove(&mut self) -> Result<String> {
        let mut line = String::new();

        loop {
            line.clear();

            let n = self.stdout.read_line(&mut line).await?;
            if n == 0 {
                break;
            }

            let trimmed = line.trim();

            if trimmed.starts_with("bestmove") {
                let mut parts = trimmed.split_whitespace();

                parts.next();

                if let Some(bestmove) = parts.next() {
                    return Ok(bestmove.to_string());
                } else {
                    return Err(anyhow::anyhow!("malformed bestmove line"));
                }
            }
        }

        Err(anyhow::anyhow!("engine closed before sending bestmove"))
    }

    pub async fn handle_game_state(&mut self, state: &GameStateEvent) -> Result<String> {
        let position_cmd = if state.moves.trim().is_empty() {
            format!("position {}", self.initial_pos)
        } else {
            format!("position {} moves {}", self.initial_pos, state.moves)
        };

        self.send_command(&position_cmd).await?;

        let go_cmd = format!(
            "go wtime {} btime {} winc {} binc {} movetime 1000",
            state.wtime, state.btime, state.winc, state.binc
        );

        self.send_command(&go_cmd).await?;
        self.read_bestmove().await
    }

    pub async fn kill(&mut self) -> Result<()> {
        self.child.kill().await.context("failed to kill process")?;
        Ok(())
    }
}
