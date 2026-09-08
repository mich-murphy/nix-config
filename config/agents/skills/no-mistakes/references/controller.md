# Rust controller contract

Read this file before running an epic with this skill. `controller/` contains
`epic-control`, a local Rust executable backed by bundled SQLite. It requires
no daemon, Python runtime, API key, or database service. Cargo.lock pins its
build dependencies. The built-in process, GitHub, and repository-helper runners
currently target Linux. Native agent and Jira tool receipts use the same
platform-independent state transitions, but other operating systems have not
been validated.

## Build and invocation

Resolve `EPIC_SKILL` to the directory containing the loaded SKILL.md. Resolve
`EPIC_RUN` to one new, absolute, ignored directory in the primary checkout,
usually `.local/epic-runs/<epic-key>-<unique-run-id>`. Do not place it in a worker.
Do not reuse environment names such as HOME or CODEX_HOME.

```sh
. "$EPIC_SKILL/controller/env.sh"
cargo build --release --locked --manifest-path "$EPIC_SKILL/controller/Cargo.toml"
EPIC_CTL="$CARGO_TARGET_DIR/release/epic-control"
"$EPIC_CTL" --state "$EPIC_RUN" init --input "$INIT_REQUEST"
"$EPIC_CTL" --state "$EPIC_RUN" next
```

The script derives the cache root as
`${XDG_CACHE_HOME:-$HOME/.cache}/no-mistakes`; build output and the
pinned toolchain stay below it. Each request is a JSON object supplied through
`--input <file>` or stdin. Do
not include `command` inside JSON. `status`, `next`, and `drive` need no input.
The program emits `{"ok":true,"result":...}` on success and returns nonzero
with a JSON error on rejection. A rejection is a guardrail to resolve, never
permission to edit the database, reset counters, create a replacement run, or
execute the rejected action directly.

Keep short request files and long logs inside the ignored run directory after
initialization. Prepare the init request in an existing ignored location.
The controller verifies that the run path is ignored and outside worker paths.
`state.sqlite3` is authoritative. `ledger.md` is a generated projection; `status`
regenerates it after an interrupted report write. Preserve the SQLite database
and its WAL/SHM companions together while the controller is running. Never copy
only a live database file as a checkpoint. Ordinary recovery opens the original
run directory directly.

`status` includes operation records, role sessions, counters, check deadlines,
subtask synchronization, and delivery facts. It does not read credentials.
Do not include secret material in request files, briefs, or recorded evidence.

## Progress policy and feedback

Read [progress-and-feedback.md](progress-and-feedback.md) during preflight.
It defines `loop_policy`, `configure-loop`, `loop-plan`, `measure`, `checkpoint`,
`lesson-record`, and `human-review`, plus legacy-state adoption. The controller
requires these progress records as well as the validation and review below.
Do not resume an older binary after writing the new fields.

## Coordinator loop

The invoking Codex session remains the coordinator. After each terminal action,
call `next`, carry out its bounded action using this contract, and repeat until
completion or an explicitly recorded hold. Do not ask another model to manage
counters, poll CI, or interpret a controller rejection.

`drive` runs the deterministic CI-wait portion of this loop. It polls only
required checks, preserves a per-head deadline, and records deferral when that
deadline expires. It returns to the coordinator for implementation, business
judgment, Jira connector access, or a merge decision. It never decides that a
business criterion is satisfied or silently launches another task.

Run long controller commands through the host's yielding execution tool. Keep
the returned process/session ID, monitor the retained logs, and provide progress
updates. Do not run a second `run-agent`, `run-check`, or `drive` for the same
activity while the first remains active. The database rejects overlapping
claimed agents and operations. The two-agent limit means coordinator plus one
active implementer, reviewer, or escalation session.

## Preflight and queue

Read the repository instructions and the Jira/landing skills. Use existing
read-only tools to establish GitHub/Jira/toolchain access, actual model
availability, and the configured status and scheduling meanings. `init` records
that evidence; it does not fabricate a Jira connection or infer custom fields.

An init request has this shape:

