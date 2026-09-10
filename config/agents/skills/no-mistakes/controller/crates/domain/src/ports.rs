use crate::{
    Instant,
    delivery::PullRequest,
    ids::{LaunchId, PrNumber, Sha, SlotId, TaskId},
    risk::{Assignment, HarnessKind},
};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortError(pub String);

impl std::fmt::Display for PortError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for PortError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProcessIdentity {
    pub pid: u32,
    pub start_ticks: u64,
    pub group: u32,
}

pub trait Clock {
    fn now(&self) -> Instant;

    /// Blocks for `seconds` (a fake clock in tests advances its own time
    /// instead of really sleeping). Used by a bounded wait such as
    /// `poll-checks --wait` to poll without busy-looping.
    fn sleep(&self, seconds: u64);
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SlotState {
    Missing,
    Unsafe(String),
    Checkout { clean: bool, head: Sha },
}

pub trait Vcs {
    fn head(&self, revision: &str) -> Result<Sha, PortError>;
    fn on_main(&self, commit: &Sha) -> Result<bool, PortError>;
    fn changed_paths(&self, base: &Sha, head: &Sha) -> Result<Vec<String>, PortError>;
    fn changed_lines(&self, base: &Sha, head: &Sha) -> Result<u32, PortError>;
    fn commit_paths(&self, base: &Sha, head: &Sha) -> Result<Vec<Vec<String>>, PortError>;
    fn ignored(&self, path: &Path) -> Result<bool, PortError>;
    fn reserve(&self, task: &TaskId) -> Result<bool, PortError>;
    fn inspect_slot(&self, slot: &SlotId) -> Result<SlotState, PortError>;
    fn bind_slot(&self, task: &TaskId, slot: &SlotId, branch: &str) -> Result<(), PortError>;
    fn reuse_slot(&self, slot: &SlotId, branch: &str) -> Result<(), PortError>;
    fn clean_slot(&self, slot: &SlotId, delete: bool) -> Result<(), PortError>;
}

pub trait GitHub {
    fn create(&self, title: &str, body: &Path, head: &Sha) -> Result<PullRequest, PortError>;
    fn ready(&self, number: PrNumber, head: &Sha) -> Result<PullRequest, PortError>;
    fn merge(
        &self,
        number: PrNumber,
        head: &Sha,
        method: MergeMethod,
    ) -> Result<PullRequest, PortError>;
    fn observe(&self, number: PrNumber) -> Result<PullRequest, PortError>;
}

pub trait Harness {
    fn kind(&self) -> HarnessKind;
    fn capabilities(&self) -> Capabilities;
    fn run(
        &self,
        request: &LaunchRequest,
        started: &mut dyn FnMut(ProcessIdentity) -> Result<(), PortError>,
    ) -> Result<LaunchResult, PortError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchRequest {
    pub id: LaunchId,
    pub assignment: Assignment,
    pub prompt: String,
    pub cwd: std::path::PathBuf,
    pub session: Option<String>,
    pub reviewer: bool,
    /// Where the controller has written the reviewer's output schema, for
    /// a harness whose `Capabilities::structured_output` is `Native`
    /// (Codex `--output-schema`). `app` writes the file and fills this in
    /// before the launch; a harness that ignores it (pi, `SelfValidated`)
    /// is unaffected.
    pub output_schema: Option<std::path::PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchResult {
    pub session: String,
    pub output: String,
    pub tokens: Option<Tokens>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Tokens {
    pub input: u64,
    pub cached: Option<u64>,
    pub output: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Capabilities {
    pub structured_output: StructuredOutput,
    pub isolation: Isolation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum StructuredOutput {
    Native,
    SelfValidated,
    BestEffort,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Isolation {
    Sandbox,
    ToolRestriction,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum MergeMethod {
    Merge,
    Squash,
    Rebase,
}

/// Durable bookkeeping for the trace mirror: the external instance, spans
/// still open, and records not yet sent. It lives beside the event log,
/// never inside it; nothing here is a domain fact or enters the fold.
pub trait TraceState {
    fn get(&self, key: &str) -> Result<Option<String>, PortError>;
    fn set(&mut self, key: &str, value: &str) -> Result<(), PortError>;
    fn remove(&mut self, key: &str) -> Result<(), PortError>;
}

/// Mirrors committed events to an external trace. Both methods are best
/// effort: a failure is kept in `TraceState` for a later retry and never
/// reaches the caller, so tracing cannot fail or block a command.
pub trait Tracer {
    fn record(&self, state: &mut dyn TraceState, records: &[crate::event::EventRecord]);
    fn finish(&self, state: &mut dyn TraceState);
}
