//! Where the coordinator's own transcript is. The controller cannot see
//! the session that drives it, so the skill's `PreToolUse` hook writes a
//! small JSON note per working directory before every shell command; the
//! note for the controller's own working directory names the transcript.

use serde_json::Value;
use std::path::PathBuf;

/// The hook's note directory: `$XDG_CACHE_HOME/no-mistakes/coordinator`,
/// falling back to `~/.cache`.
fn directory() -> Option<PathBuf> {
    let cache = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache")))?;
    Some(cache.join("no-mistakes").join("coordinator"))
}

/// The note file for `cwd`: the path with every separator replaced, the
/// same encoding the hook uses.
pub(super) fn note_name(cwd: &str) -> String {
    format!("{}.json", cwd.replace('/', "-"))
}

/// The note for the current working directory, as its raw JSON text, if
/// the hook has written one.
pub(super) fn locate() -> Option<String> {
    let cwd = std::env::current_dir().ok()?;
    let path = directory()?.join(note_name(&cwd.to_string_lossy()));
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str::<Value>(&text).ok()?;
    Some(text)
}

/// The transcript path inside a note.
pub(super) fn transcript_path(note: &str) -> Option<PathBuf> {
    let value: Value = serde_json::from_str(note).ok()?;
    value
        .get("transcript_path")
        .and_then(Value::as_str)
        .map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn note_name_flattens_the_path() {
        assert_eq!(note_name("/home/me/dev"), "-home-me-dev.json");
    }

    #[test]
    fn transcript_path_reads_the_hook_note() {
        let note = r#"{"session_id":"s","transcript_path":"/t.jsonl","cwd":"/x"}"#;
        assert_eq!(transcript_path(note), Some(PathBuf::from("/t.jsonl")));
        assert_eq!(transcript_path("{}"), None);
    }
}
