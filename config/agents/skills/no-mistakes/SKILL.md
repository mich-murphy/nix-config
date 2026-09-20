---
name: no-mistakes
description: Deliver a bounded body of work sequentially through implementation, independent review, verified merge, and optional tracker synchronization. Use for an autonomous delivery run or to resume its ledger, not to inspect or edit this skill.
hooks:
  PreToolUse:
    - matcher: Bash
      hooks:
        - type: command
          command: "\"$HOME/.claude/skills/no-mistakes/hooks/coordinator.sh\""
          timeout: 5
---

# No mistakes

Deliver one task at a time. Measure the starting behavior, make a bounded change,
and verify the result before moving on.

A delivery run requires a work order: a bounded set of tasks with acceptance
criteria, fixed before implementation begins. A tracker epic (Jira) is one
source; a spec written and agreed with the user is another. Either way the
scope is explicit, frozen at discovery, and complete when every task is
verified; the run is never tied to a tracker's existence. Editing this
skill does not start a run. Automatic discovery grants no authority.

## Controller

Build the bundled controller before a run:

```sh
EPIC_SKILL=/absolute/path/to/no-mistakes
. "$EPIC_SKILL/controller/env.sh"
cargo build --release --locked --manifest-path "$EPIC_SKILL/controller/Cargo.toml"
CONTROLLER="$CARGO_TARGET_DIR/release/controller"
```

The run directory is `<repository>/.no-mistakes/<work-key>`. Reuse it when it
exists; never create a second directory for the same work order. It must be
Git-ignored, so add `.no-mistakes/` to `.git/info/exclude` when it is not.
Initialize it with a run config and one of `profiles/codex.toml`,
`profiles/pi.toml`, or `profiles/claude.toml`. The config names the
repository, the work key, the review mode, the merge policy, and optionally
an external tracker; a run without a tracker records status in the ledger
alone and completes on verified delivery.

Every command that writes events returns `next` in its output: the next
action, a command, a filled request template, and a strict schema. Perform
that action directly. Call `next` on its own only after an interruption, a
rejection, or a read-only command. Follow it until the run completes or
records a hold. Use `--check` to validate a mutation without writing events.

The controller owns state, budgets, process records, delivery history, and the
generated ledger. Never edit SQLite, discard a run to reset limits, or perform a
rejected governed action directly. A rejection includes the actual phase and the
next valid action. Use `status` after interruption. Use `recover-operation` only
for the persisted process identity it names. GitHub operations without an owned
process require `observe-pr` instead.

[enforcement.md](references/enforcement.md) maps every normative rule in this
skill to the mechanism that enforces it (controller rejection, structural
impossibility, or a named judgment owner). When you add a rule, decide its
mechanism there first.

Consult [controller.md](references/controller.md) for setup and command groups
when `next` is unavailable. Read conditional references only when the situation
matches. Controller maintenance must pass
[rust-quality-gates.md](references/rust-quality-gates.md).

## Authority

A run authorizes task-scoped investigation, implementation, validation, commits,
pushes, draft PRs, marking PRs ready, ordinary conflict repair, tracker status
transitions for this run's tasks (when a tracker is configured), and cleanup of
this run's merged work. Necessary local tests and builds may run longer than
60 seconds.

Merge is the exception. With the run config's `autonomous_merge` off (the
default), a merge also needs a per-task user receipt: `next` asks for it once
every other gate is green, and the decision is registered with `grant` as a
`Grant::Merge`. One receipt merges exactly once; a follow-up delivery needs a
fresh one. Setting `autonomous_merge: true` is the user's explicit, per-run
opt-in to merge every task whose gates pass.

The run also authorizes the routine judgments below. Make them and record the
reasoning in the plan, checkpoint, or PR body. Do not stop to ask.

- Requirements admit more than one reading: take the reading consistent with
  the work order and the surrounding code. Hold only when the readings lead to
  materially different code and nothing in the tracker or the repository
  decides.
- Evidence for an automated criterion: a test or check at a public boundary
  that would fail without the change. Live failure injection, third-party
  message receipts, and deployment exercises are required only for a
  criterion the work text itself marks as human or operational acceptance.
- Ownership of an unassigned task in the work order: this run's.
- Work already on main that meets the criteria: verify it, do not redo it.
- A transient tracker or GitHub failure: observe, then retry.
- A slot this run owns and has completed: clean it.

It does not authorize deployment, release, infrastructure mutation, production
data changes, browser automation, or human-only acceptance. Tracker authority
is limited to status transitions for this run's tasks and recorded subtasks.
Do not change descriptions, assignment, priority, dates, links, membership,
workflow configuration, comments, or time records.