```json
{
  "repo": "/absolute/primary/checkout",
  "github_repo": "bsncraft/businesscraft",
  "epic": "https://your-jira.example/browse/GAIN-100",
  "statuses": {"todo":"10000","progress":"3","review":"10001","done":"10002"},
  "coordinator_model": "gpt-5.6-sol",
  "coordinator_effort": "medium",
  "preflight": "Actual access checks, instruction versions, status mapping and scheduling field evidence",
  "loop_policy": {"max_open_prs":2,"max_stalled_checkpoints":2,"max_implementation_turns":8,"review_mode":"autonomous","feedback_file":null}
}
```

Use actual status IDs, not these example values. Record the coordinator's
actual model even on a mismatch. The response reports a mismatch; a prompt
cannot change the running coordinator model.

`discover` takes `tasks` and an optional `planning_order` array of every delivery
key exactly once. Otherwise scheduling uses priority, due date, rank, then key.
Normalize priority to an ordinal with smaller numbers first, not a Jira custom
field ID. Dates are UTC Unix seconds; `null` means unspecified. `not_before`
represents an explicit execution restriction only. Jira rank must be resolved
into an order-preserving string. Each task is:

```json
{
  "key":"GAIN-101", "parent":"GAIN-100", "subtasks":["GAIN-102"],
  "priority":1, "due":null, "rank":"resolved-rank", "not_before":null,
  "dependencies":[], "member":true, "ownership_clear":true,
  "ownership_evidence":"Fresh Jira, branch, PR and run-record reconciliation",
  "jira_status":"10000", "resolved":false, "blocker":null
}
```

Dependencies use `key`, `code`, `verified`, `evidence`, and `main_commit`.
Verified code prerequisites require an actual commit present on origin/main.
An in-batch prerequisite also requires its recorded main delivery. The
controller detects dependency cycles. External prerequisites do not become
new implementation tasks. List subtasks under their parent delivery task;
separately scheduling a parent and its subtask is rejected.

Before each selection, `refresh` receives the complete frozen task list,
including removed issues with `member:false`. New tasks become follow-up scope.
Changed subtask membership blocks that task for scope reconciliation.
Unstarted tasks that became terminal are preserved. Queue observations expire
for selection after five minutes and are consumed by each claim/resume.

Observe each known run PR with `gh-observe` before selection. The configured
open-PR cap blocks new claims, while `resume` can still recover existing work.
`claim` takes `{"key":"GAIN-101"}` and must name the first eligible task from
`next`. It atomically reserves the issue in the primary checkout as well as
claiming it in the run database. Another run cannot claim the same issue in a
different worker. Unknown ownership is a blocker. Reservations from interrupted
runs are not automatically stolen or cleared.

## Jira bridge

Jira remains on the repository's authenticated connector. The controller does
not extract OAuth tokens, create another authentication route, or send comments.
The coordinator supplies fresh observations from that connector. The controller
validates their structure and permitted transitions; it cannot independently
authenticate a receipt supplied by the coordinator.

Before detailed work, after draft creation, and after final verification:

1. Re-read the issue and available transitions using the Jira skill.
2. Call `jira-prepare` with the parent delivery `key`, target `issue`, observed
   `current` status ID, `resolved`, desired `target`, and `transitions` as
   `[{"id":"transition-id","to":"status-id"}]`.
3. If it returns `already-correct`, do not send a transition.
4. Otherwise call `dispatch` with `{"operation":<returned-id>}` immediately
   before the connector mutation. The stored result becomes `unknown` before
   the request can leave the process. Use only the returned issue and transition
   ID. The API body is `{"transition":{"id":"..."}}`. Do not add fields,
   comments, assignments, or workflow changes.
5. Re-read the issue and call `jira-observe` with `operation`, `current`,
   `resolved`, and `evidence` naming the actual tool response/source.

A timeout is not a failed transition. Re-read and observe before any retry.
`jira-retry` accepts `operation`, freshly observed `current`, `resolved`, and
fresh `transitions`. It allows one retry only when the previous status still
matches and the target remains appropriate. Call `dispatch` again only after
that command prepares the retry. Terminal receipt replays and observations
superseded by newer operations are rejected.

After a failed synchronization, `sync-failed` takes `operation` and a precise
`reason`. It releases the active task under the skill's recovery exception.
It cannot clear an unknown outcome. Before finishing, revisit failed
synchronizations once, observing actual state before any action. An old merge
must never be repeated to fix Jira.

