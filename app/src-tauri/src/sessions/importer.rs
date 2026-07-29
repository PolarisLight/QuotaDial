use crate::{
    error::AppError,
    sessions::{
        discovery::discover_jsonl,
        parser::{parse_reader_with_context, ParseContext, PARSER_VERSION},
    },
    storage::repository::{AccountRepository, SourceFileState},
};
use std::{
    fs::File,
    io::{Seek, SeekFrom},
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ImportReport {
    pub scanned_files: i64,
    pub imported_events: i64,
    pub skipped_lines: i64,
    pub last_error: Option<String>,
}

pub struct SessionImporter<'a> {
    repository: &'a AccountRepository,
    codex_home: PathBuf,
}

impl<'a> SessionImporter<'a> {
    pub fn new(repository: &'a AccountRepository, codex_home: &Path) -> Self {
        Self {
            repository,
            codex_home: codex_home.to_owned(),
        }
    }

    pub fn reconcile(&self, _now: i64) -> Result<ImportReport, AppError> {
        let files = discover_jsonl(&self.codex_home)?;
        let mut report = ImportReport {
            scanned_files: files.len() as i64,
            ..ImportReport::default()
        };
        for path in files {
            match self.import_file(&path) {
                Ok(file_report) => {
                    report.imported_events += file_report.imported_events;
                    report.skipped_lines += file_report.skipped_lines;
                }
                Err(error) => {
                    report.last_error = Some(format!("{}: {error}", path.display()));
                }
            }
        }
        Ok(report)
    }

    fn import_file(&self, path: &Path) -> Result<ImportReport, AppError> {
        let path_string = path.to_string_lossy().into_owned();
        let metadata = std::fs::metadata(path)?;
        let size = i64::try_from(metadata.len())
            .map_err(|_| AppError::Unavailable("session file is too large".into()))?;
        let modified_at = system_time_nanos(metadata.modified().ok());
        let identity = file_identity(path, &metadata);
        let previous = self.repository.latest_source_state(&path_string)?;
        let reset = previous.as_ref().is_some_and(|state| {
            size < state.byte_offset
                || (state.file_identity.is_some()
                    && identity.is_some()
                    && state.file_identity != identity)
        });
        let parser_changed = previous
            .as_ref()
            .is_some_and(|state| state.parser_version != PARSER_VERSION);
        let generation = previous
            .as_ref()
            .map(|state| state.generation + i64::from(reset))
            .unwrap_or(0);
        let offset = previous
            .as_ref()
            .filter(|_| !reset && !parser_changed)
            .map(|state| state.byte_offset)
            .unwrap_or(0);
        let context = previous
            .as_ref()
            .filter(|_| !reset && !parser_changed)
            .map(|state| ParseContext {
                session_id: state.session_id.clone(),
                current_model: state.current_model.clone(),
            })
            .unwrap_or_default();

        if offset == size && !reset && !parser_changed {
            return Ok(ImportReport {
                scanned_files: 1,
                ..ImportReport::default()
            });
        }

        let mut file = File::open(path)?;
        file.seek(SeekFrom::Start(offset as u64))?;
        let parsed = parse_reader_with_context(&mut file, &path_string, offset as u64, context)?;
        let state = SourceFileState {
            path: path_string,
            generation,
            file_identity: identity,
            byte_offset: parsed.next_offset as i64,
            observed_size: size,
            modified_at,
            parser_version: PARSER_VERSION,
            session_id: parsed.current_session_id.clone(),
            current_model: parsed.current_model.clone(),
            last_error: None,
        };
        let inserted = self.repository.import_session_file(&state, &parsed)?;
        Ok(ImportReport {
            scanned_files: 1,
            imported_events: inserted,
            skipped_lines: parsed.skipped_lines,
            last_error: None,
        })
    }
}

fn system_time_nanos(value: Option<std::time::SystemTime>) -> i64 {
    value
        .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
        .map(|value| value.as_nanos().min(i64::MAX as u128) as i64)
        .unwrap_or_default()
}

