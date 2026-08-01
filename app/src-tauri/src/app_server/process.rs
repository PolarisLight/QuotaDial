use super::protocol::RpcPeer;
use crate::error::AppError;
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
    process::Stdio,
};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::{Child, Command},
};

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub async fn spawn_codex_app_server() -> Result<(RpcPeer, Child), AppError> {
    let mut failures = Vec::new();
    for executable in codex_candidates() {
        let mut command = Command::new(&executable);
        command
            .args(["app-server", "--listen", "stdio://"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        #[cfg(target_os = "windows")]
        command.creation_flags(CREATE_NO_WINDOW);

        let mut child = match command.spawn() {
            Ok(process) => process,
            Err(error) => {
                failures.push(format!("{}: {error}", executable.to_string_lossy()));
                continue;
            }
        };

        let Some(stdin) = child.stdin.take() else {
            failures.push(format!(
                "{}: missing stdin pipe",
                executable.to_string_lossy()
            ));
            let _ = child.kill().await;
            continue;
        };
        let Some(stdout) = child.stdout.take() else {
            failures.push(format!(
                "{}: missing stdout pipe",
                executable.to_string_lossy()
            ));
            let _ = child.kill().await;
            continue;
        };
        if let Some(stderr) = child.stderr.take() {
            tokio::spawn(async move {
                let mut lines = BufReader::new(stderr).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    log::warn!(target: "codex_app_server", "{line}");
                }
            });
        }

        let peer = RpcPeer::new(stdout, stdin);
        match peer.initialize().await {
            Ok(()) => return Ok((peer, child)),
            Err(error) => {
                failures.push(format!("{}: {error}", executable.to_string_lossy()));
                let _ = child.kill().await;
            }
        }
    }

    let detail = failures
        .last()
        .map(|failure| format!(" Last error: {failure}."))
        .unwrap_or_default();
    Err(AppError::Unavailable(format!(
        "Codex CLI was not found or could not start app-server.{detail} Set QUOTADIAL_CODEX_PATH to a working codex executable."
    )))
}

fn codex_candidates() -> Vec<OsString> {
    let mut candidates = Vec::new();
    if let Some(path) = std::env::var_os("QUOTADIAL_CODEX_PATH") {
        push_unique(&mut candidates, path);
    }
    if let Some(path) = std::env::var_os("CODEX_MONITOR_CODEX_PATH") {
        push_unique(&mut candidates, path);
    }
    push_unique(&mut candidates, "codex");

    #[cfg(target_os = "windows")]
    add_windows_candidates(&mut candidates);

    #[cfg(target_os = "macos")]
    {
        push_unique(
            &mut candidates,
            "/Applications/ChatGPT.app/Contents/Resources/codex",
        );
        if let Some(home) = std::env::var_os("HOME") {
            push_unique(
                &mut candidates,
                PathBuf::from(home)
                    .join(".local/bin/codex")
                    .into_os_string(),
            );
        }
    }

    candidates
}

fn push_unique(candidates: &mut Vec<OsString>, candidate: impl Into<OsString>) {
    let candidate = candidate.into();
    if !candidate.is_empty() && !candidates.iter().any(|item| item == &candidate) {
        candidates.push(candidate);
    }
}

#[cfg(target_os = "windows")]
fn add_windows_candidates(candidates: &mut Vec<OsString>) {
    push_unique(candidates, "codex.exe");
    push_unique(candidates, "codex.cmd");

    if let Some(local_app_data) = std::env::var_os("LOCALAPPDATA") {
        push_unique(
            candidates,
            PathBuf::from(local_app_data)
                .join("Microsoft/WindowsApps/codex.exe")
                .into_os_string(),
        );
    }
    if let Some(app_data) = std::env::var_os("APPDATA") {
        push_unique(
            candidates,
            PathBuf::from(app_data)
                .join("npm/codex.cmd")
                .into_os_string(),
        );
    }
    if let Some(user_profile) = std::env::var_os("USERPROFILE") {
        let user_profile = PathBuf::from(user_profile);
        push_unique(
            candidates,
            user_profile.join(".local/bin/codex.exe").into_os_string(),
        );
        for extensions_root in [
            user_profile.join(".vscode/extensions"),
            user_profile.join(".vscode-insiders/extensions"),
            user_profile.join(".cursor/extensions"),
            user_profile.join(".windsurf/extensions"),
        ] {
            for candidate in vscode_codex_candidates(&extensions_root, windows_codex_platform()) {
                push_unique(candidates, candidate);
            }
        }
    }
}

#[cfg(target_os = "windows")]
fn windows_codex_platform() -> &'static str {
    match std::env::consts::ARCH {
        "aarch64" => "windows-aarch64",
        _ => "windows-x86_64",
    }
}

fn vscode_codex_candidates(extensions_root: &Path, platform: &str) -> Vec<OsString> {
    let Ok(entries) = std::fs::read_dir(extensions_root) else {
        return Vec::new();
    };
    let mut extension_directories = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("openai.chatgpt-"))
        })
        .collect::<Vec<_>>();
    extension_directories.sort_by(|left, right| right.file_name().cmp(&left.file_name()));
    extension_directories
        .into_iter()
        .map(|directory| directory.join("bin").join(platform).join("codex.exe"))
        .filter(|path| path.is_file())
        .map(PathBuf::into_os_string)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn discovers_bundled_codex_from_newest_vscode_extension_first() {
        let directory = tempfile::tempdir().unwrap();
        for version in ["26.700.1", "26.721.9"] {
            let executable = directory
                .path()
                .join(format!("openai.chatgpt-{version}-win32-x64"))
                .join("bin/windows-x86_64/codex.exe");
            fs::create_dir_all(executable.parent().unwrap()).unwrap();
            fs::write(executable, []).unwrap();
        }
        fs::create_dir_all(directory.path().join("unrelated.extension-1.0.0")).unwrap();

        let candidates = vscode_codex_candidates(directory.path(), "windows-x86_64");

        assert_eq!(candidates.len(), 2);
        assert!(Path::new(&candidates[0])
            .to_string_lossy()
            .contains("26.721.9"));
        assert!(Path::new(&candidates[1])
            .to_string_lossy()
            .contains("26.700.1"));
    }

    #[test]
    fn skips_extension_entries_without_a_codex_binary() {
        let directory = tempfile::tempdir().unwrap();
        fs::create_dir_all(
            directory
                .path()
                .join("openai.chatgpt-26.721.9-win32-x64/bin/windows-x86_64"),
        )
        .unwrap();

        assert!(vscode_codex_candidates(directory.path(), "windows-x86_64").is_empty());
    }

    #[test]
    fn candidate_list_deduplicates_exact_paths() {
        let mut candidates = Vec::new();
        push_unique(&mut candidates, "codex");
        push_unique(&mut candidates, "codex");
        assert_eq!(candidates, vec![OsString::from("codex")]);
    }
}