Before transitioning a subtask, call `subtask-record` with `key`, `issue`,
`current`, `resolved`, `criteria` containing that subtask's acceptance IDs, and
`ownership_evidence`. Its status, terminal history and synchronization remain
separate from its parent's. Pre-existing terminal subtasks cannot be reopened.
A subtask cannot become Done merely because its parent merged. Complete the
mapped acceptance and independently reviewed delivery evidence first.

## Brief, worker and snapshots

Once Jira In Progress is confirmed, call `brief`:

```json
{
  "key":"GAIN-101", "tier":"routine", "reason":"Actual risk classification",
  "criteria":[{"id":"AC1","text":"Complete observable acceptance criterion","human_only":false}]
}
```

Supported tiers are `routine`, `complex`, and `high-risk`. Include every
criterion. Keep full sources, boundaries and implementation plans in
`<KEY>/brief.md`; the controller hashes the structured acceptance criteria.
Changing them invalidates existing acceptance and review evidence without
resetting budgets. After binding, `loop-plan` records their starting conditions,
targets and scope. `measure` supplements, rather than replaces, `evidence`.

Call `reserve-slot` with `slot`, then `provision-slot` with `slot` and a
repository-conforming `branch` containing the task key. These use the primary
checkout's worktree helper. Existing slots without this run's reservation are
occupied, even when clean. For a safely recovered or reset owned slot, create
the next task branch from fresh origin/main and call `bind` with `key` and
`slot`. `bind` verifies repository ownership, a clean checkout, branch identity,
and the starting main commit. Never use it to adopt another worker's code.

An unbound prepared follow-up may reuse a slot from a named completed task only
when the current user explicitly authorizes that exact task and slot. Preserve
the old branch, reset the run-owned checkout through the repository helper,
create the new task branch from fresh `origin/main`, then use:

```json
{
  "key":"GAIN-101",
  "slot":"worker7",
  "historical_slot_reuse":{
    "historical_key":"GAIN-102",
    "source":"Actual current-user message authorizing this task and slot",
    "artifact":"/absolute/ignored/run/historical-slot-approval.md"
  }
}
```

Call `bind` with that request. The controller keeps the unfinished-owner,
reservation, clean-checkout, branch and fresh-main checks. It also requires a
prepared delivery with no current launch, operation, evidence, review, plan,
measurement, checks or PR. The named historical task must have a confirmed
completed delivery for the slot. This exception does not make `provision-slot`
overwrite an existing checkout.

If main advanced after follow-up preparation, this authorized bind refreshes
only the unbound prepared base, current snapshot and an unused current
extra-cycle snapshot. The original task baseline, archived proof, delivery
history, counters, deadlines and launch records do not change. The controller
stores the task, delivery, slot, named historical task and receipt digest in
schema10. It checks the receipt again before a launch. A later prepared delivery
archives this receipt and may use the same continuing user instruction through
a new exact bind request; the exception never carries into a delivery by itself.

After coherent commits, call `snapshot` with `key`. It reads Git directly and
binds evidence to base SHA, head SHA and requirements hash. Uncommitted changes,
branch drift and foreign/symlinked worktrees are rejected. Changed snapshots
invalidate review and acceptance evidence. Preserve narratives in
`<KEY>/evidence.md` and `<KEY>/review.md`; do not maintain parallel counters there.

## Models and validation

Prefer `run-agent` when the built-in runner can provide the required tools.
It claims the budget before startup and controls model, effort, sandbox, session
reuse and standard speed. It requires saved ChatGPT authentication, removes
API-key environment overrides, and does not change global settings.

```json
{
  "key":"GAIN-101", "role":"implementer", "model":"gpt-5.6-terra",
  "effort":"medium", "reason":"", "ci_repair":false,
  "prompt":"/absolute/ignored/run/GAIN-101/implementer.txt",
  "timeout_seconds":3600
}
```

Roles are `implementer`, `reviewer`, and `escalation`. Supply the exact authority,
worktree, base/head, requirements, relevant skills and required result in the
prompt file. Reviewers receive a fresh role session without implementation
conversation. The runner resumes the recorded session for later repairs or
verification. It disables recursive agent delegation for its child invocation.
Reviewers use a read-only filesystem sandbox. The built-in runner records actual
JSON events and reported usage, never inferred dollar costs.

