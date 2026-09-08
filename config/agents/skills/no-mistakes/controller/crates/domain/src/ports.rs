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

pub trait Clock {
    fn now(&self) -> Instant;
}

pub trait Vcs {
    fn head(&self, revision: &str) -> Result<Sha, PortError>;
    fn on_main(&self, commit: &Sha) -> Result<bool, PortError>;
    fn changed_paths(&self, base: &Sha, head: &Sha) -> Result<Vec<String>, PortError>;
    fn commit_paths(&self, base: &Sha, head: &Sha) -> Result<Vec<Vec<String>>, PortError>;
    fn reserve(&self, task: &TaskId) -> Result<bool, PortError>;
    fn bind_slot(&self, task: &TaskId, slot: &SlotId, branch: &str) -> Result<(), PortError>;
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
    fn run(&self, request: &LaunchRequest) -> Result<LaunchResult, PortError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchRequest {
    pub id: LaunchId,
    pub assignment: Assignment,
    pub prompt: String,
    pub cwd: std::path::PathBuf,
    pub session: Option<String>,
    pub reviewer: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchResult {
    pub session: String,
    pub output: String,
    pub tokens: Option<Tokens>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tokens {
    pub input: u64,
    pub cached: Option<u64>,
    pub output: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Capabilities {
    pub structured_output: StructuredOutput,
    pub isolation: Isolation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StructuredOutput {
    Native,
    SelfValidated,
    BestEffort,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Isolation {
    Sandbox,
    ToolRestriction,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MergeMethod {
    Merge,
    Squash,
    Rebase,
}
