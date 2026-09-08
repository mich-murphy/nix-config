use crate::{Process, ProcessRequest, success};
use domain::{
    ids::{Sha, SlotId, TaskId},
    ports::{PortError, Vcs},
};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    str::FromStr,
};

pub struct Git<P> {
    process: P,
    repo: PathBuf,
}

impl<P> Git<P> {
    pub fn new(process: P, repo: PathBuf) -> Self {
        Self { process, repo }
    }
}

impl<P: Process> Git<P> {
    fn git(&self, args: &[&str], cwd: &Path) -> Result<String, PortError> {
        let request = ProcessRequest {
            program: "git".into(),
            args: args.iter().map(ToString::to_string).collect(),
            cwd: cwd.to_owned(),
            stdin: None,
            env: BTreeMap::new(),
            remove_env: Vec::new(),
        };
        success(self.process.run(&request)?)
    }
}

impl<P: Process> Vcs for Git<P> {
    fn head(&self, revision: &str) -> Result<Sha, PortError> {
        parse_sha(self.git(&["rev-parse", revision], &self.repo)?.trim())
    }

    fn on_main(&self, commit: &Sha) -> Result<bool, PortError> {
        let request = ProcessRequest {
            program: "git".into(),
            args: vec![
                "merge-base".into(),
                "--is-ancestor".into(),
                commit.to_string(),
                "origin/main".into(),
            ],
            cwd: self.repo.clone(),
            stdin: None,
            env: BTreeMap::new(),
            remove_env: Vec::new(),
        };
        Ok(self.process.run(&request)?.code == Some(0))
    }

    fn changed_paths(&self, base: &Sha, head: &Sha) -> Result<Vec<String>, PortError> {
        let range = format!("{base}...{head}");
        let output = self.git(&["diff", "--name-only", &range], &self.repo)?;
        Ok(output.lines().map(ToOwned::to_owned).collect())
    }

    fn changed_lines(&self, base: &Sha, head: &Sha) -> Result<u32, PortError> {
        let range = format!("{base}...{head}");
        let output = self.git(&["diff", "--numstat", &range], &self.repo)?;
        Ok(output
            .lines()
            .fold(0_u32, |total, line| total.saturating_add(line_change(line))))
    }

    fn commit_paths(&self, base: &Sha, head: &Sha) -> Result<Vec<Vec<String>>, PortError> {
        let range = format!("{base}..{head}");
        let commits = self.git(&["rev-list", "--reverse", &range], &self.repo)?;
        commits
            .lines()
            .map(|commit| {
                self.git(
                    &["diff-tree", "--no-commit-id", "--name-only", "-r", commit],
                    &self.repo,
                )
                .map(|output| output.lines().map(ToOwned::to_owned).collect())
            })
            .collect()
    }

    fn ignored(&self, path: &Path) -> Result<bool, PortError> {
        let request = ProcessRequest {
            program: "git".into(),
            args: vec![
                "check-ignore".into(),
                "--quiet".into(),
                path.to_string_lossy().into_owned(),
            ],
            cwd: self.repo.clone(),
            stdin: None,
            env: BTreeMap::new(),
            remove_env: Vec::new(),
        };
        Ok(self.process.run(&request)?.code == Some(0))
    }

    fn reserve(&self, task: &TaskId) -> Result<bool, PortError> {
        let head = self.head("HEAD")?;
        let reference = format!("refs/no-mistakes/claims/{task}");
        let zero = "0".repeat(40);
        let request = ProcessRequest {
            program: "git".into(),
            args: vec!["update-ref".into(), reference, head.to_string(), zero],
            cwd: self.repo.clone(),
            stdin: None,
            env: BTreeMap::new(),
            remove_env: Vec::new(),
        };
        Ok(self.process.run(&request)?.code == Some(0))
    }

    fn bind_slot(&self, _task: &TaskId, slot: &SlotId, branch: &str) -> Result<(), PortError> {
        let path = self.repo.join(".worktrees").join(slot.as_ref());
        let path_text = path.to_string_lossy();
        self.git(
            &["worktree", "add", "-b", branch, &path_text, "origin/main"],
            &self.repo,
        )?;
        Ok(())
    }

    fn clean_slot(&self, slot: &SlotId, delete: bool) -> Result<(), PortError> {
        let path = self.repo.join(".worktrees").join(slot.as_ref());
        let path_text = path.to_string_lossy();
        if delete {
            self.git(&["worktree", "remove", &path_text], &self.repo)?;
        } else {
            self.git(&["reset", "--hard", "origin/main"], &path)?;
            self.git(&["clean", "-fd"], &path)?;
        }
        Ok(())
    }
}

fn line_change(line: &str) -> u32 {
    let mut fields = line.split_whitespace();
    let added = fields
        .next()
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(0);
    let deleted = fields
        .next()
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(0);
    added.saturating_add(deleted)
}

fn parse_sha(value: &str) -> Result<Sha, PortError> {
    Sha::from_str(value).map_err(|error| PortError(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::Cell, str::FromStr};

    struct Fake(Cell<u8>);

    impl Process for Fake {
        fn run(&self, request: &ProcessRequest) -> Result<crate::ProcessOutput, PortError> {
            let call = self.0.get();
            self.0.set(call + 1);
            let update = request.args.first().is_some_and(|arg| arg == "update-ref");
            let code = Some(u8::from(update && call > 1).into());
            let stdout = fake_stdout(update);
            Ok(crate::ProcessOutput {
                code,
                stdout,
                stderr: String::new(),
            })
        }
    }

    fn fake_stdout(update: bool) -> String {
        if update {
            String::new()
        } else {
            format!("{}\n", "a".repeat(40))
        }
    }

    #[test]
    fn git_reservation_is_exclusive() -> Result<(), domain::ids::InvalidId> {
        let git = Git::new(Fake(Cell::new(0)), PathBuf::from("/repo"));
        let task = TaskId::from_str("GAIN-1")?;
        assert!(
            git.reserve(&task)
                .map_err(|_| domain::ids::InvalidId("reserve"))?
        );
        assert!(
            !git.reserve(&task)
                .map_err(|_| domain::ids::InvalidId("reserve"))?
        );
        Ok(())
    }
}