If the host's native subagent tools are needed, use the same fields with
`agent-begin`, omitting `prompt` and `timeout_seconds`. This reserves a launch
and returns the required model, effort, role, snapshot and existing session.
Then use actual runner controls to launch or resume exactly that role, passing
the returned `context` unchanged. It contains the pinned plan, scoped feedback
and previous progress checkpoint. Each launch stores that packet for audit. After
its actual turn is terminal, call `agent-finish` with `launch`, actual `session`,
`outcome` of `completed`, `failed` or `cancelled`, `review`, and `usage`.
`review` and `usage` may be null. Never claim a session ended merely because
its output has been quiet. After an implementer turn ends, preserve coherent
commits, record `snapshot`, then `checkpoint` before another agent turn. Native session terminal receipts remain the host's
responsibility. Do not call both `agent-begin` and `run-agent` for one launch.

Review results require `snapshot`, `verdict`, `findings`, and `evidence_gaps`.
The snapshot has `base`, `head`, and `requirements`. Verdict is `PASS`,
`CHANGES_REQUIRED`, or `BLOCKED`. Findings use `id`, `impact`, `category`,
`location`, `trigger`, `consequence`, and `correction`. PASS requires no open
findings/gaps, current passing evidence and satisfied measurements for every
criterion, and a current scope checkpoint for changed code.

Use `disposition` with `key`, `finding`, `decision` of `accepted`, `declined`,
or `corrected`, and concrete `evidence`. Reviewer verification is still required
after corrections or declined material findings. Model escalation does not
reset counters. Astra used as reviewer consumes both review and escalation
budgets, and requires a prior Sol review.

`run-check` executes an argument array without shell interpolation:

```json
{
  "key":"GAIN-101", "criteria":["AC1"],
  "argv":["bun","run","test:unit:file","--","path/to/test.ts"],
  "cwd":"/absolute/primary/.worktrees/worker1/web/apps/web",
  "timeout_seconds":3600, "implementation":["repo-relative/source.ts:25"]
}
```

Use the owning test skill's actual commands. A zero exit code records a passed
command result only if the snapshot stayed unchanged. The coordinator/reviewer
must still assess whether the test demonstrates the claimed criterion.
Checks cannot satisfy human-only criteria.

For source inspection, external runtime proof, or user-supplied human acceptance,
use `evidence` with `key` and an `evidence` object containing `criterion`,
`status`, `kind`, absolute `artifact` path, `implementation` references,
`description`, and `human_source`. Status is `passed`, `failed`, `skipped`, or
`unavailable`. Kind is `command`, `inspection`, or `human`. `human_source` is
null except when citing actual acceptance supplied by the current user. Never
manufacture this attestation. Artifact hashes are rechecked before PASS, merge,
Done preparation/dispatch and completion. Replacing evidence invalidates review.

## GitHub, completion and cleanup

Create one draft PR at a safe committed checkpoint, even if implementation is
still incomplete. Call `gh-prepare` with:

```json
{
  "key":"GAIN-101", "action":"create-pr",
  "title":"feat(scope): delivered behavior (GAIN-101)",
  "body":"/absolute/ignored/run/GAIN-101/pr-body.md", "method":null
}
```

Call `gh-run` with the returned `operation`. It looks for an existing PR before
creation, checks branch/head identity, persists dispatch, invokes `gh`, and
reconciles the result. It will not blindly rerun an uncertain mutation. After
creation, synchronize Jira Under Review immediately.

For `ready`, use `gh-prepare` with `key`, `action:"ready"`, and null `title`,
`body`, `method`, then `gh-run`. Mark coherent drafts ready so required CI can
actually start. `poll-checks` takes `key` and reads required checks; `drive`
handles bounded waiting. Only actual pending required checks on a ready PR
start the 30-minute deadline. Missing check configuration fails closed. Polling
never resets a deadline or attributes checks to a different snapshot.

If the user selected human-review mode, record an actual human review of the
current snapshot with `human-review` first. Autonomous mode adds no human gate.
For merge, use `gh-prepare` with `action:"merge"` and `method` set to the
repository's actual default `merge`, `squash`, or `rebase`, with null `title`
and `body`. Run `gh-run`. It rechecks evidence, the live head, and required
checks; uses `--match-head-commit`; never uses administrator bypass; and verifies
GitHub MERGED plus the merge commit on freshly fetched origin/main.

