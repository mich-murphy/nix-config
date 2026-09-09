---
name: no-mistakes
description: Deliver a Jira epic or task sequentially through implementation, independent review, verified merge, and Jira synchronization. Use for an autonomous delivery run or to resume its ledger, not to inspect or edit this skill.
---

# No mistakes

Deliver one task at a time. Measure the starting behavior, make a bounded change,
and verify the result before moving on.

A delivery run requires an epic URL or an unambiguous epic identity in the user
request. Editing this skill does not start a run. Automatic discovery grants no
authority.

## Controller

Build the bundled controller before a run:

```sh
EPIC_SKILL=/absolute/path/to/no-mistakes
. "$EPIC_SKILL/controller/env.sh"
cargo build --release --locked --manifest-path "$EPIC_SKILL/controller/Cargo.toml"
CONTROLLER="$CARGO_TARGET_DIR/release/controller"
```

Create an absolute Git-ignored run directory outside worker checkouts. Initialize
it with a run config and either `profiles/codex.toml` or `profiles/pi.toml`.
After every terminal action, call `next`. Its response contains the next action,
a command, a filled request template, and a strict schema. Follow it until the
run completes or records a hold. Use `--check` to validate a mutation without
writing events.

The controller owns state, budgets, process records, delivery history, and the
generated ledger. Never edit SQLite, discard a run to reset limits, or perform a
rejected governed action directly. A rejection includes the actual phase and the
next valid action. Use `status` after interruption. Use `recover-operation` only
for the persisted process identity it names. GitHub operations without an owned
process require `observe-pr` instead.

Consult [controller.md](references/controller.md) for setup and command groups
when `next` is unavailable. Read conditional references only when the situation
matches. Controller maintenance must pass
[rust-quality-gates.md](references/rust-quality-gates.md).

## Authority

A run authorizes task-scoped investigation, implementation, validation, commits,
pushes, draft PRs, marking PRs ready, ordinary conflict repair, merge after all
gates pass, Jira status transitions, and cleanup of this run's merged work.
Necessary local tests and builds may run longer than 60 seconds.

It does not authorize deployment, release, infrastructure mutation, production
data changes, browser automation, or human-only acceptance. Jira authority is
limited to status transitions for this run's tasks and recorded subtasks. Do not
change descriptions, assignment, priority, dates, links, membership, workflow
configuration, comments, or time records.

Autonomous review mode is the default. Human-review mode applies only when the
user chooses it before initialization. It adds a human decision for the exact
snapshot. It does not replace independent review.

An explicit user receipt can grant one pair, a new delivery, narrowed acceptance,
or historical slot reuse. Register it through `grant`, `open-delivery`,
`narrow-acceptance`, or `bind-slot` as applicable. A grant is task-scoped,
criteria-bound, and consumed once. It never resets existing counters or permits
an external operation.

## Models and ownership

The selected profile fixes the harness, model assignments, effort, fallbacks,
and escalation model. The controller computes the task tier and may only raise
it. Report a coordinator mismatch. Do not claim that a prompt changed the active
coordinator model.

Use at most two running agents including the coordinator, and one code writer.
Keep one implementer session through repairs and a separate reviewer session.
Pause implementation during review. Do not launch Claude, recursive delegation,
bookkeeping agents, polling agents, or summary agents.

Escalation needs a completed profile reviewer launch. It answers one bounded
consequential question. Missing business decisions are blockers, not
escalation prompts.

The coordinator owns scheduling, evidence judgment, GitHub delivery, and Jira
connector work. Workers do not edit the controller, run state, Jira, GitHub, or
other worker checkouts.

## Discovery and scheduling

Read repository instructions and verify Jira, GitHub, Git, model, and toolchain
access. Discover every child task with pagination. Freeze membership for the
batch. Before each claim, read the selected issue's current membership, status,
dependencies, ownership, and scheduling facts, then send only the refresh delta.

