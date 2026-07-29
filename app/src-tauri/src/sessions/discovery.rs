use crate::error::AppError;
use std::path::{Path, PathBuf};

pub fn codex_home() -> Result<PathBuf, AppError> {
    if let Some(path) = std::env::var_os("CODEX_HOME") {
        return Ok(PathBuf::from(path));
    }
    dirs::home_dir()
        .map(|path| path.join(".codex"))
        .ok_or_else(|| AppError::Unavailable("unable to resolve Codex home".into()))
}

pub fn discover_jsonl(codex_home: &Path) -> Result<Vec<PathBuf>, AppError> {
    let canonical_home = match codex_home.canonicalize() {
        Ok(path) => path,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error.into()),
    };
    let mut files = Vec::new();
    for directory in ["sessions", "archived_sessions"] {
        visit(&canonical_home, &canonical_home.join(directory), &mut files)?;
    }
    files.sort();
    files.dedup();
    Ok(files)
}

fn visit(root: &Path, directory: &Path, files: &mut Vec<PathBuf>) -> Result<(), AppError> {
    let entries = match std::fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        let metadata = std::fs::symlink_metadata(&path)?;
        if metadata.file_type().is_symlink() {
            let target = match path.canonicalize() {
                Ok(target) => target,
                Err(_) => continue,
            };
            if !target.starts_with(root) {
                continue;
            }
            if target.is_dir() {
                visit(root, &target, files)?;
            } else if target.extension().and_then(|value| value.to_str()) == Some("jsonl") {
                files.push(target);
            }
        } else if metadata.is_dir() {
            visit(root, &path, files)?;
        } else if path.extension().and_then(|value| value.to_str()) == Some("jsonl") {
            files.push(path.canonicalize()?);
        }
    }
    Ok(())
}