Use `gh-observe` with `key` after interruptions or external changes. Queued or
auto-merge requests are unfinished until MERGED is observed. If a command
failed without taking effect, `gh-resolve-failure` takes `operation` and
`evidence` proving the caller terminated and explaining the failure. It re-reads
remote state before permitting another attempt. Do not use it to erase a
queued or otherwise ambiguous merge.

After final acceptance, `final-verify` takes `key`, `main_commit`, and an
absolute final-verification `evidence` artifact. It checks existing acceptance,
review and main ancestry. For already-satisfied work, this identifies the
reviewed main head without an empty PR. Then synchronize the parent and each
subtask to Done. Call `complete` with `key`; it refuses incomplete acceptance,
review, main delivery, parent synchronization or subtask synchronization.

`cleanup-check` takes `key` and exposes exact allowed slot/path details.
`cleanup` takes `key` and `delete:false` to reset for reuse or `delete:true`
to preview and then delete through repository helpers. The wrappers verify
ownership and postconditions. They do not scan unrelated worker ports or invoke
installer, release, deployment or infrastructure workflows. Helper failures
preserve the slot and operation records.

## Explicitly approved pair

`authorize-budget-adjustment` records an actual current-user instruction after
run-owned work has been held `needs-human`. Use `mode:"additional-cycle"` for
one implementation and review pair. Use `mode:"documentation-cycle"` with the
exact existing Markdown paths for a path-scoped pair. The command also requires
`key`, `source`, an absolute receipt `artifact`, and a bounded `scope`.

A failed or cancelled launch spends its half of the pair. The receipt digest
registers once for the task, requirements changes invalidate it, and only one
unused pair may exist. A pair adds capacity without resetting ordinary counters,
stall detection, CI repair, or escalation limits. It does not waive acceptance,
independent review, GitHub checks, or external-operation rules.

## Holds and recovery

`hold` takes `key`, `outcome` of `deferred` or `needs-human`, and `reason` with
an exact next action. CI deferral requires the deadline to have expired.
Human acceptance, missing decisions, ownership and exhausted budgets use
needs-human. Unknown operations and running agents must be reconciled first.

After refreshing the queue, `resume` takes `key` and `final_revisit`. Set the
latter true for the one final revisit before reporting. Counters, artifacts,
sessions and per-head deadlines survive this operation. Do not create another
run to obtain more budget.

The built-in runner puts each command in an owned process group and persists
its PID/start identity before releasing a startup gate. A supervisor crash
before that release cannot execute the requested command. `recover-agent`
takes `launch` and `terminate`; `recover-operation` takes `operation` and
`terminate`. False inspects termination; true may stop only the verified owned
group. Surviving descendants require their inherited ownership marker. On Linux,
`pgrep` is used only with the recorded group ID for that recovery check.

After agent recovery, use the actual recorded session with `agent-finish` and
a failed/cancelled outcome. A startup that never released its gate is recorded
as failed-before-start without inventing a session. Interrupted checks never
produce passing evidence. For GitHub mutations, process termination alone does
not establish whether the remote action succeeded: observe GitHub afterward.

For interrupted `provision-slot` or `cleanup`, first recover the owned process,
then call `reconcile-helper` with `operation`. It checks the selected checkout
and reservation postconditions without rerunning the helper. Partial work is
preserved and marked failed for inspection. Unknown helper startup before a
released gate is safely recorded as not executed. A reset with no successful
helper receipt must not imply that dependency installation completed.

`provision-slot` checks the unfinished-task slot guard before it starts the
repository helper. For an older uncertain provision that already created the
requested checkout, `reconcile-helper` records the actual clean state, branch,
HEAD and `origin/main`. If another unfinished task still owns that slot, it
records the provision as failed, preserves the checkout and existing ownership,
leaves the current task unbound, and permits a different available slot. Other
binding failures remain unsettled errors for explicit diagnosis. A complete
provision replay requires the operation task to remain active. If that task is
already bound, reconciliation confirms the operation only when its recorded
slot matches the operation slot exactly and no other unfinished task owns it.

## Enforcement boundary and validation

The controller enforces its transactions, budgets, runner parameters, process
claims and operation preconditions. Jira/native-agent observations and human
acceptance citations are trusted coordinator inputs. It does not prove business
correctness or authenticate a human from a text string. A filesystem sandbox
alone does not remove all network credentials. An agent or user with unrestricted
shell access can bypass this tool or edit its files; this is not an adversarial
security boundary. The skill therefore requires all governed actions to use
these commands and forbids workers from modifying controller state.

