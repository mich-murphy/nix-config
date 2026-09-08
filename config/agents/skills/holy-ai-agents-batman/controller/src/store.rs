use crate::{
    ids::{Digest as ReceiptDigest, Sha},
    model::*,
};
use anyhow::{Context, Result, bail, ensure};
use rusqlite::{Connection, TransactionBehavior, params};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before epoch")
        .as_secs()
}
pub fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub fn git(repo: &Path, args: &[&str]) -> Result<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .context("run git")?;
    ensure!(
        out.status.success(),
        "git {} failed: {}",
        args.join(" "),
        String::from_utf8_lossy(&out.stderr)
    );
    Ok(String::from_utf8(out.stdout)?.trim().to_owned())
}
pub fn clean(repo: &Path) -> Result<()> {
    ensure!(
        git(repo, &["status", "--porcelain"])?.is_empty(),
        "checkout has uncommitted or untracked work"
    );
    Ok(())
}
pub fn sha(repo: &Path, value: &str) -> Result<Sha> {
    ensure!(!value.starts_with('-'), "invalid revision");
    git(
        repo,
        &["rev-parse", "--verify", &format!("{value}^{{commit}}")],
    )?
    .parse()
    .map_err(anyhow::Error::msg)
}
pub fn on_main(repo: &Path, commit: &str) -> Result<()> {
    let commit = sha(repo, commit)?;
    let result = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args([
            "merge-base",
            "--is-ancestor",
            commit.as_ref(),
            "origin/main",
        ])
        .status()?;
    ensure!(
        result.success(),
        "delivery commit is not available on origin/main"
    );
    Ok(())
}
pub fn worktree(state: &State, task: &Task) -> Result<PathBuf> {
    let slot = task.slot.as_ref().context("task has no bound slot")?;
    let expected = state.repo.join(".worktrees").join(slot);
    let path = expected.canonicalize().context("missing worker checkout")?;
    ensure!(
        path == expected,
        "worker checkout is redirected by a symlink"
    );
    let common = git(
        &path,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    )?;
    ensure!(
        PathBuf::from(common).canonicalize()? == state.repo.join(".git").canonicalize()?,
        "foreign Git checkout"
    );
    ensure!(
        state.slots.get(slot) == Some(&state.run_id),
        "slot is not owned by this run"
    );
    let owner = fs::read_to_string(state.repo.join(".local/epic-control/slots").join(slot))?;
    ensure!(owner == state.run_id, "slot reservation changed");
    Ok(path)
}
pub fn snapshot(state: &State, task: &Task) -> Result<Snapshot> {
    let path = worktree(state, task)?;
    clean(&path)?;
    ensure!(
        Some(git(&path, &["branch", "--show-current"])?) == task.branch,
        "branch changed"
    );
    let previous = task.snapshot.as_ref().context("missing base snapshot")?;
    Ok(Snapshot {
        base: previous.base.clone(),
        head: sha(&path, "HEAD")?,
        requirements: crate::stages::revision(task)?,
    })
}
pub fn fresh(state: &State, task: &Task) -> Result<Snapshot> {
    let current = snapshot(state, task)?;
    ensure!(
        Some(&current) == task.snapshot.as_ref(),
        "snapshot drift: record a new snapshot and revalidate"
    );
    Ok(current)
}
pub fn artifact(path: &Path) -> Result<ReceiptDigest> {
    ensure!(path.is_file(), "evidence artifact is missing");
    let bytes = fs::read(path)?;
    ensure!(!bytes.is_empty(), "evidence artifact is empty");
    hash(&bytes).parse().map_err(anyhow::Error::msg)
}

