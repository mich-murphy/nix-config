//! A readable digest of the coordinator's transcript for one time window.
//! The transcript is Claude Code's session JSONL, whose shape is not a
//! contract, so everything here is tolerant: unknown lines are skipped,
//! and a line without a timestamp is kept only when no window applies.

use serde_json::Value;

/// Per-entry text cap, and the cap for the whole digest.
const ENTRY_CAP: usize = 600;
const DIGEST_CAP: usize = 120_000;

pub(super) fn digest(text: &str, from: u64, to: u64) -> String {
    let mut lines = Vec::new();
    for line in text.lines() {
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        let at = value
            .get("timestamp")
            .and_then(Value::as_str)
            .and_then(parse_rfc3339);
        if at.is_some_and(|at| at < from || at > to) {
            continue;
        }
        lines.extend(render(&value));
    }
    if lines.is_empty() {
        return "(no transcript entries in the window)".into();
    }
    fit(lines.join("\n"), DIGEST_CAP)
}

fn render(value: &Value) -> Vec<String> {
    let kind = value.get("type").and_then(Value::as_str).unwrap_or("");
    if kind != "user" && kind != "assistant" {
        return Vec::new();
    }
    let stamp = value
        .get("timestamp")
        .and_then(Value::as_str)
        .unwrap_or("--");
    let side = if value.get("isSidechain").and_then(Value::as_bool) == Some(true) {
        " (subagent)"
    } else {
        ""
    };
    let prefix = format!("[{stamp}] {kind}{side}");
    match value.pointer("/message/content") {
        Some(Value::String(text)) => vec![format!("{prefix}: {}", clip(text, ENTRY_CAP))],
        Some(Value::Array(blocks)) => blocks
            .iter()
            .filter_map(|block| render_block(block).map(|text| format!("{prefix} {text}")))
            .collect(),
        _ => Vec::new(),
    }
}

fn render_block(block: &Value) -> Option<String> {
    let kind = block.get("type").and_then(Value::as_str)?;
    match kind {
        "text" => Some(format!(
            "text: {}",
            clip(block.get("text").and_then(Value::as_str)?, ENTRY_CAP)
        )),
        "tool_use" => {
            let name = block.get("name").and_then(Value::as_str).unwrap_or("?");
            let input = block.get("input").map(tool_input).unwrap_or_default();
            Some(format!("tool_use {name}: {}", clip(&input, ENTRY_CAP)))
        }
        "tool_result" => Some(format!(
            "tool_result: {}",
            clip(&tool_result(block.get("content")), ENTRY_CAP)
        )),
        _ => None,
    }
}

/// A shell command is the whole story of a `Bash` call; anything else is
/// shown as compact JSON.
fn tool_input(input: &Value) -> String {
    input
        .get("command")
        .and_then(Value::as_str)
        .map_or_else(|| input.to_string(), ToOwned::to_owned)
}

fn tool_result(content: Option<&Value>) -> String {
    match content {
        Some(Value::String(text)) => text.clone(),
        Some(Value::Array(parts)) => parts
            .iter()
            .filter_map(|part| part.get("text").and_then(Value::as_str))
            .collect::<Vec<_>>()
            .join(" "),
        _ => String::new(),
    }
}

pub(super) fn clip(text: &str, cap: usize) -> String {
    let flat = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.chars().count() <= cap {
        return flat;
    }
    let kept: String = flat.chars().take(cap).collect();
    format!("{kept}…")
}

/// Keeps the head and the tail when the digest is too long: friction
/// tends to sit at the start (setup) and the end (handoff) of a task.
fn fit(text: String, cap: usize) -> String {
    if text.chars().count() <= cap {
        return text;
    }
    let head: String = text.chars().take(cap / 3).collect();
    let tail: String = text
        .chars()
        .rev()
        .take(cap - cap / 3)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    format!("{head}\n…[transcript middle elided]…\n{tail}")
}

/// `YYYY-MM-DDTHH:MM:SS(.fraction)Z` to Unix seconds; anything else is
/// `None`. Days-from-civil, the inverse of the trace formatter.
pub(super) fn parse_rfc3339(text: &str) -> Option<u64> {
    let text = text.strip_suffix('Z')?;
    let (date, time) = text.split_once('T')?;
    let mut date = date.split('-').map(str::parse::<i64>);
    let (year, month, day) = (date.next()?.ok()?, date.next()?.ok()?, date.next()?.ok()?);
    let mut time = time.split(':');
    let hour: u64 = time.next()?.parse().ok()?;
    let minute: u64 = time.next()?.parse().ok()?;
    let second: u64 = time.next()?.split('.').next()?.parse().ok()?;
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    u64::try_from(days)
        .ok()?
        .checked_mul(86_400)?
        .checked_add(hour * 3_600 + minute * 60 + second)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_timestamps_both_ways() {
        assert_eq!(parse_rfc3339("1970-01-01T00:00:51Z"), Some(51));
        assert_eq!(
            parse_rfc3339("2026-09-09T22:38:00.366Z"),
            Some(1_788_993_480)
        );
        assert_eq!(parse_rfc3339("nonsense"), None);
    }

    #[test]
    fn digest_keeps_window_entries_and_summarises_blocks() {
        let text = concat!(
            r#"{"type":"user","timestamp":"1970-01-01T00:00:10Z","message":{"content":"please fix"}}"#,
            "\n",
            r#"{"type":"assistant","timestamp":"1970-01-01T00:00:20Z","message":{"content":[{"type":"tool_use","name":"Bash","input":{"command":"controller bind-slot GAIN-2"}}]}}"#,
            "\n",
            r#"{"type":"user","timestamp":"1970-01-01T00:00:21Z","isSidechain":true,"message":{"content":[{"type":"tool_result","content":[{"text":"slot occupied"}]}]}}"#,
            "\n",
            r#"{"type":"user","timestamp":"1970-01-01T00:05:00Z","message":{"content":"too late"}}"#,
            "\nnot json\n",
            r#"{"type":"mode","mode":"normal"}"#,
        );
        let rendered = digest(text, 0, 60);
        assert!(rendered.contains("user: please fix"));
        assert!(rendered.contains("tool_use Bash: controller bind-slot GAIN-2"));
        assert!(rendered.contains("(subagent) tool_result: slot occupied"));
        assert!(!rendered.contains("too late"));
    }

    #[test]
    fn fit_keeps_head_and_tail() {
        let long = "a".repeat(50) + &"b".repeat(50);
        let fitted = fit(long, 30);
        assert!(fitted.starts_with("aaaaaaaaaa\n…"));
        assert!(fitted.ends_with(&"b".repeat(20)));
    }
}
