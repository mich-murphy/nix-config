use crate::{Process, ProcessRequest, success};
use domain::{
    delivery::{Check, CheckState, PrState, PullRequest},
    ids::{PrNumber, Sha},
    ports::{GitHub, MergeMethod, PortError},
};
use serde::Deserialize;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    str::FromStr,
};

pub struct Gh<P> {
    process: P,
    repo: PathBuf,
    name: String,
}

impl<P> Gh<P> {
    pub fn new(process: P, repo: PathBuf, name: String) -> Self {
        Self {
            process,
            repo,
            name,
        }
    }
}

impl<P: Process> Gh<P> {
    fn gh(&self, args: Vec<String>) -> Result<String, PortError> {
        let request = ProcessRequest {
            program: "gh".into(),
            args,
            cwd: self.repo.clone(),
            stdin: None,
            env: BTreeMap::new(),
            remove_env: Vec::new(),
            timeout_seconds: 300,
        };
        success(self.process.run(&request)?)
    }

    fn view(&self, number: PrNumber) -> Result<PullRequest, PortError> {
        let output = self.gh(vec![
            "pr".into(),
            "view".into(),
            number.0.to_string(),
            "--repo".into(),
            self.name.clone(),
            "--json".into(),
            "number,state,isDraft,headRefOid,mergeCommit,statusCheckRollup".into(),
        ])?;
        parse_pr(&output)
    }
}

impl<P: Process> GitHub for Gh<P> {
    fn create(&self, title: &str, body: &Path, head: &Sha) -> Result<PullRequest, PortError> {
        let output = self.gh(vec![
            "pr".into(),
            "create".into(),
            "--draft".into(),
            "--repo".into(),
            self.name.clone(),
            "--title".into(),
            title.into(),
            "--body-file".into(),
            body.to_string_lossy().into_owned(),
            "--head".into(),
            head.to_string(),
        ])?;
        let number = output
            .trim()
            .rsplit('/')
            .next()
            .and_then(|value| value.parse().ok())
            .ok_or_else(|| PortError("gh did not return a PR number".into()))?;
        self.view(PrNumber(number))
    }

    fn ready(&self, number: PrNumber, head: &Sha) -> Result<PullRequest, PortError> {
        let current = self.view(number)?;
        exact_head(&current, head)?;
        self.gh(vec![
            "pr".into(),
            "ready".into(),
            number.0.to_string(),
            "--repo".into(),
            self.name.clone(),
        ])?;
        self.view(number)
    }

    fn merge(
        &self,
        number: PrNumber,
        head: &Sha,
        method: MergeMethod,
    ) -> Result<PullRequest, PortError> {
        let current = self.view(number)?;
        exact_head(&current, head)?;
        let method = match method {
            MergeMethod::Merge => "--merge",
            MergeMethod::Squash => "--squash",
            MergeMethod::Rebase => "--rebase",
        };
        self.gh(vec![
            "pr".into(),
            "merge".into(),
            number.0.to_string(),
            "--repo".into(),
            self.name.clone(),
            method.into(),
            "--match-head-commit".into(),
            head.to_string(),
        ])?;
        self.view(number)
    }

    fn observe(&self, number: PrNumber) -> Result<PullRequest, PortError> {
        self.view(number)
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GhPr {
    number: u64,
    state: String,
    is_draft: bool,
    head_ref_oid: String,
    merge_commit: Option<Oid>,
    #[serde(default)]
    status_check_rollup: Vec<GhCheck>,
}

#[derive(Deserialize)]
struct Oid {
    oid: String,
}

#[derive(Deserialize)]
struct GhCheck {
    #[serde(default)]
    name: String,
    #[serde(default)]
    status: String,
    #[serde(default)]
    conclusion: String,
}

fn parse_pr(text: &str) -> Result<PullRequest, PortError> {
    let value: GhPr = serde_json::from_str(text).map_err(|error| PortError(error.to_string()))?;
    let state = match value.state.as_str() {
        "OPEN" => PrState::Open,
        "CLOSED" => PrState::Closed,
        "MERGED" => PrState::Merged,
        other => return Err(PortError(format!("unknown PR state {other}"))),
    };
    let merge = value
        .merge_commit
        .map(|commit| parse_sha(&commit.oid))
        .transpose()?;
    let checks = value
        .status_check_rollup
        .into_iter()
        .filter(|check| !check.name.is_empty())
        .map(parse_check)
        .collect();
    Ok(PullRequest {
        number: PrNumber(value.number),
        state,
        draft: value.is_draft,
        head: parse_sha(&value.head_ref_oid)?,
        merge,
        checks,
    })
}

fn parse_check(check: GhCheck) -> Check {
    let state = match (check.status.as_str(), check.conclusion.as_str()) {
        ("COMPLETED", "SUCCESS" | "NEUTRAL") => CheckState::Pass,
        ("COMPLETED", "SKIPPED") => CheckState::Skipped,
        ("COMPLETED", _) => CheckState::Fail,
        _ => CheckState::Pending,
    };
    Check {
        name: check.name,
        state,
        required: true,
    }
}

fn exact_head(pr: &PullRequest, head: &Sha) -> Result<(), PortError> {
    if &pr.head == head {
        Ok(())
    } else {
        Err(PortError("PR head changed".into()))
    }
}

fn parse_sha(value: &str) -> Result<Sha, PortError> {
    Sha::from_str(value).map_err(|error| PortError(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn github_observation_parses_pr() -> Result<(), PortError> {
        let text = format!(
            "{{\"number\":7,\"state\":\"MERGED\",\"isDraft\":false,\"headRefOid\":\"{}\",\"mergeCommit\":{{\"oid\":\"{}\"}},\"statusCheckRollup\":[{{\"name\":\"test\",\"status\":\"COMPLETED\",\"conclusion\":\"SUCCESS\"}}]}}",
            "a".repeat(40),
            "b".repeat(40)
        );
        let pr = parse_pr(&text)?;
        assert_eq!(pr.state, PrState::Merged);
        assert_eq!(pr.checks[0].state, CheckState::Pass);
        Ok(())
    }

    #[test]
    fn github_observation_ignores_nameless_rollup_placeholders() -> Result<(), PortError> {
        let text = format!(
            "{{\"number\":7,\"state\":\"OPEN\",\"isDraft\":false,\"headRefOid\":\"{}\",\"mergeCommit\":null,\"statusCheckRollup\":[{{\"name\":\"test\",\"status\":\"COMPLETED\",\"conclusion\":\"SUCCESS\"}},{{}}]}}",
            "a".repeat(40),
        );
        let pr = parse_pr(&text)?;
        assert_eq!(pr.checks.len(), 1);
        assert_eq!(pr.checks[0].name, "test");
        Ok(())
    }
}
