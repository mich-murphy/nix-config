# Progress and feedback contract

Read during preflight and use alongside [controller.md](controller.md). These
commands extend the same SQLite state and guarded delivery operations. They do
not start an epic, grant external authority, change CI policy, or replace
business acceptance with a count.

## Run policy and older runs

`init` accepts this optional `loop_policy`; omitted fields take these defaults:

```json
{
  "max_open_prs": 2,
  "max_stalled_checkpoints": 2,
  "max_implementation_turns": 8,
  "review_mode": "autonomous",
  "feedback_file": null
}
```

Limits must be positive. Set them from the user's constraints and expected work
at preflight. Policy freezes at the first claim. Never create another run or
weaken the policy to bypass an exhausted limit. The implementation-turn limit
counts all implementer launches, including cancelled or failed starts, repairs,
model transfers and continuations. Review and escalation retain their separate
existing limits. Exhaustion still permits validation and review of existing
work; unfinished implementation becomes `needs-human` with its diagnosis.

Only select `human-review` when the user requests human review before delivery.
It adds an exact-snapshot human decision without removing independent Codex
review. The default remains autonomous merging under the existing conditions.

Existing databases without `loop_policy` still open for status and recovery.
`next` returns `configure-loop`. Call it with `{"policy":{...}}` in the original
run directory, then add missing plans and measurements. Existing launches,
review counters, deadlines and reservations survive. The first successful
mutation upgrades the state to schema 2, which the earlier binary refuses.
Do not fabricate a new
baseline from current code; inspect the original base revision. Do not run an
older controller binary on state written by this version.

Before each new claim, use `gh-observe` for all known run PRs. Open drafts,
deferred PRs and needs-human PRs all count against the cap. The controller
counts this run's recorded PR observations, not every PR in the repository.
At capacity, `next` returns `recover-unfinished-prs`. Observe outcomes and
resume existing work, or report a hold if nothing can proceed. Never close an
unfinished PR merely to release capacity. `resume` remains available at the cap.

## Baselines and local examples

After `brief` and binding the worker, call `loop-plan` before launching an agent:

```json
{
  "key": "GAIN-101",
  "plan": {
    "baseline_commit": "<original task base SHA>",
    "family": "web-screen",
    "deliverable": "Allow authorized users to create a customer contact",
    "components": ["web/apps/web/app/(app)/contacts", "web/packages/contacts"],
    "examples": ["<approved peer implementation at a named revision>"],
    "baselines": [{
      "criterion": "AC1",
      "starting_condition": "The contact creation action is absent at the base revision",
      "target": "A valid contact persists and appears in the customer contact list",
      "method": "Exercise creation and reload through the owning runtime boundary",
      "artifact": "/absolute/ignored/run/GAIN-101/baseline-AC1.txt"
    }]
  }
}
```

Include each criterion exactly once. Use actual repository paths and local
examples; the example above is a schema illustration. Explain in `brief.md`
when no appropriate existing example exists. `components` names whole files or
directory prefixes. The controller records the actual changed paths and diff
size later; these expectations do not authorize an unrelated change.

For a follow-up, the review snapshot uses the new delivery base while the
baseline commit and criterion baselines remain at the original task revision.
See [follow-up recovery](followup-recovery.md).

The controller verifies the baseline commit equals the task's original base,
hashes the baseline artifacts, and freezes the starting conditions for those
criteria. Changed requirements require a revised `brief` and plan; they never
reset budgets. A plan update invalidates acceptance and review. Preserve the
old proof artifacts for audit. A legitimate scope reassessment must retain the
accepted deliverable and explain any necessary extra components.

## Measurements and checkpoints

`measure` takes `key`, `criterion`, `observed`, `satisfied`, and an absolute
`artifact` path. It records the current snapshot and artifact digest. Describe
what changed relative to the baseline, including unchanged or regressed
behavior. Mark `satisfied:true` only when the target is demonstrated. A count
reduction alone cannot establish preserved behavior or absence of new defects.
For inventory work compare stable item identities, removals and additions.

Use `run-check` or `evidence` as well. Measurements explain the observed outcome;
validation records establish its proof. The same real artifact may support
both when appropriate. Command success never sets `satisfied` automatically.
Human-only criteria still require actual human evidence. Changed snapshots or
artifacts make previous measurements ineligible for acceptance. Changing a
measurement invalidates review and any human-review receipt.

After each terminal implementer turn, commit coherent work where possible,
record `snapshot`, and call `checkpoint` before another agent launch:

```json
{
  "key": "GAIN-101",
  "advanced": true,
  "finding": "The failed save is isolated to the repository transaction boundary",
  "next_step": "Correct that transaction and rerun the failed persistence case",
  "artifact": "/absolute/ignored/run/GAIN-101/diagnosis-1.txt",
  "scope_reason": "The action and repository change remain one contact-creation behavior"
}
```

