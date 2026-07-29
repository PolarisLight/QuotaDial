use super::protocol::RpcPeer;
use crate::error::AppError;
use std::{ffi::OsString, path::PathBuf, process::Stdio};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::{Child, Command},
};

pub async fn spawn_codex_app_server() -> Result<(RpcPeer, Child), AppError> {
    let mut last_error = None;
    let mut child = None;
    for executable in codex_candidates() {
        match Command::new(&executable)
            .args(["app-server", "--listen", "stdio://"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
        {
            Ok(process) => {
                child = Some(process);
                break;
            }
            Err(error) => last_error = Some(error),
        }
    }
    let mut child = child.ok_or_else(|| {
        AppError::Spawn(last_error.unwrap_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::NotFound, "Codex CLI was not found")
        }))
    })?;

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

fn codex_candidates() -> Vec<OsString> {
    let mut candidates = Vec::new();
    if let Some(path) = std::env::var_os("CODEX_MONITOR_CODEX_PATH") {
        candidates.push(path);
    }
    candidates.push(OsString::from("codex"));

    #[cfg(target_os = "macos")]
    {
        candidates.push(OsString::from(
            "/Applications/ChatGPT.app/Contents/Resources/codex",
        ));
        if let Some(home) = std::env::var_os("HOME") {
            candidates.push(
                PathBuf::from(home)
                    .join(".local/bin/codex")
                    .into_os_string(),
            );
        }
    }

    candidates
}