fn file_identity(path: &Path, metadata: &std::fs::Metadata) -> Option<String> {
    let created = system_time_nanos(metadata.created().ok());
    if created > 0 {
        Some(created.to_string())
    } else {
        path.canonicalize()
            .ok()
            .map(|value| value.to_string_lossy().into_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::repository::AccountRepository;
    use std::{
        fs::{self, OpenOptions},
        io::Write,
        path::{Path, PathBuf},
    };
    use tempfile::TempDir;

    struct TestCodexHome {
        _directory: TempDir,
        codex_home: PathBuf,
        session_file: PathBuf,
    }

    impl TestCodexHome {
        fn with_root_fixture() -> Self {
            let directory = tempfile::tempdir().unwrap();
            let codex_home = directory.path().join(".codex");
            let session_directory = codex_home.join("sessions/2026/07/29");
            fs::create_dir_all(&session_directory).unwrap();
            fs::create_dir_all(codex_home.join("archived_sessions")).unwrap();
            let session_file = session_directory.join("root.jsonl");
            fs::write(
                &session_file,
                include_bytes!("../../tests/fixtures/sessions/root.jsonl"),
            )
            .unwrap();
            Self {
                _directory: directory,
                codex_home,
                session_file,
            }
        }

        fn path(&self) -> &Path {
            &self.codex_home
        }

        fn append_usage_event(&self, input_tokens: i64, output_tokens: i64) {
            let mut file = OpenOptions::new()
                .append(true)
                .open(&self.session_file)
                .unwrap();
            let event = serde_json::json!({
                "timestamp": "2026-07-29T08:04:00Z",
                "type": "event_msg",
                "payload": {
                    "type": "token_count",
                    "info": {
                        "last_token_usage": {
                            "input_tokens": input_tokens,
                            "cached_input_tokens": 0,
                            "output_tokens": output_tokens,
                            "reasoning_output_tokens": 0,
                            "total_tokens": input_tokens + output_tokens
                        },
                        "total_token_usage": {
                            "total_tokens": 1_200 + input_tokens + output_tokens
                        }
                    }
                }
            });
            writeln!(file, "{event}").unwrap();
        }

        fn truncate(&self) {
            fs::write(&self.session_file, []).unwrap();
        }

        fn rewrite_with_same_fixture(&self) {
            fs::write(
                &self.session_file,
                include_bytes!("../../tests/fixtures/sessions/root.jsonl"),
            )
            .unwrap();
        }
    }

    #[test]
    fn importing_the_same_file_twice_does_not_duplicate_usage() {
        let fixture = TestCodexHome::with_root_fixture();
        let repository = AccountRepository::open_in_memory().unwrap();
        let importer = SessionImporter::new(&repository, fixture.path());

        importer.reconcile(1_000).unwrap();
        importer.reconcile(2_000).unwrap();

        assert_eq!(repository.session_event_count().unwrap(), 1);
    }

    #[test]
    fn parser_version_change_replays_a_complete_file() {
        let fixture = TestCodexHome::with_root_fixture();
        let repository = AccountRepository::open_in_memory().unwrap();
        let path = fixture
            .session_file
            .canonicalize()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        let bytes = include_bytes!("../../tests/fixtures/sessions/root.jsonl");
        let parsed = crate::sessions::parser::parse_reader(bytes.as_slice(), &path, 0).unwrap();
        repository
            .import_session_file(
                &SourceFileState {
                    path: path.clone(),
                    generation: 0,
                    file_identity: None,
                    byte_offset: parsed.next_offset as i64,
                    observed_size: parsed.next_offset as i64,
                    modified_at: 0,
                    parser_version: PARSER_VERSION - 1,
                    session_id: parsed.current_session_id.clone(),
                    current_model: parsed.current_model.clone(),
                    last_error: None,
                },
                &parsed,
            )
            .unwrap();

        SessionImporter::new(&repository, fixture.path())
            .reconcile(2_000)
            .unwrap();

        let state = repository.latest_source_state(&path).unwrap().unwrap();
        assert_eq!(state.parser_version, PARSER_VERSION);
        assert_eq!(repository.session_event_count().unwrap(), 1);
    }

    #[test]
    fn appending_a_complete_line_imports_only_the_new_event() {
        let fixture = TestCodexHome::with_root_fixture();
        let repository = AccountRepository::open_in_memory().unwrap();
        let importer = SessionImporter::new(&repository, fixture.path());
        importer.reconcile(1_000).unwrap();
        fixture.append_usage_event(200, 40);

        importer.reconcile(2_000).unwrap();

        assert_eq!(repository.session_event_count().unwrap(), 2);
    }

    #[test]
    fn truncation_starts_a_new_generation_but_replay_stays_idempotent() {
        let fixture = TestCodexHome::with_root_fixture();
        let repository = AccountRepository::open_in_memory().unwrap();
        let importer = SessionImporter::new(&repository, fixture.path());
        importer.reconcile(1_000).unwrap();
        fixture.truncate();
        importer.reconcile(2_000).unwrap();
        fixture.rewrite_with_same_fixture();

        importer.reconcile(3_000).unwrap();

        assert_eq!(repository.session_event_count().unwrap(), 1);
        assert_eq!(repository.source_generation_count().unwrap(), 2);
    }
}
