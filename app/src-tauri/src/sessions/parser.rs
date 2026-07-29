use crate::{domain::session::TokenBreakdown, error::AppError};
use chrono::{DateTime, Local, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{io::Read, path::Path};

pub const PARSER_VERSION: i64 = 1;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ParsedSessionMetadata {
    pub session_id: String,
    pub parent_session_id: Option<String>,
    pub title: String,
    pub started_at: i64,
    pub last_active_at: i64,
    pub cwd: Option<String>,
    pub model: Option<String>,
    pub source_path: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ParsedUsageEvent {
    pub fingerprint: String,
    pub session_id: String,
    pub occurred_at: i64,
    pub model: Option<String>,
    pub tokens: TokenBreakdown,
    pub source_path: String,
    pub source_offset: u64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ParsedFile {
    pub metadata: Option<ParsedSessionMetadata>,
    pub events: Vec<ParsedUsageEvent>,
    pub next_offset: u64,
    pub skipped_lines: i64,
}

#[derive(Debug, Deserialize)]
struct Usage {
    #[serde(default)]
    input_tokens: i64,
    #[serde(default)]
    cached_input_tokens: i64,
    #[serde(default)]
    output_tokens: i64,
    #[serde(default)]
    reasoning_output_tokens: i64,
    #[serde(default)]
    total_tokens: i64,
}

pub fn parse_reader(
    mut reader: impl Read,
    source_path: &str,
    start_offset: u64,
) -> Result<ParsedFile, AppError> {
    let mut bytes = Vec::new();
    reader.read_to_end(&mut bytes)?;

    let mut metadata: Option<ParsedSessionMetadata> = None;
    let mut events = Vec::new();
    let mut skipped_lines = 0;
    let mut current_model: Option<String> = None;
    let mut consumed = 0_u64;

    for segment in bytes.split_inclusive(|byte| *byte == b'\n') {
        if !segment.ends_with(b"\n") {
            break;
        }

        let line_offset = start_offset + consumed;
        consumed += segment.len() as u64;
        let line = match std::str::from_utf8(&segment[..segment.len() - 1]) {
            Ok(value) => value.trim_end_matches('\r'),
            Err(_) => {
                skipped_lines += 1;
                continue;
            }
        };
        if line.trim().is_empty() {
            continue;
        }

        let value: Value = match serde_json::from_str(line) {
            Ok(value) => value,
            Err(_) => {
                skipped_lines += 1;
                continue;
            }
        };
        let kind = value.get("type").and_then(Value::as_str);
        let payload = value.get("payload").unwrap_or(&Value::Null);
        let occurred_at = parse_timestamp(
            value
                .get("timestamp")
                .and_then(Value::as_str)
                .or_else(|| payload.get("timestamp").and_then(Value::as_str)),
        );

        match kind {
            Some("session_meta") => {
                let Some(session_id) = payload.get("id").and_then(Value::as_str) else {
                    skipped_lines += 1;
                    continue;
                };
                let cwd = payload
                    .get("cwd")
                    .and_then(Value::as_str)
                    .map(str::to_owned);
                let started_at = parse_timestamp(
                    payload
                        .get("timestamp")
                        .and_then(Value::as_str)
                        .or_else(|| value.get("timestamp").and_then(Value::as_str)),
                );
                metadata = Some(ParsedSessionMetadata {
                    session_id: session_id.to_owned(),
                    parent_session_id: payload
                        .pointer("/source/subagent/thread_spawn/parent_thread_id")
                        .and_then(Value::as_str)
                        .map(str::to_owned),
                    title: generated_title(cwd.as_deref(), started_at),
                    started_at,
                    last_active_at: started_at,
                    cwd,
                    model: current_model.clone(),
                    source_path: source_path.to_owned(),
                });
            }
            Some("turn_context") => {
                current_model = payload
                    .get("model")
                    .and_then(Value::as_str)
                    .map(str::to_owned);
                if let Some(metadata) = metadata.as_mut() {
                    metadata.model = current_model.clone();
                    metadata.last_active_at = metadata.last_active_at.max(occurred_at);
                }
            }
            Some("event_msg")
                if payload.get("type").and_then(Value::as_str) == Some("token_count") =>
            {
                let Some(session_id) = metadata.as_ref().map(|item| item.session_id.as_str())
                else {
                    skipped_lines += 1;
                    continue;
                };
                let Some(last_usage) = payload.pointer("/info/last_token_usage") else {
                    continue;
                };
                let usage: Usage = match serde_json::from_value(last_usage.clone()) {
                    Ok(usage) => usage,
                    Err(_) => {
                        skipped_lines += 1;
                        continue;
                    }
                };
                let tokens = TokenBreakdown {
                    input_tokens: usage.input_tokens.max(0),
                    cached_input_tokens: usage.cached_input_tokens.max(0),
                    output_tokens: usage.output_tokens.max(0),
                    reasoning_output_tokens: usage.reasoning_output_tokens.max(0),
                };
                let cumulative_total = payload
                    .pointer("/info/total_token_usage/total_tokens")
                    .and_then(Value::as_i64);
                events.push(ParsedUsageEvent {
                    fingerprint: fingerprint(
                        session_id,
                        current_model.as_deref(),
                        &tokens,
                        cumulative_total
                            .or(Some(usage.total_tokens))
                            .filter(|value| *value > 0),
                        occurred_at,
                    ),
                    session_id: session_id.to_owned(),
                    occurred_at,
                    model: current_model.clone(),
                    tokens,
                    source_path: source_path.to_owned(),
                    source_offset: line_offset,
                });
                if let Some(metadata) = metadata.as_mut() {
                    metadata.last_active_at = metadata.last_active_at.max(occurred_at);
                }
            }
            _ => {}
        }
    }

    Ok(ParsedFile {
        metadata,
        events,
        next_offset: start_offset + consumed,
        skipped_lines,
    })
}

fn parse_timestamp(value: Option<&str>) -> i64 {
    value
        .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
        .map(|value| value.timestamp())
        .unwrap_or_default()
}

fn generated_title(cwd: Option<&str>, started_at: i64) -> String {
    let project = cwd
        .and_then(|cwd| Path::new(cwd).file_name())
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("Codex");
    let date = DateTime::<Utc>::from_timestamp(started_at, 0)
        .map(|value| value.with_timezone(&Local).format("%-m月%-d日").to_string())
        .unwrap_or_else(|| "未知日期".to_owned());
    format!("{project} · {date}")
}

fn fingerprint(
    session_id: &str,
    model: Option<&str>,
    tokens: &TokenBreakdown,
    cumulative_total_tokens: Option<i64>,
    occurred_at: i64,
) -> String {
    let stable_counter = cumulative_total_tokens.unwrap_or(occurred_at);
    let value = format!(
        "{session_id}\0{}\0{}\0{}\0{}\0{}\0{stable_counter}",
        model.unwrap_or(""),
        tokens.input_tokens,
        tokens.cached_input_tokens,
        tokens.output_tokens,
        tokens.reasoning_output_tokens,
    );
    blake3::hash(value.as_bytes()).to_hex().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_last_usage_and_subagent_parent_without_message_text() {
        let parsed = parse_reader(
            include_bytes!("../../tests/fixtures/sessions/child.jsonl").as_slice(),
            "child.jsonl",
            0,
        )
        .unwrap();

        assert_eq!(
            parsed
                .metadata
                .as_ref()
                .unwrap()
                .parent_session_id
                .as_deref(),
            Some("root-1")
        );
        assert_eq!(parsed.events.len(), 1);
        assert_eq!(parsed.events[0].tokens.input_tokens, 300);
        assert_eq!(parsed.events[0].model.as_deref(), Some("gpt-5.6-codex"));
        assert!(!serde_json::to_string(&parsed)
            .unwrap()
            .contains("agent_nickname"));
    }

    #[test]
    fn skips_a_bad_line_and_continues_at_the_next_complete_line() {
        let parsed = parse_reader(
            include_bytes!("../../tests/fixtures/sessions/malformed.jsonl").as_slice(),
            "malformed.jsonl",
            0,
        )
        .unwrap();
        assert_eq!(parsed.skipped_lines, 1);
        assert_eq!(parsed.events.len(), 1);
    }
}
