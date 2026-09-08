//! Path-scoped authority keeps documentation launches outside ordinary budgets.
use crate::{authority, model::*, store::*};
use anyhow::{Context, Result, ensure};
use std::path::Path;

fn pair(t: &Task) -> Option<&authority::Authority> {
    authority::current_pair(&t.authorities).filter(|entry| authority::scoped(entry))
}

pub fn pending(t: &Task) -> bool {
    pair(t).is_some()
}

pub fn implementation_available(t: &Task) -> bool {
    pair(t).is_some_and(|entry| authority::pair_available(entry, true))
}

pub fn exempt(t: &Task, launch: u64) -> bool {
    t.authorities.iter().any(|entry| {
        authority::scoped(entry)
            && entry.used.as_ref().is_some_and(|used| {
                used.implementation == Some(launch) || used.review == Some(launch)
            })
    })
}

fn regular_document(repo: &Path, revision: &str, path: &str) -> Result<()> {
    let entry = git(repo, &["ls-tree", revision, "--", path])?;
    ensure!(
        entry.starts_with("100644 blob "),
        "documentation file must be an existing non-executable regular Git blob"
    );
    Ok(())
}

pub fn validate_scope(s: &State, t: &Task, snap: &Snapshot) -> Result<()> {
    let Some(entry) = pair(t) else { return Ok(()) };
    authority::validate_requirements(entry, &snap.requirements)?;
    let paths = authority::paths(entry).context("missing documentation paths")?;
    let checkout = worktree(s, t)?;
    for path in paths {
        regular_document(&checkout, &snap.head, path)?;
    }
    let commits = git(
        &checkout,
        &["rev-list", &format!("{}..{}", snap.base, snap.head)],
    )?;
    for commit in commits.lines() {
        let changed = git(
            &checkout,
            &[
                "diff-tree",
                "--root",
                "--no-commit-id",
                "--name-only",
                "--no-renames",
                "-r",
                "-m",
                commit,
            ],
        )?;
        ensure!(
            changed
                .lines()
                .all(|path| paths.iter().any(|allowed| allowed == path)),
            "documentation round changed an unapproved file"
        );
    }
    Ok(())
}

pub fn before_launch(s: &State, t: &Task, role: &Role) -> Result<bool> {
    let Some(entry) = pair(t) else {
        return Ok(false);
    };
    ensure!(
        *role != Role::Escalation,
        "path-scoped pair does not authorize escalation"
    );
    authority::validate_requirements(entry, &crate::stages::revision(t)?)?;
    validate_scope(s, t, &fresh(s, t)?)?;
    ensure!(
        authority::pair_available(entry, *role == Role::Implementer),
        "path-scoped pair is spent"
    );
    if *role == Role::Reviewer {
        ensure!(
            entry
                .used
                .as_ref()
                .is_some_and(|used| used.implementation.is_some()),
            "documentation repair must precede review"
        );
    }
    Ok(true)
}

pub fn record_launch(t: &mut Task, launch: &Launch) -> Result<()> {
    let entry =
        authority::current_pair_mut(&mut t.authorities).context("missing path-scoped pair")?;
    authority::record_pair(entry, launch.role == Role::Implementer, launch.id)
}
