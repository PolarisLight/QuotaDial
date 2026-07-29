use super::protocol::RpcPeer;
use crate::error::AppError;
use std::process::Stdio;
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::{Child, Command},
};

pub async fn spawn_codex_app_server() -> Result<(RpcPeer, Child), AppError> {
    let mut child = Command::new("codex")
        .args(["app-server", "--listen", "stdio://"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(AppError::Spawn)?;

    let stdin = child.stdin.take().ok_or(AppError::MissingPipe("stdin"))?;
    let stdout = child.stdout.take().ok_or(AppError::MissingPipe("stdout"))?;
    if let Some(stderr) = child.stderr.take() {
        tokio::spawn(async move {
            let mut lines = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                log::warn!(target: "codex_app_server", "{line}");
            }
        });
    }

    let peer = RpcPeer::new(stdout, stdin);
    peer.initialize().await?;
    Ok((peer, child))
}
