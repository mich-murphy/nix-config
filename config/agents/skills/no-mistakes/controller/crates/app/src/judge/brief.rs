//! The judge's brief: everything the controller knows about one task,
//! rendered as Markdown. The projection gives the outcome, the ledger the
//! timeline, the rejection log and the coordinator transcript the
//! friction. Nothing is interpreted here; the judge does that.

use super::{coordinator, transcript};
use adapters::prefactor::rfc3339;
use domain::{
    budget::Limits,
    event::{Event, EventRecord, LaunchOutcome},
    ids::TaskId,
    judge::RejectionNote,
    state::State,
    task::Task,
};
use serde_json::Value;

const EVENT_CAP: usize = 600;
const OUTPUT_CAP: usize = 1_500;

pub(super) fn render(
    state: &State,
    task: &Task,
    events: &[EventRecord],
    rejections: &[RejectionNote],
    coordinator: Option<&str>,
    now: u64,
) -> String {
    let from = claimed_at(events, &task.id).unwrap_or(0);
    let mut out = format!("# Judge brief: {}\n\n", task.id);
    out.push_str(&format!(
        "Window: {} to {} (task claimed to now). Tier: {:?}.\n\n",
        rfc3339(from),
        rfc3339(now),
        task.tier.current
    ));
    section(&mut out, "Task state (projection)", &pretty(task));
    let limits = Limits::for_tier(
        task.tier.current,
        state.config.as_ref().map(|config| &config.caps),
    );
    section(&mut out, "Budget limits for the tier", &pretty(&limits));
    out.push_str(&launches(state, &task.id));
    out.push_str(&timeline(events, &task.id, from));
    out.push_str(&rejected(rejections, &task.id, from));
    out.push_str(&coordinator_section(coordinator, from, now));
    out
}

fn section(out: &mut String, title: &str, json: &str) {
    out.push_str(&format!("## {title}\n\n```json\n{json}\n```\n\n"));
}

fn pretty<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_string_pretty(value)
        .unwrap_or_else(|error| format!("\"unserialisable: {error}\""))
}

fn claimed_at(events: &[EventRecord], task: &TaskId) -> Option<u64> {
    events
        .iter()
        .find(|record| matches!(&record.event, Event::Claimed { task: id, .. } if id == task))
        .map(|record| record.at)
}

fn launches(state: &State, task: &TaskId) -> String {
    let mut out = String::from("## Launches for this task\n\n");
    let mut any = false;
    for launch in state.launches.iter().filter(|launch| launch.task == *task) {
        any = true;
        let (status, detail) = match &launch.outcome {
            None => ("running", String::new()),
            Some(LaunchOutcome::Cancelled) => ("cancelled", String::new()),
            Some(LaunchOutcome::Failed { reason }) => {
                ("failed", transcript::clip(reason, EVENT_CAP))
            }
            Some(LaunchOutcome::Completed { output, .. }) => {
                ("completed", transcript::clip(output, OUTPUT_CAP))
            }
        };
        out.push_str(&format!(
            "- launch {} role {:?} {status}; usage {}\n",
            launch.id.0,
            launch.role,
            launch.usage.map_or("unknown".to_owned(), |tokens| format!(
                "in {} out {}",
                tokens.input, tokens.output
            ))
        ));
        if !detail.is_empty() {
            out.push_str(&format!("  output: {detail}\n"));
        }
    }
    if !any {
        out.push_str("(none)\n");
    }
    out.push('\n');
    out
}

/// Every event about this task from its claim onward, plus run-level
/// events in the same window, each as its kind and a clipped body.
fn timeline(events: &[EventRecord], task: &TaskId, from: u64) -> String {
    let mut out = String::from("## Ledger timeline\n\n");
    for record in events.iter().filter(|record| record.at >= from) {
        let Ok(value) = serde_json::to_value(&record.event) else {
            continue;
        };
        let owner = value.get("task").and_then(Value::as_str);
        if owner.is_some_and(|owner| owner != task.to_string()) {
            continue;
        }
        let kind = value.get("kind").and_then(Value::as_str).unwrap_or("?");
        let mut body = value.clone();
        if let Some(object) = body.as_object_mut() {
            object.remove("kind");
        }
        out.push_str(&format!(
            "- #{} [{}] {:?} {kind}: {}\n",
            record.sequence,
            rfc3339(record.at),
            record.actor,
            transcript::clip(&body.to_string(), EVENT_CAP)
        ));
    }
    out.push('\n');
    out
}

fn rejected(rejections: &[RejectionNote], task: &TaskId, from: u64) -> String {
    let mut out = String::from("## Rejected controller commands in the window\n\n");
    let mut any = false;
    for note in rejections
        .iter()
        .filter(|note| note.at >= from && note.task.as_ref().is_none_or(|owner| owner == task))
    {
        any = true;
        out.push_str(&format!(
            "- [{}] {}{} {}: {}\n",
            rfc3339(note.at),
            note.command,
            if note.check { " (--check)" } else { "" },
            note.class,
            transcript::clip(&note.message, EVENT_CAP)
        ));
    }
    if !any {
        out.push_str("(none)\n");
    }
    out.push('\n');
    out
}

fn coordinator_section(note: Option<&str>, from: u64, to: u64) -> String {
    let mut out = String::from("## Coordinator transcript in the window\n\n");
    let Some(path) = note.and_then(coordinator::transcript_path) else {
        out.push_str("(unavailable: the skill's hook did not record a transcript path)\n\n");
        return out;
    };
    match std::fs::read_to_string(&path) {
        Ok(text) => out.push_str(&format!(
            "Source: {}\n\n```text\n{}\n```\n\n",
            path.display(),
            transcript::digest(&text, from, to)
        )),
        Err(error) => out.push_str(&format!(
            "(unavailable: {} could not be read: {error})\n\n",
            path.display()
        )),
    }
    out
}
