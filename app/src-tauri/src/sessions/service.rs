use crate::{
    domain::session::LocalSessionView, error::AppError, sessions::importer::SessionImporter,
    storage::repository::AccountRepository,
};
use notify::{RecursiveMode, Watcher};
use std::{path::PathBuf, sync::Arc, time::Duration};
use tokio::sync::{mpsc, watch, Mutex};

pub struct SessionService {
    repository: Arc<AccountRepository>,
    codex_home: PathBuf,
    snapshot: watch::Sender<LocalSessionView>,
    scan_lock: Mutex<()>,
}

impl SessionService {
    pub fn new(repository: Arc<AccountRepository>, codex_home: PathBuf) -> Self {
        let (snapshot, _) = watch::channel(LocalSessionView::default());
        Self {
            repository,
            codex_home,
            snapshot,
            scan_lock: Mutex::new(()),
        }
    }

    pub fn subscribe(&self) -> watch::Receiver<LocalSessionView> {
        self.snapshot.subscribe()
    }

    pub fn snapshot(&self) -> LocalSessionView {
        self.snapshot.borrow().clone()
    }

    pub async fn rescan(&self) -> Result<LocalSessionView, AppError> {
        let _guard = self.scan_lock.lock().await;
        let now = chrono::Utc::now().timestamp();
        let repository = self.repository.clone();
        let codex_home = self.codex_home.clone();
        let report = tokio::task::spawn_blocking(move || {
            SessionImporter::new(repository.as_ref(), &codex_home).reconcile(now)
        })
        .await
        .map_err(|error| AppError::Unavailable(format!("session scan task failed: {error}")))??;

        let mut view = self.repository.local_session_view(now)?;
        view.diagnostics.scanned_files = report.scanned_files;
        view.diagnostics.skipped_lines = report.skipped_lines;
        view.diagnostics.last_imported_at = Some(now);
        view.diagnostics.last_error = report.last_error;
        self.snapshot.send_replace(view.clone());
        Ok(view)
    }

    pub async fn run(self: Arc<Self>, shutdown: watch::Receiver<bool>) {
        let (_settings_runtime, settings) = {
            let runtime = crate::settings::SettingsRuntime::new(Default::default());
            let receiver = runtime.subscribe();
            (runtime, receiver)
        };
        self.run_with_settings(shutdown, settings).await;
    }

    pub async fn run_with_settings(
        self: Arc<Self>,
        mut shutdown: watch::Receiver<bool>,
        mut settings: watch::Receiver<crate::settings::AppSettings>,
    ) {
        let _ = self.rescan().await;
        let (event_sender, mut event_receiver) = mpsc::unbounded_channel();
        let mut watcher =
            notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
                let _ = event_sender.send(event);
            })
            .ok();
        if let Some(watcher) = watcher.as_mut() {
            for directory in [
                self.codex_home.join("sessions"),
                self.codex_home.join("archived_sessions"),
            ] {
                if directory.exists() {
                    let _ = watcher.watch(&directory, RecursiveMode::Recursive);
                }
            }
        }

        loop {
            let delay = tokio::time::sleep(settings.borrow().session_scan_duration());
            tokio::pin!(delay);
            tokio::select! {
                _ = &mut delay => {
                    let _ = self.rescan().await;
                }
                changed = settings.changed() => {
                    if changed.is_err() {
                        break;
                    }
                }
                event = event_receiver.recv() => {
                    if event.is_some() {
                        tokio::time::sleep(Duration::from_millis(250)).await;
                        while event_receiver.try_recv().is_ok() {}
                        let _ = self.rescan().await;
                    }
                }
                changed = shutdown.changed() => {
                    if changed.is_err() || *shutdown.borrow() {
                        break;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::repository::AccountRepository;
    use std::{fs, sync::Arc};

    #[tokio::test]
    async fn rescan_publishes_sessions_without_waiting_for_account_refresh() {
        let directory = tempfile::tempdir().unwrap();
        let codex_home = directory.path().join(".codex");
        let session_directory = codex_home.join("sessions/2026/07/29");
        fs::create_dir_all(&session_directory).unwrap();
        fs::write(
            session_directory.join("root.jsonl"),
            include_bytes!("../../tests/fixtures/sessions/root.jsonl"),
        )
        .unwrap();
        let repository = Arc::new(AccountRepository::open_in_memory().unwrap());
        let service = SessionService::new(repository, codex_home);
        let mut updates = service.subscribe();

        service.rescan().await.unwrap();
        updates.changed().await.unwrap();

        assert_eq!(updates.borrow().sessions.len(), 1);
    }
}