Respect explicit planning order. Otherwise select by priority, due date with
undated tasks last, Jira rank, then key. A code dependency needs its delivery on
main. Jira Done alone is not enough. Surface missing decisions, ambiguous
ownership, unmapped statuses, and external authority at once. Dependency blocks
inside the run may wait silently.

One task stays active through verified merge and Jira synchronization. At the
open-PR cap, recover existing work rather than starting another task. Do not
adopt another worker's branch or clear an unknown reservation.

## Jira stages

Use the coordinator's authenticated Jira connector. The controller stores intent
and observation but holds no Jira credential.

- To Do means implementation has not started.
- In Progress starts before detailed investigation or implementation. It also
  covers merged work with unfinished acceptance.
- Under Review starts when a draft PR exists and lasts through review, repair,
  checks, and merge preparation.
- Done requires verified delivery on main, complete acceptance, independent
  PASS, and confirmed subtask results.

Before a transition, read the issue and available transitions. Call `set-status`,
perform the exact returned transition, read the issue again, and call
`observe-status`. A timeout is unknown, not failed. Observe before retrying.
Never repeat a merge to repair Jira synchronization.

## Implementation loop

After claim and confirmed In Progress, record the full criteria with `brief`.
Use `bind-slot` for a fresh safe checkout. Historical reuse needs a matching
receipt and a completed prior owner. Dirty, redirected, foreign, unknown, or
unfinished-owner slots stay untouched.

Call `plan` before launching an implementer. Record the original baseline,
smallest complete deliverable, components, local examples, criterion targets,
and measurement methods. Keep the baseline fixed across follow-up deliveries.

After every implementer turn, commit coherent work, call `snapshot`, then
`checkpoint`. State the observation that changed, the remaining uncertainty,
and one next action. Do not call a new commit or rewritten summary progress.
Reassess out-of-plan paths and large diffs without dropping Jira scope.

Choose the simplest change that meets the criteria. Keep responsibilities in
the owning layer. Search callers and neighboring implementations before adding
an abstraction. Missing business rules or contradictory requirements require a
hold with one precise question.

## Evidence and review

Use `run-check` for commands and `record-proof` for criterion results. Compare
each result with its baseline. A passing command alone is not acceptance.
Human-only criteria require an actual human receipt. Preserve failed, skipped,
and unavailable results honestly. Changed code, criteria, proof artifacts, or
baselines invalidate dependent review.

Tests must detect the real failure at a public boundary. Include relevant valid,
invalid, isolation, permission, and failure cases. Do not assert private helper
calls, source text, or incidental state layout. For managed controls and stale
references, follow [acceptance-and-repair.md](references/acceptance-and-repair.md).

Review the immutable base and head after readiness passes. The reviewer reports
typed findings and evidence gaps. The controller computes the verdict from
severity and tier. Every finding needs a concrete disposition before merge.
Accepted corrections return to the same implementer, then the same independent
reviewer verifies the complete new snapshot.

## Publish and finish

Create one draft PR from a safe committed checkpoint. Keep its head tied to the
recorded snapshot. Mark it ready when coherent. Required checks must be present
and green. `poll-checks --wait` preserves one deadline per head.

Do not bypass branch protection. Observe GitHub's merged state and the merge
commit on main.

A narrowed delivery may merge its approved subset but cannot complete the task.
For more code, hold the task and use `open-delivery` with a fresh receipt. A
verification delivery permits reviewer work only. A replaced PR is recorded by
`observe-pr`; later code or cleanup is an ordinary separately approved delivery.

Call `final-verify`, synchronize the parent and each recorded subtask to Done,
then `complete` and `cleanup`. Cleanup uses only the owned slot. Do not scan or
stop unrelated workers or processes.

## Handoff

At the end, run `usage-report`. Missing usage remains unknown and never blocks
delivery. Report each task's delivery outcome, confirmed Jira status, PR and
head, acceptance evidence, blocker, and next action. Include the ledger path,
remaining work, slot status, model substitutions, escalation use, usage,
baseline changes, recurring findings, and unfinished PR count. Assess the epic's
own acceptance separately. Never claim completion with unresolved evidence or
Jira synchronization.