For controller maintenance, follow [the Rust quality gates](rust-quality-gates.md)
and run the canonical command from any directory:

```sh
"$EPIC_SKILL/controller/quality.sh"
```

This includes formatting, strict Clippy, per-function cyclomatic complexity,
nesting and size limits, behavioral tests, gate tests and the locked release
build. The reference defines test review and explains what each gate can prove.

Tests use isolated Git repositories and fake GitHub/Codex executables. They
exercise actual CLI and process boundaries without model billing or live
Jira/GitHub mutation. Live connector availability and human-only acceptance
remain per-run prerequisites, not claims made by these tests.

## Unified budget adjustments

Use `authorize-budget-adjustment` for every new pair:

```json
{
  "key":"GAIN-101",
  "mode":"additional-cycle",
  "source":"Actual current-user message approving one more repair and review",
  "artifact":"/absolute/ignored/run/user-approval.txt",
  "scope":"Correct the remaining transaction finding; requirements are unchanged",
  "paths":[]
}
```

Hold the run-owned unfinished task first. `additional-cycle` requires exhausted
ordinary review capacity or prepared follow-up work. It grants one implementation
turn and one review. A later pair needs a fresh receipt after the current pair is
spent.

`documentation-cycle` uses the same fields with nonempty `paths` naming existing,
repository-relative, non-executable Markdown files. Its launches do not count
against ordinary implementation or review limits, and it does not clear an
already exhausted budget. The controller checks every intermediate commit, so a
reverted source change still blocks the scoped pair. Full acceptance remains
required.

Retroactive documentation credit has been removed. Do not edit counters or
SQLite to reclassify a pair after it runs.

## Atomic criterion results

`evidence-batch` records measurements and evidence in one transaction:

```json
{
  "key":"GAIN-101",
  "snapshot":{"base":"<base>","head":"<head>","requirements":"<requirements>"},
  "results":[{
    "observed":"The saved contact appears after reload, compared with the absent base behavior",
    "satisfied":true,
    "evidence":{
      "criterion":"AC1","status":"passed","kind":"inspection",
      "artifact":"/absolute/ignored/run/contact-proof.txt",
      "implementation":["<actual source reference>"],
      "description":"Observed persistence through the owning runtime",
      "human_source":null
    }
  }]
}
```

Each criterion occurs once. The same proof artifact supports its measurement and
evidence. Every existing guard still applies, including current snapshot,
criterion identity, artifact hashing and human-only evidence. Any invalid entry
rejects the whole batch without partial records. Successful recording invalidates
previous review and human acceptance, just like the individual commands. Failed,
skipped and unavailable evidence remains representable; recording it never
manufactures acceptance. Reuse unchanged executable proof for a documentation
change only with an explicit explanation and current snapshot assessment.

## Usage and machine output

Prefer actual per-turn usage supplied by the runner in `agent-finish`. When a
native runner does not expose it, register its log with `usage-bind` for automatic
terminal import. `usage-import` remains the explicit recovery/overhead path:

```json
{
  "request":{
    "session":"<actual session id>",
    "artifact":"/absolute/session.jsonl",
    "start_line":1,"end_line":120,
    "launch":42,"category":"implementation"
  }
}
```

Call `usage-import` with that request. Select the exact launch interval from
session event boundaries, using one-based inclusive line numbers. Resumed turns
start after the previous imported interval. The importer subtracts the last
cumulative counter before `start_line` from the last within the interval; it
never sums cumulative events or adds reasoning tokens to output. The first
interval starting at line1 uses zero as its baseline. A later interval requires
a preceding counter. Session identity, terminal launch/session binding, interval
bounds, category and overlap are checked. A launch already carrying usage cannot
receive a second accounting record. Missing counters remain unknown.

Use `launch:null` and `category:"coordinator"` or `"controller-maintenance"` for
sessions outside the task launch ledger. Task sessions cannot be relabelled as
coordinator overhead. Task categories are `implementation`, `repair` after an
earlier reviewer launch, `review` and `escalation`. `usage-report` reads state
without changing it and returns category totals and missing launch IDs. Imported
launches contribute through the launch once, not again through the import receipt.
Receipts retain counters and a digest of the observed interval, not conversation
text or credentials. Session files may continue growing after an import.