Autonomous review mode is the default. Human-review mode applies only when the
user chooses it before initialization. It adds a human decision for the exact
snapshot. It does not replace independent review.

The run config's `follow_up_deliveries` is standing authorization: a held task
may reopen that many further code deliveries, and any number of verification
deliveries, through `open-delivery` with no receipt. Use it for holds this run
raised itself, such as incomplete acceptance after a merge or a replaced PR. A
hold for a missing product decision still waits for the answer. Beyond the
standing count, an explicit user receipt can grant one pair, a new delivery, a
merge, narrowed acceptance, or historical slot reuse. Register it through
`grant`, `open-delivery`, `narrow-acceptance`, or `bind-slot` as applicable. A
grant is task-scoped, criteria-bound, and consumed once. It never resets
existing counters or permits an external operation.

## Models and ownership

The selected profile fixes the harness, model assignments, effort, fallbacks,
and escalation model. The controller computes the task tier and may only raise
it. Report a coordinator mismatch. Do not claim that a prompt changed the active
coordinator model.

Use at most two running agents including the coordinator, and one code writer.
Keep one implementer session through repairs and a separate reviewer session.
Pause implementation during review. Launch agents only through the
controller's `run-agent` command. Do not use recursive delegation, bookkeeping
agents, polling agents, or summary agents.

Escalation needs a completed profile reviewer launch. It answers one bounded
consequential question. A missing product decision that changes what code to
write is a hold with one precise question; a choice between equivalent
implementations is the coordinator's to make.

The coordinator owns scheduling, evidence judgment, GitHub delivery, and tracker
connector work. Workers do not edit the controller, run state, the tracker,
GitHub, or other worker checkouts.

## Discovery and scheduling

Fix the work order before the first claim. From a tracker, discover every child
task with pagination; from a written spec, record the agreed tasks directly.
Either way `discover` freezes membership for the batch, and each task's
criteria, dependencies, and priority are explicit before implementation
starts. Verify GitHub, Git, model, and toolchain access, plus tracker access
when one is configured. Read repository instructions. Before each claim, read
the selected task's current membership, status, dependencies, ownership, and
scheduling facts, then send only the refresh delta.

Respect explicit planning order. Otherwise select by priority, due date with
undated tasks last, tracker rank when one exists, then key. A code dependency
needs its delivery on main. A tracker's Done alone is not enough.

A task in the work order is owned by this run when it is unassigned, assigned
to the run's user, or assigned to someone with no branch, PR, or comment on it
in the last seven days. Report `ownership_clear` accordingly. Only another
person's active work makes ownership unclear; record that with the task's
evidence and keep going. The controller schedules around a questioned task and
puts the question to you once nothing else can proceed. Report unmapped
statuses and external authority the same way, once, without stopping the rest
of the queue.

One task stays active through verified merge and tracker synchronization. The
open-PR cap counts only PRs this run opened. At the cap, recover this run's
existing work rather than starting another task. Do not adopt another worker's
branch or clear an unknown reservation.

## Tracker synchronization

Applies only when the run config names a tracker. Without one, the ledger is
the whole status record: the tracker commands reject, and `complete` needs
verified delivery only.

Use the coordinator's authenticated tracker connector. The controller stores
intent and observation but holds no tracker credential.

- To Do means implementation has not started.
- In Progress starts before detailed investigation or implementation. It also
  covers merged work with unfinished acceptance.
- Under Review starts when a draft PR exists and lasts through review, repair,
  checks, and merge preparation.
- Done requires verified delivery on main, complete acceptance, independent
  PASS, and confirmed subtask results.

Before a transition, read the issue and available transitions. Call
`set-status`, perform the exact returned transition, read the issue again, and
call `observe-status`. A timeout is unknown, not failed. Observe before
retrying. A failed transition may be retried twice with a fresh read; the
controller holds the task after the third failure. Never repeat a merge to
repair tracker synchronization.

## Implementation loop

After claim (and confirmed In Progress when a tracker is configured), record
the full criteria with `brief`.
If a commit already on main meets every criterion, for example work an earlier
run merged, call `open-delivery` with a verification delivery of that commit
and no receipt; the task then needs a plan, proof, review, and final
verification, but no slot and no PR. Otherwise use `bind-slot` for a fresh
safe checkout. Slot names are free: pick one whose directory does not exist.
Historical reuse needs a matching receipt and a completed prior owner. Dirty,
redirected, foreign, unknown, or unfinished-owner slots stay untouched; choose
another name instead of cleaning them.