A resolved uncertainty is progress even without a code change. A new SHA,
rewritten status summary, or repeated test output is insufficient. With no new
result, set `advanced:false`. Do not manufacture a commit to obtain a checkpoint.
If the work cannot be safely checkpointed, preserve it and hold rather than
launching another turn. Keep each proof artifact intact at its recorded path.

The controller associates the checkpoint with the latest implementation launch,
records actual changed paths, added/deleted lines and binary-file counts, and
reports paths outside the plan. For any code change, `scope_reason` must assess
whether it remains a small, complete deliverable. An out-of-plan path requires
an explanation of its necessity within existing scope. Line counts are signals
for judgment, not fixed merge thresholds. If the task is too broad, hold with a
concrete decomposition proposal; do not edit Jira scope or omit criteria.

Two consecutive unchanged checkpoints stop further implementation by default.
A fresh, supported observation can clear the streak before the limit is reached.
Duplicate proof content cannot claim new progress. Scope-only reassessments of
integration changes may use a new snapshot without another agent launch; they
never alter stall counters. Replanning and resumption do not clear them.

PASS requires satisfied measurements and validation for every criterion. Changed
code also requires a checkpoint covering the current snapshot. Integration
changes need renewed measurements, scope assessment and affected validation.
The coordinator and independent reviewer assess the meaning of the evidence;
the controller authenticates neither natural-language judgments nor human
identity from a string.

## Persistent scoped lessons

`lesson-record` takes `key` and a `lesson` with this structure:

```json
{
  "id": "contact-lookups-use-shared-metadata",
  "families": ["web-screen"],
  "source": "<review finding and binding repository rule>",
  "instruction": "Use the shared reference metadata for contact lookups",
  "example": "<approved implementation path and revision>",
  "status": "proposed",
  "acceptance_source": null
}
```

Record proposals after dispositioning findings. Accepting a lesson requires
`status:"accepted"`, an `acceptance_source` explaining its evidence-based
disposition, and final verified delivery of the source task on main. This can
reinforce existing binding instructions; introducing a new business convention
or changing authority requires a real human decision. Do not treat bot comments
as authorization. Record accepted lessons before cleaning the source worker.

Accepted lessons are immutable. Use `status:"retired"` with the evidence-backed
reason from a subsequently verified task to retire a mistaken lesson, then
propose a corrected ID. Proposed lessons can be refined before acceptance.
The controller records source task and main commit. Subsequent task plans load
accepted run lessons matching `family`, or the explicit `*` family. Proposed
and retired lessons are excluded. Avoid broad `*` guidance unless it applies
across task families. Do not accumulate one-off review commentary as rules.

For lessons shared between separate runs, configure `feedback_file` as a
repository-relative path to a committed JSON array of the same lesson objects.
Read an existing library at preflight when available. Creating or changing a
shared library follows ordinary repository delivery rules; do not quietly add
policy changes to a product PR. When no library exists, leave the setting null.
Run-local learning remains active without it. Export genuinely reusable lessons
for a later reviewed library change rather than creating an empty library.

The controller reads the library at `origin/main` when preparing each task plan.
Ensure main was fetched during normal task selection. Missing or malformed
configured files fail closed. The packet records the file revision, digest,
selected library lessons and accepted run lessons. It stays pinned for that task;
changing feedback intentionally requires replanning and renewed review. Every
launch saves the packet. Native `agent-begin` returns it in `context`, which the
coordinator passes unchanged. `run-agent` inserts it into the actual prompt.
Reviewers receive the same rules and examples without implementer conversation.
Feedback provides guidance; it never overrides repository instructions,
acceptance criteria, budgets or the user's authority.

## Optional human review

After independent PASS and before merge, `human-review` takes `key`, the exact
`snapshot` object, `source` naming the actual human and decision, and an absolute
`artifact` containing its receipt. A model verdict is not a human receipt. The
controller binds and hashes it, then checks it again at merge and completion.
Already-satisfied tasks in this mode also require human review of the inspected
main snapshot. A stale or changed receipt fails closed.

When `next` returns `await-human-review`, prepare the concrete reviewed PR or
main evidence for the user. Required checks can still be observed with
`poll-checks`; do not start another implementation on that basis. If the user is
unavailable, record `needs-human` and apply the ordinary unfinished-PR cap.
Only this selected mode introduces that review stop. Autonomous mode retains
standing merge authority and all existing human-only acceptance boundaries.

## Cross-cutting lesson selection

`loop-plan.plan` optionally accepts `lesson_families`, a list of additional
applicable tags alongside its primary `family`. For example, a permission task
that changes operator instructions can name `reference-propagation`. Accepted
run/library lessons matching any selected tag are included once in the pinned
packet. Unrelated, proposed and retired lessons stay excluded. Adding tags to an
existing plan is replanning and invalidates acceptance/review without resetting
budgets. This optional field writes schema6; omit it for unchanged legacy plans.