Record coordinator and maintenance intervals at handoff when available. Do not
wait for telemetry or change product acceptance because it is unavailable.
Native usage is normalized at settlement. Malformed optional usage produces a
warning and remains unrecorded, so it can be imported later without blocking
agent settlement. An omitted cache count remains null, including in aggregate
totals that contain any unreported cache count.

Cached input is part of input. Subscription token records do not establish dollar
costs; `billing_cost` remains null.

`review-schema` returns the exact schema used by the bundled runner. Supply it to
native reviewers with their frozen snapshot. `validate-review` takes
`{"snapshot":{...},"review":{...}}` and rejects extra fields, wrong identities,
duplicate findings and PASS with gaps before settlement. It does not launch an
agent, consume a review attempt or grant acceptance. Correct serialization from
the existing reviewer result without inventing findings or changing its verdict.
`agent-finish` remains the authority for freshness and acceptance enforcement.

Write complete machine responses to an ignored file before parsing them. For
example, redirect controller stdout to a response file and stderr to a separate
error file, check the exit status, then parse the response locally and display a
small summary. Never JSON-parse a host tool's truncated display or error text.
The batch response returns criterion IDs rather than repeating long evidence.

## Completed delivery observations

After verified completion and Jira synchronization, `gh-observe` checks the
remote PR/head/merge identity and presence on main without requiring its former
worker HEAD. Reset, reuse or removal of that worker does not invalidate completed
acceptance or overwrite its cleanup next action. Unfinished external merges
still require current acceptance and independent review; they cannot take the
completed-delivery path. Remote head or merge-identity drift fails closed.

## Follow-up after a verified merge

Use `prepare-followup` only for an idle, held, run-owned task whose PR is verified
merged but whose full acceptance remains unfinished. It handles an absent old
worker without treating that checkout as fresh. It writes schema7 only when
preparation succeeds; reading or building an older run does not migrate it.
Older binaries reject schema7 in both read and write paths.

See [the follow-up recovery contract](followup-recovery.md) for the request,
archive layout, slot replacement and budget sequence. Explicit staged acceptance
is a separate command contract below.
`gh-observe` now accepts an optional `pr` number. Omit it for current delivery;
name an archived PR to verify its immutable remote identity without changing
current delivery or settling any operations. Unknown PR numbers are rejected.

## Closed PR replaced externally

Use [superseded recovery](superseded-recovery.md) for `recover-superseded` and
`prepare-approved-cleanup`, their schema9 records, and narrowly scoped reuse of
an existing exercise approval across preparation and cleanup. These commands
preserve closed PR history and full task acceptance.

## Explicit staged acceptance

Schema8 adds stage history and native usage-source bindings. Schema1 through
schema7 still load without migration; only a successful new feature write needs
schema8. Both Store read and write paths reject newer schemas. Old binaries
reject schema8. Reading `review-readiness` or `reconcile-plan` changes no state.

`begin-stage` takes the following request after the task is active and bound,
before its PR merges, with no live agent or pending/uncertain operation:

```json
{
  "request": {
    "key": "GAIN-123",
    "source": "Actual user message approving this sequencing",
    "artifact": "/absolute/ignored/stage-authorization.md",
    "requirements": "<unchanged full task requirements hash>",
    "scope": "Temporary diagnostic code and its removal plan",
    "operational_proof": "Exact later runtime outcome and verification",
    "cleanup_plan": "Removal PR, remaining allowance and restoration proof",
    "criteria": [
      {"id":"STAGE1","text":"Diagnostic implementation and executable guards verified","human_only":false}
    ]
  }
}
```

Stage criterion IDs must differ from full task IDs. The controller preserves
`Task.criteria` and `Task.requirements`; the active stage has separate criteria
and a receipt-bound revision in the current snapshot. This revision identifies
evidence, plan, review, human acceptance and GitHub operations for that stage.
The launch context includes both full and stage criteria. Never manually substitute
a weaker brief. Changing the authorization artifact blocks dependent work.

Beginning a stage invalidates current acceptance/review/plan, so prepare a stage
`loop-plan` and fresh measured evidence. The original task baseline remains fixed.
Record a current checkpoint when changed source requires it. Run readiness and
independent review, then the normal exact-head GitHub operations and required
checks. A stage PASS permits that merge only. Counters, findings, CI deadlines,
consumed exceptions and delivery history remain intact; beginning or ending a
stage grants zero agent launches or live-operation authority.