pub struct Store {
    pub root: PathBuf,
    db: Connection,
}
impl Store {
    pub fn open(root: &Path) -> Result<Self> {
        ensure!(root.join("state.sqlite3").is_file(), "run not initialized");
        let root = root.canonicalize()?;
        let db = Connection::open(root.join("state.sqlite3"))?;
        db.busy_timeout(std::time::Duration::from_secs(10))?;
        db.execute_batch("PRAGMA foreign_keys=ON; PRAGMA synchronous=FULL;")?;
        Ok(Self { root, db })
    }
    pub fn create(root: &Path, state: State) -> Result<Self> {
        ensure!(
            !root.exists(),
            "run directory already exists; resume it instead"
        );
        ensure!(root.is_absolute(), "state path must be absolute");
        let repo = &state.repo;
        ensure!(
            repo.join(".git").is_dir(),
            "initialize from the primary checkout"
        );
        ensure!(
            root.starts_with(repo) && !root.starts_with(repo.join(".worktrees")),
            "state must be inside primary checkout, outside resettable worker paths"
        );
        let rel = root.strip_prefix(repo)?;
        ensure!(
            !rel.components()
                .any(|c| matches!(c, std::path::Component::ParentDir)),
            "invalid state path"
        );
        let ignored = Command::new("git")
            .arg("-C")
            .arg(repo)
            .args(["check-ignore", "--quiet", "--no-index"])
            .arg(rel)
            .status()?;
        ensure!(
            ignored.success(),
            "state directory is not Git-ignored; choose an ignored location"
        );
        fs::create_dir_all(root.parent().context("state parent")?)?;
        ensure!(
            root.parent().unwrap().canonicalize()? == root.parent().unwrap(),
            "state path contains a symlink"
        );
        fs::create_dir(root)?;
        let db = Connection::open(root.join("state.sqlite3"))?;
        db.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;
            CREATE TABLE state (id INTEGER PRIMARY KEY CHECK(id=1), data TEXT NOT NULL);
            CREATE TABLE events (seq INTEGER PRIMARY KEY, at INTEGER NOT NULL, action TEXT NOT NULL, data TEXT NOT NULL);")?;
        db.execute(
            "INSERT INTO state VALUES(1, ?1)",
            [serde_json::to_string(&state)?],
        )?;
        let store = Self {
            root: root.to_path_buf(),
            db,
        };
        store.render(&state)?;
        Ok(store)
    }
    pub fn read(&self) -> Result<State> {
        let data: String = self
            .db
            .query_row("SELECT data FROM state WHERE id=1", [], |r| r.get(0))?;
        let state: State = serde_json::from_str(&data)?;
        ensure!(
            matches!(state.version, 1..=10),
            "unsupported state schema; do not overwrite"
        );
        Ok(state)
    }
    pub fn change<F>(&mut self, action: &str, f: F) -> Result<serde_json::Value>
    where
        F: FnOnce(&mut State) -> Result<serde_json::Value>,
    {
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let data: String = tx.query_row("SELECT data FROM state WHERE id=1", [], |r| r.get(0))?;
        let mut state: State = serde_json::from_str(&data)?;
        ensure!(matches!(state.version, 1..=10), "unsupported state schema");
        let result = f(&mut state)?;
        // Preserve newer schema markers so older binaries cannot drop their guards.
        state.version = state.version.max(2);
        tx.execute(
            "UPDATE state SET data=?1 WHERE id=1",
            [serde_json::to_string(&state)?],
        )?;
        tx.execute(
            "INSERT INTO events(at,action,data) VALUES(?1,?2,?3)",
            params![
                i64::try_from(now())?,
                action,
                serde_json::to_string(&result)?
            ],
        )?;
        tx.commit()?;
        // The database is authoritative. A failed projection never rolls back a completed action.
        if let Err(error) = self.render(&state) {
            eprintln!("ledger projection failed; run status to regenerate: {error}");
        }
        Ok(result)
    }
    pub fn render(&self, state: &State) -> Result<()> {
        let mut md = format!(
            "# Epic delivery ledger\n\nRun: {}\n\nEpic: {}\n\nGenerated from state.sqlite3. Do not edit this file.\n\n| Key | Parent | Priority | Dates | Prerequisites | Eligibility | Delivery | Jira | Sync | Slot | Branch | Base/head | PR | Counters | Next action |\n| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |\n",
            state.run_id, state.epic
        );
        md.push_str(&format!("Loop policy: {:?}\n\n", state.loop_policy));
        for (key, t) in &state.tasks {
            let fields = vec![
                key.clone(),
                t.spec.parent.to_string(),
                t.spec.priority.to_string(),
                format!("due {:?}, not before {:?}", t.spec.due, t.spec.not_before),
                t.spec
                    .dependencies
                    .iter()
                    .map(|d| d.key.clone())
                    .collect::<Vec<_>>()
                    .join(", "),
                format!(
                    "member {}, ownership {}; {:?}",
                    t.spec.member, t.spec.ownership_clear, t.spec.blocker
                ),
                format!("{:?}", t.delivery),
                t.spec.jira_status.clone(),
                format!("{:?}", t.sync),
                t.slot.as_ref().map(ToString::to_string).unwrap_or_default(),
                t.branch.clone().unwrap_or_default(),
                t.snapshot
                    .as_ref()
                    .map(|s| format!("{}/{}", s.base, s.head))
                    .unwrap_or_default(),
                format!(
                    "primary {:?}, current {:?}, delivery {}, history {}, stage {:?}, external replacements {:?}",
                    t.delivery_history.first().and_then(|d| d.task.pr).or(t.pr),
                    t.pr,
                    crate::followup::identity(t),
                    t.delivery_history.len(),
                    crate::stages::current(t).map(|stage| stage.id),
                    t.delivery_history
                        .iter()
                        .filter_map(|d| d.superseded_by.as_ref().map(|r| r.pr))
                        .collect::<Vec<_>>()
                ),
                format!(
                    "review {}, repair {}, CI {}, Astra {}, stalled {}, implementation {}, documentation launches {}",
                    t.reviews,
                    t.repairs,
                    t.ci_repairs,
                    t.escalations,
                    t.loop_state.stalled,
                    state
                        .launches
                        .iter()
                        .filter(|l| l.key == *key
                            && l.role == Role::Implementer
                            && !crate::documentation::exempt(t, l.id))
                        .count(),
                    state
                        .launches
                        .iter()
                        .filter(|l| l.key == *key && crate::documentation::exempt(t, l.id))
                        .count()
                ),
                t.next_action.clone(),
            ];
            md.push_str(&format!(
                "| {} |\n",
                fields
                    .iter()
                    .map(|s| s.replace('|', "\\|").replace('\n', " "))
                    .collect::<Vec<_>>()
                    .join(" | ")
            ));
        }
        let temp = self.root.join(format!("ledger.{}.tmp", std::process::id()));
        fs::write(&temp, md)?;
        fs::rename(temp, self.root.join("ledger.md"))?;
        Ok(())
    }
}

pub fn required_text(value: &str, name: &str) -> Result<()> {
    if value.trim().is_empty() {
        bail!("{name} is required");
    }
    Ok(())
}