Call `plan` before launching an implementer. Record the original baseline,
smallest complete deliverable, components, local examples, criterion targets,
and measurement methods. Keep the baseline fixed across follow-up deliveries.

After every implementer turn, commit coherent work and call `checkpoint`; it
snapshots the turn itself. State the observation that changed, the remaining
uncertainty, and one next action. Progress means a new observation, not a new
commit or a rewritten summary. If a turn ends without a new observation, with
the implementer stalled, looping, or returning low-confidence output, call
`escalate-tier` with the reason before the next implementer turn instead of
retrying the same assignment. Reassess out-of-plan paths and large diffs
without dropping work-order scope. Use `snapshot` alone only for an integration
commit made without an implementer turn.

Choose the simplest change that meets the criteria. Keep responsibilities in
the owning layer. Search callers and neighboring implementations before adding
an abstraction. Decide ambiguities as the Authority section says; hold only
for a missing product decision that changes what code to write.

## Evidence and review

Use `run-check` for commands and `record-proof` for criterion results. Compare
each result with its baseline. A passing command alone is not acceptance; a
test at a public boundary that fails without the change is. Mark a criterion
`human_only` only when its work text requires a person's judgment or an
operation this run cannot perform; such criteria need an actual human receipt.
Do not manufacture live incidents, deployments, or third-party receipts for a
criterion that a test can demonstrate. Preserve failed, skipped, and
unavailable results honestly. Changed code, criteria, proof artifacts, or
baselines invalidate dependent review.

Tests must detect the real failure at a public boundary. Include relevant valid,
invalid, isolation, permission, and failure cases. Do not assert private helper
calls, source text, or incidental state layout. For managed controls and stale
references, follow [acceptance-and-repair.md](references/acceptance-and-repair.md).

Review the immutable base and head after readiness passes. The reviewer reports
typed findings and evidence gaps. The controller computes the verdict from
severity and tier. Every finding needs a concrete disposition before merge.
Accepted corrections return to the same implementer, then the same independent
reviewer verifies the complete new snapshot. The controller appends the
previously reviewed head and its findings to a repair reviewer's prompt, so
the reviewer concentrates on the delta while still reporting on the whole
snapshot; keep the repair prompt itself short.

## Publish and finish

Create one draft PR from a safe committed checkpoint. Keep its head tied to the
recorded snapshot. Mark it ready when coherent. Required checks must be present
and green. `poll-checks --wait` preserves one deadline per head.

Do not bypass branch protection. Observe GitHub's merged state and the merge
commit on main. When the merge policy is receipt-required (the default), `next`
asks for the merge receipt before naming the merge; register the user's
decision with `grant` and then merge. A receipt is consumed by the merge it
authorizes.

A narrowed delivery may merge its approved subset but cannot complete the task.
For more code, hold the task and use `open-delivery`, under the standing count
or with a fresh receipt. A verification delivery permits reviewer work only. When this run's PR was merged
by someone else, `observe-pr` adopts the merge and the task continues; only a
PR replaced by a different one needs a receipt for later work.

Call `final-verify`, synchronize the parent and each recorded subtask to Done
when a tracker is configured, then `complete` and, when the task owns a slot,
`cleanup`. Cleanup uses only the owned slot. Do not scan or stop unrelated
workers or processes. Slot destruction is anchored: before any reset, clean,
or removal the adapter pins the slot's head under
`refs/no-mistakes/custody/<slot>/<head>`, so a destroyed head stays
recoverable from the main repository.

`complete` also starts the judge for the task as a detached background
process. Do not wait for it, monitor it, or run `judge` yourself; continue with
cleanup and the next task. Its verdict lands in the ledger and in Prefactor on
its own. Read [quality.md](references/quality.md) only if a judge launch needs
recovery or its verdict is questioned.

## Handoff

At the end, run `usage-report`. It starts a judge for any worked task still
without a verdict and leaves the trace open until the last judge lands. Missing
usage remains unknown and never blocks delivery. Then compile the retrospective
as [retrospective.md](references/retrospective.md) describes and store it in
the run directory. Report each task's delivery outcome, confirmed tracker
status (external runs), PR and head, acceptance evidence, blocker, and next
action. Include the ledger path, remaining work, slot status, model
substitutions, escalation use, usage, baseline changes, recurring findings,
and unfinished PR count. Assess the work order's own acceptance separately.
Never claim completion with unresolved evidence or tracker synchronization.