After the merge is verified, `end-stage {"key":"GAIN-123"}` archives the stage
review, evidence and merge and restores the full-criteria snapshot identity. It
invalidates current proof and review. `final-verify`, Jira Done and `complete`
remain blocked until full task acceptance, including native proof and cleanup,
has current evidence and independent PASS. The ended stage's reviewer may review
full acceptance after merge within the remaining task budget; implementations
still require a separately authorized follow-up delivery. If the ended stage
exhausted the review allowance, hold and use `authorize-budget-adjustment` with
`mode:"additional-cycle"` and a fresh actual user receipt for the final review.
For this merged ended-stage case the response grants zero implementation turns
and one review; implementation remains blocked. Consumed receipts are archived
and cannot be replayed. Missing allowance is not solved by starting another stage.

For cleanup/correction code, end the stage, hold, refresh and `prepare-followup`
using the existing recovery contract. Preserve all original criteria. Bind the
new owned worker, grant only the actual authorized cycle, and use full acceptance
or another explicitly authorized stage as appropriate. Reuse earlier evidence
only after identifying its source/digest, explaining why it still applies and
recording a current measurement for independent review. An old stage PASS never
automatically satisfies final acceptance.

## Review readiness and settlement receipts

`review-readiness {"key":"GAIN-123"}` reports the first current acceptance
blocker and snapshot without launching an agent or mutating state. Ordinary
`agent-begin` and `run-agent` reviewer requests enforce the same check before
incrementing counters. Resolve missing proof first. For a deliberate evidence-gap
investigation, both commands accept `allow_evidence_gaps:true` with a specific
`reason`. This consumes the ordinary review allowance and cannot PASS without
all required evidence. Other roles cannot use the flag.

The CLI saves a valid native review result to immutable
`native-review-<launch>-<digest>.json` before attempting `agent-finish` settlement.
A saved result is not an accepted review. If settlement rejects changed evidence,
preserve the claim and original result, restore/reconcile the same reviewed
artifacts, and retry the same launch. Do not relabel a real PASS as a failed
review merely to free bookkeeping, or spend another invocation to repeat it.
Changed source, criteria or genuinely new evidence still need independent review;
never fabricate acceptance from the saved result. Managed runner result files
already preserve the native terminal output. Ordinary process-recovery guards
remain binding.

## Automatic native usage capture

After a native launch returns its actual session identity, register the exact log
and start boundary before settlement:

```json
{
  "launch":42,
  "source":{
    "session":"<actual native session id>",
    "artifact":"/absolute/session.jsonl",
    "start_line":121,
    "end_line":null
  }
}
```

Call `usage-bind` with this payload. The first launch in a fresh session may start
at line1. A resumed turn needs its own start line after the preceding turn's
cumulative counter. Match actual native session/turn boundaries, not guessed
wall-clock times. The source identity is validated and recorded once. On
`agent-finish`, an absent per-turn usage value triggers best-effort import of the
bound interval. Its end is frozen at settlement, so later session growth cannot
add another turn's tokens. A known exact end may be supplied instead of null.

`usage-sync` retries only missing usage for terminal launches with fixed registered
boundaries. It never expands an old interval or counts an imported launch twice.
Unavailable/invalid logs produce unknown-usage diagnostics without changing the
successful delivery outcome. If no terminal boundary was captured, use explicit
`usage-import` with proven bounds. A terminal launch may also be bound for the
first time when both bounds are explicit, then synced. Imported intervals use the
existing counter subtraction, session/category validation and overlap guards.

At handoff: `usage-sync`, explicitly import the available coordinator interval,
then `usage-report`. Report remaining unknown launch IDs. Do not present an empty
report as evidence that accounting succeeded. No cost is inferred from tokens.

## Bounded reconciliation

`reconcile-plan {"key":"GAIN-123"}` is read-only and returns `inspect` with
reasons plus reusable completed PR/merge receipts. It selects the current task,
relevant unsettled prerequisites, open PRs, failed synchronization and uncertain
operations. Without a key, it also includes all unfinished known deliveries for
one restart/final reconciliation. Completed unrelated PRs are not reobserved by
default. Follow the selected task's Jira refresh/ownership rules and explicitly
observe external changes or contradictory completion facts. This plan performs
no GitHub calls and replaces no required exact-head or operational readback.
