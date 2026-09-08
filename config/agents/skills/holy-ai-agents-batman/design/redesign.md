# Controller and skill redesign

Status: proposal, revision 2, not yet implemented.
Date: 2026-09-08.
Scope: the `holy-ai-agents-batman` skill and its bundled Rust controller.

This document collates the review findings, the decisions taken on them, and the
resulting design. It is the reference for the rebuild. It supersedes the earlier
incremental porting plan.

Revision 2 incorporates the review of revision 1. Jira stays on the
coordinator's connector (no controller credential). The phase enum carries only
phase-exclusive data; budgets, synchronisation and holds sit beside it.
Stages, follow-up and superseded recovery become one delivery history. Events
record outcomes and are folded, never re-executed. Tests are re-scoped to
behaviour at a public boundary. Typing lands before the crate split.

## 1. Why rebuild rather than patch

The controller works. One real run (GAIN-847, 51 tasks, 109 agent launches,
2,324 events over 2.5 days) produced 15 autonomously merged tasks with
independent review and confirmed Jira synchronisation. Correctness is not the
problem.

Three structural defects limit it:

1. No I/O boundary. `runtime.rs` is 1,651 lines and is simultaneously process
   executor, GitHub adapter, agent launcher, slot provisioner and policy
   participant. `merge_pr_command` calls `engine::reviewed` and
   `loops::human_review` before emitting argv. There is no trait for any
   external system and fifteen bare `Command::new` sites: twelve in
   `runtime.rs` and three `git` calls in `store.rs`.
2. No typed state machine. The delivery lifecycle exists only as a 90-line
   if/else ladder (`engine::task_action`) returning `&'static str`, which the
   coordinator then string-matches. Stages, follow-up and superseded recovery
   were each added by threading another branch into that ladder.
3. No policy/mechanism split. `budget.rs` clones the whole `State`, re-dispatches
   a legacy input into the engine, and overwrites on success. A transaction
   inside a transaction, implemented by whole-state cloning.

Each was introduced incrementally in response to a specific run failure. The
accumulated result is that both the code and the skill prose are reactive
sediment. The rebuild is a re-layering, not a rewrite of the rules.

## 2. Terminology

Domain language stays as-is. Port names are concrete, never invented
abstractions.

| Concept | Name | Notes |
| --- | --- | --- |
| Unit of delivery | Epic, Task, Subtask | unchanged |
| Issue tracker | Jira | no port; facts are supplied by the coordinator from its connector (Section 7) |
| Code host | GitHub | port trait `GitHub`, not "forge"; wraps `gh` |
| Version control | VCS | port trait `Vcs`, wraps `git` |
| Agent runner | Harness | port trait `Harness`, covers Codex and pi |
| One attempt to land or prove work | Delivery | `Code` or `Verification`; a task has one or more; Section 5 |
| Explicit user permission | Authority | one type; a receipt carrying one `Grant` |
| Accepting a criteria subset for one merge | Narrowing | replaces "stage"; produces a merge, never acceptance |
| Binary | `controller` | renamed; a run may deliver a single task, so a tracker-shaped name overstated the scope |

A run may deliver one task or a whole epic. Nothing in the design requires more
than one task, so single-task invocation works without a special case.

## 3. Crate layout

Workspace under `controller/crates/`. Names describe responsibility. No shared
prefix. `core` is unavailable as a crate name because it collides with Rust's
`core`, hence `domain`.

| Crate | Responsibility | Enforced property |
| --- | --- | --- |
| `domain` | types, phases, events, rules, budgets, risk, acceptance; `ports` module holding the traits `GitHub`, `Vcs`, `Harness`, `Clock` | no I/O, no `std::process`, no `serde_json::Value`, no clock, no `unwrap`/`expect`; rules never call a port |
| `adapters` | one module per external system: `github`, `git`, `codex`, `pi`, `sqlite` | all process execution lives here |
| `app` | one handler per command: gather facts through ports, `decide`, `commit` | no argv, no printing |
| `cli` | argv parsing, JSON in and out, help | thin |

Four crates rather than five. Traits are types; defining them in `domain` does
not break its purity, and a separate `contracts` crate would enforce nothing
that the dependency edge `domain` -> nothing does not already enforce. `app` and
`cli` may merge later without losing a property.

The rule that makes the layering hold: `domain` rules receive already-fetched
facts as arguments and return decisions. `app` is the only place a port is
called. This is what makes the rules unit-testable without mocks, and it is why
multi-harness support falls out of the structure instead of being retrofitted
as a config file.

## 4. State representation

Today the lifecycle is inferred on every call from roughly fifteen independently
settable fields (`pr`, `pr_open`, `draft`, `merged_commit`, `main_verified`,
`final_verified`, `review`, `snapshot`, `slot`, `branch`). Contradictory
combinations are representable, and much of the reactive prose in `SKILL.md`
exists to warn the model away from them.

Replace with a phase that carries only what is exclusive to that phase.
Everything monotonic or orthogonal sits beside it:

```rust
struct Task {
    id: TaskId,
    spec: TaskSpec,             // identity, criteria, dependencies, last-read Jira status
    phase: Phase,               // exclusive data only
    hold: Option<Hold>,         // orthogonal; phase records where work stopped
    sync: Sync,                 // Jira synchronisation, orthogonal to phase
    budgets: Budgets,           // monotonic across the whole task
    deliveries: Vec<Delivery>,  // every attempt to land or prove work; current is last (Section 5)
    authorities: Vec<Authority>, // every user receipt; each registers once, each grant is used once
}

enum Phase {
    Queued,
    Blocked(BlockReason),               // dependency on another task in this run only
    NeedsInput(Question),               // surfaced to the user at once
    Claimed,
    Planned  { work: PlannedWork },     // slot, branch, base established
    InFlight { work: PlannedWork, stage: WorkStage },
    Merged   { delivery: DeliveryId, commit: Sha },   // a merge fact; acceptance may remain open
    Verified { receipt: Receipt },      // every criterion accepted; terminal
    Excluded(ExclusionReason),
}

enum WorkStage {
    Building,
    Validating,
    Reviewing  { launch: LaunchId },
    Repairing  { findings: Vec<FindingId> },
    Publishing { pr: PrNumber },
}

struct Hold { since: Instant, reason: HoldReason }

enum HoldReason {
    CiPending    { pr: PrNumber, head: Sha, deadline: Instant },   // was Delivery::Deferred
    NeedsHuman   { diagnosis: String, remaining: Vec<CriterionId> },
    BudgetExhausted(BudgetKind),
    SupersededPr { pr: PrNumber, replacement: Option<Replacement> },
}
```

Why budgets and proof are not inside the phase payloads: review and repair
counts survive `Repairing` -> `Building` -> `Reviewing`, and "escalation resets
no budget" is a rule across the whole task life. Data that must outlive a phase
cannot live in one. Proof and review belong to the delivery that produced them
(Section 5), because a stage's evidence is not the follow-up's evidence.

Consequences:

- `next` becomes an exhaustive `match` over `(phase, hold, sync)` returning a
  typed `NextAction`, not a string the coordinator pattern-matches.
- Adding a phase is a compile error at every site that must handle it.
- `Option<Sha>` for `merged_commit` disappears. Either you are in `Merged` and
  have a `Sha`, or the field does not exist.
- "A merge fact is not completion" stops being prose and becomes the distinction
  between `Merged` and `Verified`.
- "Blocked work retains its actual stage" stops being prose: a hold is set
  beside the phase and cleared on resume, and the phase never moved.

Mapping from the current `Delivery` enum: `Queued` -> `Queued`; `Active` ->
`Claimed`, `Planned` or `InFlight`; `Deferred` -> `hold: CiPending`; `Merged` ->
`Merged`; `AlreadySatisfied` -> `Verified { receipt: Existing { main_sha } }`;
`NeedsHuman` -> `hold: NeedsHuman`; `Excluded` -> `Excluded`.

## 5. Task decomposition

The 46-field `Task` struct dissolves into the shape above plus sub-structs owned
by the module that uses them.

| Sub-struct | Owner module | Fields absorbed |
| --- | --- | --- |
| `PlannedWork` | `domain::worktree` | slot binding, branch, base, snapshot, plan, baselines |
| `Budgets` | `domain::budget` | turns, stalls, checkpoints, reviews, repairs, CI repairs, escalations, adjustments |
| `Delivery` | `domain::delivery` | authority, kind, criteria, paths, base, work, proof, review, outcome, launches, operations |
| `Authority` | `domain::authority` | the six receipt types, their grants and replay rules |
| `Proof` | `domain::acceptance` | evidence, measurements |
| `Review` | `domain::review` | findings, dispositions, computed verdict |
| `PullRequest` | `domain::publish` | number, draft, checks, checks head |

Acceptance test for this decomposition: adding a field to `Review` must touch
`domain/review.rs` and nothing else. Today, adding a field to `Task` touches
`model.rs`, the 50-line `Task::new`, ledger rendering in `store.rs`, and every
module that reads it. The ten `#[serde(default)]` fields declared before the
core fields are direct evidence of that failure.

### Deliveries and authority

Stages (`stages.rs`), follow-up (`followup.rs`) and superseded recovery
(`superseded.rs`) are about 1,200 lines of source and 1,660 of tests. Read in
full they show three shapes, not one, and revision 1 of this section had the
first wrong: a stage never opens new work. `begin-stage` narrows the criteria
the current PR must satisfy, `end-stage` archives that evidence after the same
PR merges, and afterwards only a reviewer may launch, against main, with no
implementer. New code needs `prepare-followup`.

| Shape | Today | Criteria | Diff |
| --- | --- | --- | --- |
| new code | first delivery, `prepare-followup`, superseded recovery, cleanup | full, or a path-scoped subset | new branch from `origin/main` |
| no code | post-stage full acceptance, `AlreadySatisfied` | full | none; the reviewer inspects a main SHA |
| narrowing | `begin-stage` | subset with distinct IDs | the current delivery's PR |

```rust
struct Delivery {
    id: DeliveryId,
    kind: DeliveryKind,
    authority: Option<AuthorityId>,   // None only for the first delivery
    criteria: Vec<CriterionId>,       // full unless narrowed
    paths: Option<Vec<PathGlob>>,     // every commit in base..head stays inside
    base: Sha,
    work: Option<PlannedWork>,
    proof: Proof,
    review: Option<Review>,
    outcome: Outcome,
    launches: Vec<LaunchId>,
    operations: Vec<OperationId>,
}

enum DeliveryKind {
    Code         { pr: Option<PullRequest> },   // first delivery and every follow-up
    Verification { of: Sha },                   // reviewer-only launches, by construction
}

enum Outcome {
    Open,
    Merged    { commit: Sha, at: Instant },
    Replaced  { by: Replacement, at: Instant },  // closed unmerged; a distinct external PR merged at a recorded head and merge commit
    Accepted  { at: Instant },                   // Verification only
    Abandoned { reason: String, at: Instant },
}
```

Acceptance is one rule, and it is the current rule: the task is `Verified` when
the latest closed delivery has `criteria == spec.criteria`, a `PASS` review and
`Outcome::Merged` or `Accepted`. Narrowed deliveries produce merges, never
acceptance. There is no union arithmetic over criteria.

Every receipt in the three modules is `{source, artifact, digest, requirements,
recorded}` plus a purpose. There are six such types and six replay checks with
slightly different chains.

| Today | Purpose |
| --- | --- |
| `followup::Authorization` | open a code delivery |
| `stages::Request` | narrow criteria |
| `ExtraCycle` | one implementation/review pair |
| `DocumentationCycle` | one pair, path-scoped |
| `superseded::Exercise` | two deliveries under one approval |
| `HistoricalSlotReuseAuthorization` | bind a slot a closed delivery used |

```rust
struct Authority {
    id: AuthorityId,
    source: String,
    artifact: PathBuf,
    digest: Digest,
    requirements: Digest,       // of spec.criteria when granted
    recorded: Instant,
    grant: Grant,
    used: Option<UseId>,        // the pair, delivery or binding that consumed it
}

enum Grant {
    Pair      { paths: Option<Vec<PathGlob>> },   // ExtraCycle and DocumentationCycle
    Delivery  { kind: DeliveryKind, criteria: Vec<CriterionId>, paths: Option<Vec<PathGlob>> },
    Narrowing { criteria: Vec<CriterionId>, operational_proof: String, cleanup_plan: String },
    SlotReuse { historical: TaskId, slot: SlotId },
}
```

Rules over `Task.authorities`, replacing the six checks:

- A digest registers once. One exception: an unused `Pair` may be re-registered
  as a `Delivery` grant on the next delivery. This is today's "transfer" in
  `superseded::unused`, without `previous_extra_cycles` archiving.
- A grant is consumed once. A failed pair is consumed.
- At most one unused `Pair` exists at a time. This is "consume or reconcile the
  previous additional cycle first" and "documentation exception cannot authorize
  staging" as one rule.
- `requirements` must equal the current criteria digest at use.
- `Grant::Delivery` includes its first pair. Today `prepare-followup` grants
  zero launches and the same receipt must then be passed to
  `authorize-budget-adjustment`; the interstitial state existed only because
  there were two commands.
- A `Pair` used on a `Verification` delivery spends only its review half.
- Every launch draws on the task-wide `Budgets` as well; a grant adds to a
  budget, it never replaces one.

Decisions recorded here:

- The exercise mechanism is dropped. `superseded.rs` encodes one occurrence: a
  single approval covering a preparation delivery, an external terminal event
  and a path-scoped cleanup. Superseded recovery becomes `Outcome::Replaced`,
  observed from two GitHub reads, followed by an ordinary `open-delivery`;
  cleanup is a second `open-delivery` with its own receipt and `paths`. The user
  signs twice, which every other mechanism already requires and which Section
  10's batched authorisation makes cheap. `Exercise`, `stage_scope`,
  `stage_receipt_reuse` and the outcome-receipt parser are deleted, not ported.
- `superseded::scope` and the documentation path check are one rule: every
  commit in `base..head` touches only `Delivery.paths`, including commits later
  reverted.
- `HistoricalSlotReuse` is a worktree concern. It becomes
  `PlannedWork.slot: SlotBinding { slot, origin }` with
  `origin: Fresh | Reused { authority: AuthorityId }`, plus the rule that no
  delivery binds a slot any closed delivery in any task used, without one.
- `ArchivedDelivery.task: Box<Task>` goes. A closed delivery keeps its own
  proof, review, launches and operations; nothing else was ever read from the
  archive.

Preconditions shared by `followup::validate` and `superseded::held` become the
`open-delivery` preconditions for every delivery after the first: task held
with `NeedsHuman`; run-owned and not terminal before the run; a fresh
single-issue Jira read showing unresolved membership and clear ownership;
unchanged criteria digest; prior delivery closed; history intact, meaning every
`Merged` commit is on main and every `Replaced` PR is still merged at the
recorded head. The 300-second refresh window goes with the TTL (Section 10).
The `op.id > identity(t)` guard becomes structural: operations belong to a
delivery, and a closed delivery's operations are not dispatchable.

Lifecycle under this model: `Merged` with acceptance open holds `NeedsHuman`;
`open-delivery` with a receipt returns the task to `Planned` with a new
`Delivery`. `narrow-acceptance` on an in-flight `Code` delivery replaces
`begin-stage`; observing its merge is the ordinary merge observation, so
`end-stage` disappears. `final_review_allowed` and `guard_launch` disappear
because `Verification` admits no implementer. The phase ladder never learns a
new branch for any of them.

Acceptance test for this section: every kept behaviour in
`tests/improvements/*` and the slot tests in `tests/guardrails.rs` is restated
as a rule over `deliveries` and `authorities` in `design/behaviours.md` before
the types are committed (Section 17).

## 6. Input contracts

One dispatch path. Today there are two: about twenty commands deserialise into a
tagged `Input` enum with `deny_unknown_fields`, and about fifteen go through
`runtime_fields`, a hand-maintained 40-line field allowlist followed by manual
`value["key"].as_str().context("key required")` extraction. The split tracks
"does this shell out", which is an implementation detail the caller should never
see.

A single `Command` enum with `#[serde(deny_unknown_fields)]` on every variant,
matched once. At the reduced command count (Section 7) this is the simplest
thing that works. Help text, dispatch, schemas and exit codes derive from it, so
the hand-written 60-command help string and the allowlist both disappear.

### Typed fields

Parse at the edge; the interior then stops needing defensive checks. This is
where most of the control-flow reduction comes from.

| Field | Today | Proposed |
| --- | --- | --- |
| task key | `String` plus `valid_key()` at 8 sites | `TaskId` |
| slot | `String` plus `valid_slot()` | `SlotId` |
| commit | `String` | `Sha`, 40 hex, parsed |
| model, effort | `String` compared to literals | `ModelId`, `Effort` from a profile |
| authority receipt | six near-identical structs | `Authority` with a `Grant` |
| `Operation.kind` | `String` | `GitHubAction` enum |
| `Operation.details`, `.observation` | `Value` | typed per-variant payload |
| `Task.checks` | `Vec<Value>`, `.as_str()` on `"bucket"` | `Vec<Check>` with `CheckState` |
| `Launch.usage` | `Option<Value>` | `Tokens`, already exists in `telemetry.rs` |
| `dispositions` | `BTreeMap<String, String>` | `BTreeMap<FindingId, Disposition>` |
| `Finding.impact`, `.category` | free text | `Severity`, `Category` enums |
| review verdict | reviewer-declared string | computed from findings (Section 9); not an input |

`Severity` is load-bearing: it decides whether a finding blocks the merge
(Section 9). `ModelId` and `Effort` remove the eight string-equality invariants
in `agents.rs` such as `ensure!(model == "gpt-5.6-sol")`, which are
simultaneously the portability blocker and a live correctness weakness, since a
typo silently reclassifies a launch.

## 7. Command surface

The `prepare` / `dispatch` / `observe` triads exist because the model performs
the side effect: the controller emits argv or a transition, the model runs it,
then reports back so the controller can settle. That accounts for roughly 25 of
the ~60 commands and a large share of `SKILL.md` prose.

Jira stays on this model. The coordinator's MCP connector is already
authenticated and governed by `businesscraft-jira-admin`; giving the controller
its own REST credential would add a second credential surface for no gain in
reliability. So Jira collapses from five commands to two rather than one, and
GitHub and agent launches, where the controller already holds the tool, collapse
fully.

| Group | Today | After | Removed by |
| --- | --- | --- | --- |
| Jira transitions | `jira-prepare`, `dispatch`, `jira-observe`, `jira-retry`, `sync-failed` | `set-status`, `observe-status` | `set-status` records intent and returns the exact transition to perform; `observe-status` settles the fresh read to confirmed, unknown or failed, absorbing retry |
| GitHub delivery | `gh-prepare`, `gh-run`, `gh-observe`, `gh-resolve-failure` | `publish`, `observe-pr` | `GitHub` port |
| Agent launches | `agent-begin`, `agent-finish`, `run-agent`, `recover-agent` | `run-agent` | `Harness` port; begin and finish become internal |
| Usage | `usage-bind`, `usage-sync`, `usage-import`, `usage-report` | `usage-report` | automatic capture (Section 8) |
| Authority | `authorize-budget-adjustment`, `authorize-documentation-cycle`, `reconcile-documentation-extra`, `authorize-extra-cycle`, historical-slot `bind` | `grant` | one `Authority` type (Section 5); `open-delivery` and `narrow-acceptance` register their own receipt |
| Evidence | `measure`, `evidence`, `evidence-batch` | `record-proof` | one command, repeated field |
| Plan and progress | `loop-plan`, `checkpoint`, `configure-loop` | `plan`, `checkpoint` | policy moves to run config |
| Recovery | `begin-stage`, `end-stage`, `prepare-followup`, `recover-superseded`, `prepare-approved-cleanup` | `open-delivery --kind <code\|verification>`, `narrow-acceptance` | delivery history (Section 5); a narrowed merge is the ordinary merge observation; superseded recovery is `observe-pr` recording `Replaced`, then `open-delivery` |
| Risk | self-reported tier in `brief` | `escalate-tier --reason` | computed tier (Section 9); the coordinator may raise, never lower |

Result: about 60 commands become 36. Section 18 lists them. The universal
`--check` flag absorbs `review-readiness` (a rejected `run-agent --check`
spends nothing) and `cleanup-check`; `poll-checks --wait` absorbs `drive`;
`next` absorbs `reconcile-plan`; `bind-slot` absorbs `reserve-slot`,
`provision-slot` and `bind` because the controller owns git through `Vcs`.

`reconcile-documentation-extra` has no successor. It refunded a consumed
source pair whose commits turned out to be documentation only, and existed
because a hard review cap of 3 was being hit by documentation repairs. With
`full` at 5 and warnings not consuming a cycle (Section 10) the pressure that
produced it is gone. Its five rules are deleted, not ported
(`design/behaviours.md`).

Interruption recovery is unaffected. The granularity that matters is the event
log, not the command count. `set-status` still writes an intent event and
`observe-status` a settlement event, and the run is resumable between them.

## 8. Persistence and event log

Keep SQLite. The current problem is not the store, it is that SQLite is used as
a single-row JSON blob with an event table (`seq, at, action, data`) that records
only half of each event. Required properties are atomic "append event plus
update projection", crash safety, one file, no server, and ad-hoc queryability
for audit. WAL-mode SQLite provides all of them and `rusqlite` is already
vendored.

Alternatives considered and rejected: append-only JSONL loses atomic multi-write
so the log and projection can diverge on crash, which is the exact failure this
system exists to survive; `redb` and `fjall` drop the C dependency but lose SQL
introspection; `sled` is insufficiently maintained for a durability-critical
artefact; a server database is the wrong shape for a per-run file in an ignored
directory.

### Events record outcomes, not requests

Revision 1 stored the request and replayed commands through current logic. That
would re-run side effects: replaying `publish` would hit GitHub again. Replay
must fold recorded outcomes, so events are typed facts produced by handlers,
never the commands that produced them.

```rust
// domain: pure
fn decide(state: &State, command: Command, facts: Facts) -> Result<Vec<Event>, Rejection>;
fn apply(state: &mut State, event: &Event);      // total; never fails

// app: one SQLite transaction
fn commit(store: &mut Store, events: &[Event]) -> Result<State>;
```

```rust
enum Event {
    Claimed        { task: TaskId, at: Instant },
    SlotBound      { task: TaskId, binding: SlotBinding },
    LaunchStarted  { launch: Launch },
    LaunchEnded    { launch: LaunchId, result: LaunchResult, usage: Option<Tokens> },
    ProofRecorded  { task: TaskId, delivery: DeliveryId, proof: ProofEntry },
    StatusIntended { task: TaskId, target: JiraStatus, transition: TransitionId },
    StatusObserved { task: TaskId, read: JiraStatus, outcome: SyncOutcome },
    DeliveryOpened { task: TaskId, delivery: Delivery },
    // ...
}
```

```sql
CREATE TABLE events (
  seq     INTEGER PRIMARY KEY,
  at      INTEGER NOT NULL,
  actor   TEXT NOT NULL,     -- coordinator | harness | adapter
  kind    TEXT NOT NULL,     -- event variant name, cheap queries
  event   TEXT NOT NULL,     -- the typed Event, serialised
  phase   TEXT NOT NULL,     -- resulting phase of the affected task
  digest  TEXT NOT NULL      -- hash of resulting state, tamper-evident chain
);
```

Events are authoritative and the state row is a materialised projection rebuilt
by folding `apply`. `status` can verify the projection against a fold.
Compatibility is `#[serde(default)]` on event payloads only, and the rule that
an existing variant never changes meaning; new behaviour is a new variant. The
ten ad-hoc schema versions and the scattered `version.max(n)` calls disappear.

`decide` without `commit` is `--check` (Section 15). The same split is what
makes rules testable without a store (Section 17).

Existing run directories are not migrated. The old events table lacks the data
to fold, and a run in flight finishes on the old binary.

### Usage capture

Usage capture moves into the harness adapter, which parses the agent's own
stream (pi `message_update.usage` and `agent_end`; Codex `--json`) and writes
`Tokens` onto `LaunchEnded`. This deletes three commands, the entire
cumulative-versus-delta arithmetic section of `SKILL.md`, and the observed 79
per cent unknown rate (23 of 109 launches carried usage). The model was being
asked to do bookkeeping it forgets under load.

## 9. Risk tiering

The tier system is retained and moved from model self-report to computed
signals. Evidence for the change: across 109 launches the run recorded 18
high-risk, 4 complex, 0 routine, and zero Terra or Astra launches. The four-tier
table, the Terra-to-Sol transfer rule and the bounded Astra escalation have
never fired. Tier reasons include "read-only operating-document verification"
classified as high-risk, which is defensive classification under prose criteria.

### Cloudflare reference

Cloudflare's "Orchestrating AI Code Review at scale" describes a three-tier
classifier applied to every internal merge request:

| Tier | Threshold | Agents | Reported cost |
| --- | --- | --- | --- |
| trivial | <= 10 changed lines, <= 20 files | coordinator plus one general reviewer | $0.20 |
| lite | <= 100 changed lines, <= 20 files | coordinator plus reduced specialist set | $0.67 |
| full | > 100 lines, > 50 files, or any security-sensitive path | 7+ specialists | $1.68 |

Two mechanisms worth adopting beyond the thresholds:

- Path-based forced escalation. Any change under a sensitive path such as
  `auth/` or `crypto/` receives full review regardless of size. Deterministic,
  computed from the diff, and therefore belongs in Rust rather than prose.
- Severity-gated blocking. Findings are critical, warning or suggestion.
  Critical blocks the merge. Multiple warnings forming a risk pattern revoke
  approval. Isolated warnings permit approval with comments. The current design
  treats every dispositioned finding as merge-blocking, which is a direct
  contributor to the friction in Section 10.

Supporting literature is consistent: review effectiveness declines and
vulnerability likelihood rises with lines changed; broader directory spread
correlates with missed security defects; authentication and privilege weaknesses
dominate real secure-review findings. One caution from the same literature is
that no single metric works alone, so compute a score across signals rather than
thresholding one axis.

### Signals

| Signal | Source | Available today |
| --- | --- | --- |
| lines added plus deleted | `ChangeSize` in `loops::assess_scope` | yes |
| files changed | same | yes |
| sensitive path match | run config globs: auth, crypto, migrations, workflows, IaC, permissions | needs config |
| manifest or lockfile touched | diff paths | derivable |
| dependency count | `TaskSpec.dependencies` | yes |
| human-only criteria present | `Criterion.human_only` | yes |

### Sequencing

At brief time there is no diff, so the tier is provisional, computed from
planned components, dependency count and sensitive globs. It is re-evaluated at
each checkpoint once real `ChangeSize` exists. Escalation is allowed;
de-escalation is not. Self-report is removed. The coordinator keeps one door,
`escalate-tier --reason`, for judgment the signals cannot see.

Tier then selects model, effort and review depth from a profile. That profile is
also the multi-model seam, so tiering and harness portability are one change.

### Verdict is computed

The reviewer returns findings, each with `Severity` and `Category`. The
controller derives the verdict: any `Critical` -> `CHANGES_REQUIRED`; a warning
pattern (threshold per tier, same category) -> `CHANGES_REQUIRED`; incomplete
required evidence -> `BLOCKED`; otherwise `PASS`. A reviewer cannot return
`PASS` beside a critical finding, and the reviewer's own summary verdict is
recorded as opinion, not used as a gate.

## 10. Friction reduction

The run required ten separate user authorisation round trips to reach 15 merges.
The system asks for permission more often than necessary and stays quiet about
what the user actually wants to know.

### Budgets

Review budget by tier: trivial 1, lite 3, full 5. `max_stalled_checkpoints`
becomes the real runaway guard. Combined with severity gating, only critical
findings force a repair cycle; warnings are recorded and dispositioned without
consuming one.

Evidence: reviews per merged task ran 1, 1, 1, 1, 2, 2, 3, 3, 3, 3, 4, 4, 8
against a hard cap of 3, while `stalled` was 0 on every single task including
the eight-review one. The stall detector never fired. It was being pre-empted by
a cruder instrument.

### Jira traffic

`refresh` accepts a delta (`{changed: [...], removed: [...]}`) rather than the
full 51-task list. The five-minute TTL is deleted: 78 refreshes against 22
claims was pure churn driven by a wall-clock TTL shorter than a single task.

Bulk reads happen at run start and run end. Before each claim, the coordinator
confirms membership and status of the one selected issue. Status transitions are
writes and are not batched, because that would remove live visibility. There is
no long-running process to queue them: after `set-status` the coordinator
performs the transition at once through the connector, `observe-status` records
`confirmed`, `unknown` or `failed`, and `next` surfaces an `unknown` for
observation before any retry.

### Blocked work

Discovery partitions into three sets rather than one queue.

| Set | Meaning | Reported |
| --- | --- | --- |
| eligible | ready now | queue |
| blocked | waiting on another task in this run only | silent, resolves itself |
| needs-input | missing requirement, ambiguous ownership, unmapped status, human-only authority | surfaced at once, consolidated |

Only dependency blocks are silent. Everything else becomes a consolidated
question list in `questions.md`, reported at run start and whenever a new one
appears. In the observed run, 28 tasks were never reachable and 29 of 51 carried
"pre-existing terminal task is preserved", rediscovered on every pass.
`Delivery::Excluded` already exists for this and is unused.

Authorisation requests are batched. If the controller cannot proceed without
user input, it asks for everything it needs at once.

## 11. Skill rewrite

### Enforcement priority

1. If it can be made unrepresentable in the type system, do that.
2. Otherwise, if the controller can reject it, reject it, and invest in the
   error message. Error text arrives exactly when relevant at zero standing
   token cost.
3. Otherwise, if the controller can simply do it, do it: usage capture, GitHub
   observation, transition sequencing.
4. Only what survives all three goes in `SKILL.md`.

### Current token cost

| Artefact | Estimated tokens | Loaded |
| --- | --- | --- |
| `SKILL.md` | 7,000 | every run, whole |
| `references/controller.md` | 7,800 | every run, mandated by section 1 |
| `references/progress-and-feedback.md` | 2,100 | every run, same sentence |
| conditional references | 6,200 | situational |
| Mandatory floor | ~17,000 | before any work begins |

Progressive disclosure is defeated by the first section, which mandates reading
the controller contract and the progress contract up front.

### Fix

`next` returns the command name, its schema and a filled request template. The
model then never needs to read a contract to know what to call, and
`controller.md` becomes consulted rather than required. That alone removes about
10,000 tokens of mandatory context.

Delete prose that restates what the binary enforces: turn and review budgets,
open-PR cap, delivery state list, snapshot freshness, `git status` discipline,
CI deadline behaviour, and the five separate restatements of "escalation resets
no budget". This duplication has already drifted: the prose lists six delivery
states while the enum has seven.

What survives is the judgment layer: authority and precedence, Jira stage
semantics, what counts as adequate evidence, implementation simplicity, test
effectiveness, and when to stop and ask.

`status` and the final report print the resolved model profile and the
harness's declared capabilities (structured output level, reviewer isolation
level), so the coordinator states what the run had rather than the prose
asserting it.

Target: 625 lines to roughly 180, with the mandatory reference load dropping
from about 10,000 tokens to near zero.

## 12. Quality gates

| Gate | Today | Proposed |
| --- | --- | --- |
| cyclomatic complexity | 20 | 10 |
| `too-many-lines` | 120 | 80; `allow` on the single command dispatch |
| `excessive-nesting` | 5 | 3 |
| file length | none | 400 |
| `serde_json` in `domain` | none | absent from `domain/Cargo.toml`; enforced by Cargo, not a gate |
| `std::process` outside `adapters` | none | forbidden |
| `unwrap` / `expect` in `domain` | none | denied |
| clippy | `all` = warn | `all` = deny; `pedantic` = warn with a denied subset chosen at step 3 |

The custom gates protect the architecture. They make the layer boundaries
mechanically checkable rather than a convention that erodes. The existing gates
allowed a 1,651-line module and a 90-line conditional ladder through unflagged.
`pedantic` as a blanket deny brings `must_use_candidate`,
`module_name_repetitions` and `missing_errors_doc`, which add noise without
protecting anything here.

Naming is not gated mechanically. Section 17 rule 5 applies to every
identifier, not only tests.

## 13. Sequencing

Dependency order. Typing lands before the crate split so that code is moved
once, finished, rather than moved and then rewritten in place.

1. Housekeeping: move `controller/target/` out via `CARGO_TARGET_DIR`. It is
   3.8 GB and contains a vendored rustup toolchain plus maintenance snapshots.
2. Behaviour inventory (Section 17) from the existing tests. Decide keep or
   drop for each. This is the exit criterion for every later step.
3. Typed fields, the `Task` shape, `Phase`, `Hold`, `Budgets`, `Delivery` and
   `Authority`, inside the existing crate. `superseded.rs` is deleted here, not
   ported. Port each kept inventory behaviour as the module that owns it lands.
4. Events: `decide`, `apply`, `commit`; new store schema; fold-based `status`.
5. Unified command enum, command collapse (60 to 36), CLI conventions and
   the typed `AgentError` (Section 16). These land together because help text,
   dispatch and exit codes all derive from the enum.
6. Crate split and port traits: `GitHub`, `Vcs`, `Harness` with Codex and pi
   adapters. The pgrep-based launch identity (`runtime.rs:64,116`) and the
   git-based cross-run claim reservation (`store.rs:22,52`) move to `adapters`
   behind `Vcs` and a `Processes` helper; both have inventory behaviours
   (`concurrent_launch_claims_have_one_winner`,
   `cross_run_issue_claims_are_exclusive`).
7. Computed risk tiering, severity gating and the computed verdict.
8. `SKILL.md` rewrite.

Code mode (Section 15) is not in this order. It is re-decided after step 6, once
the adapters reveal whether any marshalling burden survives them.

The skill rewrite is last because the prose describes the command surface, and
documenting a surface about to be deleted wastes the effort.

Three behaviour changes do not depend on the restructure and can land first if
an improved run is wanted before the rebuild: budget defaults by tier, refresh
delta with the TTL removed, and blocked-task reporting at discovery.

## 14. Decisions and remaining questions

- Resolved: Jira credentials. None. The coordinator's connector performs every
  Jira read and transition; the controller records intent and settlement
  (Section 7).
- Resolved: the exercise mechanism is dropped; a delivery grants its own first
  pair; an unused pair may be re-purposed to the next delivery (Section 5).
- Resolved: at most one unused pair at a time, regardless of scope. The
  current suite allows a documentation pair and a source pair to coexist
  unused; that was incidental to a schema-marker test (Section 5).
- Resolved: retroactive documentation credit is dropped with no replacement
  (Section 7).
- Resolved: reviewer structured output. The `Harness` trait declares a
  three-valued capability rather than pretending harnesses are equivalent.
  `Native` covers Codex `--output-schema`, where the decoder makes
  non-conforming output impossible. `SelfValidated` covers pi, where the
  reviewer runs `controller review-schema` and `controller validate-review`
  inside its own session and iterates against a deterministic validator before
  finishing. `BestEffort` covers anything else, where the controller parses and
  re-prompts without consuming a review launch. A pi extension exposing a typed
  `submit_review` tool remains available later as a capability upgrade from
  `SelfValidated` to `Native`; it is not needed now.

```rust
enum StructuredOutput {
    Native,        // decoder-constrained
    SelfValidated, // agent validates against the controller before finishing
    BestEffort,    // controller parses; retries do not consume budget
}
```

- Resolved: pi harness shape. `pi -p --mode json --model <provider/id>
  --thinking <level> [--session <id>] [--tools ...]`. pi has no
  `--output-last-message`; the adapter takes the final message from the
  `agent_end` event. `-p` skips the trust prompt, so worktree launches pass
  `--approve` (or rely on `defaultProjectTrust: always`) or the implementer
  never sees the project's `.agents/skills`.
- Reviewer isolation on pi is `--tools` restriction rather than a sandbox mode.
  Weaker than Codex `sandbox_mode`. Recorded as an explicit capability in the
  `Harness` trait, printed by `status`, never treated as equivalent.
- Skill location. Moving to `~/.agents/skills/holy-ai-agents-batman/` makes it
  shared across pi and Codex, with nix-config managing it as live configuration
  under `config/`.

## 15. Code mode

Sources: Cloudflare `code-mode` and `code-mode-mcp`, Anthropic code execution
with MCP, and the Boundary "AI That Works" episode with Kyle from HumanLayer
(`IXe48aIw_X4`), whose implementation is public at `humanlayer/agentlayer`.

### What it is

Instead of exposing N tools, expose one code-execution tool plus an API the
model writes against. The model composes several operations in one program,
filters intermediate results locally, and returns only what it needs to reason
about. Claimed wins are fewer round trips, tool definitions loaded on demand,
and intermediate results kept out of context. Anthropic reports cases where
definitions plus results consume 50,000+ tokens before the agent reads the
request.

The simplest form is the bash tool, which pi already provides. No new runtime is
required for us to adopt the pattern.

Status: deferred pending port extraction (step 6 of section 13). The durable
findings below stand on their own and apply to single commands as much as to
batches. Whether a code-mode layer is worth building is re-decided once the
adapters exist, because most of its apparent value is compensation for their
absence.

### Why it is deferred

The controller today is the shape code mode targets: many commands, hand-built
JSON payloads, one model round trip each, and a 7,800-token reference describing
the schemas. But not every round trip is overhead. Some are the safety
mechanism, because the phase machine in section 4 exists precisely to make the
coordinator stop and judge at each transition.

The dividing line is mechanical assembly versus phase transition, which is the
same line as section 4. Applying it shows that the assembly side is mostly
removed by the adapters in sections 7 and 10, not by code mode:

| Candidate | Survives port extraction? |
| --- | --- |
| discovery, refresh | partly; Jira stays on the connector so the coordinator still supplies facts, but the delta form removes the bulk transcription |
| GitHub reconciliation reads | no; adapter fan-out replaces hand-sequenced observation |
| state queries | no; `status` gains filtering instead of dumping whole state |
| `record-proof` | marginal; one command taking an array of criterion results |
| `claim`, `publish`, `set-status`, merge | not candidates; judgment points taken one at a time through `next` |
| `checkpoint` | not a candidate; the value is the model stating what it learned |

The transcript is explicit that code mode is not a default: it is "not super in
distribution for the models yet" and suits specific problem shapes. On current
evidence our remaining shapes are too small to qualify. Revisit only if a
concrete marshalling burden survives the adapters.

### Validate before mutating

The strongest warning in the transcript is partial side effects: generated code
gets halfway through a sequence, fails, and leaves mutations applied with no
clear account of what happened. BAML's answer is compile-time checking before
execution.

Our commands write to an event log, so this applies directly. Every command
accepts `--check`, which runs `decide` (Section 8) and returns the events it
would write without committing them. A code-mode batch validates the whole
sequence first, then executes. Runtime failures such as a GitHub timeout remain
possible; those resume through `next` as today. Batches are not wrapped in a
single transaction, because the phase machine and per-command checkpointing
already provide resumption, and a long transaction would hold a write lock while
discarding real progress.

### Errors are a designed surface

The transcript calls agent-facing error design "an entire design space" and
"actually really hard", and the reference implementation redirects console
output into a captured buffer so the agent can inspect intermediate state rather
than only a final message.

Replace `format!("{e:#}")` with a structured error emitted as JSON:

```rust
struct AgentError {
    failed: Verb,        // which command
    phase: Phase,        // where the task actually is
    why: Rejection,      // typed reason, not prose
    next: NextAction,    // what to do instead
}
```

This is section 11's "invest in the rejection message" made concrete. The
message arrives exactly when relevant and costs nothing in standing context.

### Return the record of effects

`agentlayer`'s `secure_exec` tool returns `{ output, operations }`, where
`operations` is the drained list of every filesystem call the sandboxed code
made, so the agent sees its own side effects enumerated. Our equivalent: every
command returns the events it wrote, so the coordinator learns what changed
without re-reading whole state. Output is truncated, as theirs is.

### Sandboxing

We do not need one. The transcript contrasts multi-tenant backends, which
require true isolation because untrusted code sharing a memory space can reach
object prototypes and escalate, with OpenCode's plain interpreter, chosen
"because it's not a multi-tenant system. It's running on a single user box."
That is our situation.

Record two consequences honestly. First, no isolate, QuickJS, `secure-exec`
runtime or credential-injecting proxy is warranted, and none should be imported
later by analogy. Second, we therefore inherit none of their containment
properties, and must not cite them. This matches the existing posture that these
controls do not sandbox an unrestricted host agent.

### Validation at the boundary

Cloudflare's `agents` PR 962 fixed a real defect: `createCodeTool()` extracted
tool execute functions but discarded their schemas, so sandboxed code could call
tools with arbitrary payloads, bypassing the constraints tool authors declared.
Under code mode the caller is generated code rather than a careful model, which
strengthens the case for section 6: `deny_unknown_fields` and parsed newtypes on
every command, with no `serde_json::Value` reaching `domain`.

### Surface, if it is ever built

Shell invoking `controller` directly, not a generated TypeScript client. Three
layers are distinct and were previously conflated:

| Layer | Question | Choice |
| --- | --- | --- |
| glue | what the model writes | shell, the only execution surface guaranteed on both pi and Codex |
| API | what the glue calls | our Rust CLI, described in section 16 |
| runtime | where it runs | host process, no sandbox |

Cloudflare's and BAML's TypeScript sits at the API layer. Their stated reason
was distribution rather than implementation language: models have seen large
amounts of real TypeScript and few contrived tool-call examples. A conventional
CLI returning JSON is equally in distribution and needs no Node dependency or
codegen step. A harness being written in Rust is irrelevant to this choice,
since the harness language has no bearing on what the model writes as glue, and
Rust is unsuited to the glue layer anyway: it needs compilation, has no REPL,
and is rare as scripting.

The known weakness of shell is JSON assembly, where models write unreliable
`jq`. Shell is an entry point rather than the whole story, so `python3 -c` and
`node -e` remain reachable when structure manipulation is genuinely needed.

## 16. CLI conventions

The familiarity argument behind code mode applies one level down, to every
individual call. The current invocation shape is mildly out of distribution, so
the model pays a small comprehension cost on each command whether or not
batching exists. Making the CLI conventional captures most of the benefit that
motivated code mode, at far lower cost.

| Aspect | Today | Conventional |
| --- | --- | --- |
| invocation | global flag before the subcommand, request in a file | `controller claim GAIN-101 --run <dir>` |
| scalars | JSON payload on stdin or `--input` | positional arguments and flags |
| bulk input | `--input file.json` only | flags, a file, or `--input -` for stdin |
| success output | every result wrapped in `{"ok":true,"result":{...}}` | the result object directly as JSON; the caller is always an agent, so `--pretty` is the flag, not `--json` |
| errors | `format!("{e:#}")` as JSON on stderr, exit 1 | typed `AgentError` from section 15, with a distinct exit code per rejection class |
| dry run | none | `--check` on every mutating command |
| help | one hand-written string listing about 60 commands | generated per subcommand from the command enum |

Distinct exit codes matter more than they appear: they let shell branch on the
rejection class without parsing stderr, which is exactly what a coordinator
needs when recovering an interrupted run.

The `ok` envelope is removed. It duplicates the exit code, and unwrapping it is
a step the model must learn that no other CLI requires.

Help text, dispatch and the request schemas all derive from the single command
enum in section 6. That is section 11's "enforce in Rust, not prose" applied to
the tool interface, and it is what allows `references/controller.md` to shrink to
almost nothing.

## 17. Tests

The existing suite is about 4,600 lines. It is the distilled output of the
GAIN-847 run and the test names are good
(`cancellations_consume_all_three_review_attempts_across_reopen`,
`old_jira_receipt_cannot_overwrite_confirmed_state`). The bodies are not: they
build state by JSON path, assert on `Value` paths and string action names, and
some check `state.version` bumps. That couples the suite to representation, and
is the smell that made every field addition touch several files.

### Rules

1. Assert at a public boundary only. For rules, call `decide` with a state and
   facts and assert on the returned events or `Rejection` class. For the
   command surface, call the `app` handler and assert on `NextAction`, events
   written, or the rejection. Never assert on struct layout, field names, the
   serialised shape of `State`, schema versions, or any string that is not part
   of the coordinator's contract.
2. Every test traces to a behaviour the run depends on: a budget rule, an
   authority rule, a synchronisation rule, an ownership rule. A test that only
   proves wiring is dropped, not ported.
3. Adapters get contract tests against a fake process that records argv and
   replays canned output. No test spawns `gh`, `git`, `codex` or `pi`.
4. Fixtures are built through the same commands the coordinator uses, or
   through small domain constructors. A test that needs a 40-field literal is
   testing the wrong layer.
5. One rule per test, named `<subject>_<rule>` in about five words:
   `review_budget_spans_deliveries`, `pair_digest_registers_once`,
   `verification_rejects_implementer`. Today's names join several rules with
   `_and_`, for example
   `staged_last_review_and_extra_stage_cycle_recover_with_fresh_final_review_authority`,
   which pins four rules. An `_and_` in a name is the signal to split. The
   same standard applies to functions, types and variables: short, and saying
   what the thing is or does. `ordinary_exception_preserves_documentation_schema_marker`
   and `authorize_historical_slot_reuse` fail it; `one_unused_pair` and
   `Authority::fresh` pass. A name that needs a qualifier chain is describing
   a function that does too much, and the fix is usually to split the function
   rather than the name.

### Inventory

Step 2 of Section 13 produces `design/behaviours.md`: one line per rule the
existing tests protect, its owning module, and its new name. The port of each
module's lines is that module's exit criterion. Recovery rules are restated
over `deliveries` and `authorities` (Section 5); if one cannot be restated, the
types are wrong, not the rule.

Decisions already taken from reading the suite:

| Existing tests | Decision |
| --- | --- |
| `tests/improvements/followup.rs`, all but two | keep; restate over `Delivery` and `Authority` |
| `followup_preparation_does_not_grant_launches...` | rewrite: a delivery grants its first pair |
| `followup_old_schema_read_does_not_migrate...` | drop; representation |
| `tests/improvements/round2.rs` stage tests | keep; restate as narrowing plus `Verification` |
| `round2.rs` usage tests | move to adapter contract tests |
| `tests/improvements/superseded.rs` | reduce to two rules: `Replaced` is a valid `open-delivery` precondition; `paths` bound every commit |
| `tests/improvements/mod.rs` adjustment, documentation, batch, prevalidation | keep |
| `mod.rs` `usage_import_*`, `empty_tags_preserve_the_legacy_plan_wire_contract` | drop; cumulative arithmetic and wire shape both go |
| `tests/guardrails.rs` slot and provision tests | keep unchanged as worktree rules |

Expected shape after the move: most rules become fast `domain` unit tests with
no SQLite or tempdir; a small set of `app` tests cover resumption across an
interrupted `set-status`, `publish` and `run-agent`; adapter contract tests
cover argv construction for Codex and pi and usage extraction from each stream.

## 18. Catalogue

The enumerations earlier sections refer to. Payload fields are indicative;
variant sets are the contract.

### Commands

| Group | Commands |
| --- | --- |
| run | `init`, `status`, `next`, `usage-report` |
| queue | `discover`, `refresh`, `claim`, `hold`, `resume` |
| task | `brief`, `plan`, `checkpoint`, `snapshot`, `subtask-record`, `escalate-tier` |
| worktree | `bind-slot`, `cleanup` |
| proof | `record-proof`, `run-check` |
| agents | `run-agent`, `review-schema`, `validate-review`, `disposition`, `human-review`, `lesson-record` |
| delivery | `publish`, `observe-pr`, `poll-checks`, `final-verify`, `complete` |
| jira | `set-status`, `observe-status` |
| authority | `grant`, `open-delivery`, `narrow-acceptance` |
| recovery | `recover-operation` |

Thirty-six. `publish` takes `--step create|ready|merge`. `run-agent` takes a
role; model and effort come from the profile (below), so the coordinator
cannot choose a model except through a declared fallback with `--reason`.
`recover-operation` covers agent launches, provisioning helpers and GitHub
actions, since all three are `Operation`s with a process identity.

### NextAction

`next` returns `{ action, command, schema }`: the variant, the filled command
template that performs it, and that command's schema. Every variant maps to
exactly one command.

```rust
enum NextAction {
    // run
    Claim            { task: TaskId },
    RecoverUnfinished{ tasks: Vec<TaskId> },                  // open-PR cap reached
    AnswerQuestions  { questions: Vec<Question> },            // consolidated needs-input
    Report           { remaining: Vec<(TaskId, Phase, Option<Hold>)> },
    // in-flight mechanics
    MonitorLaunch    { launch: LaunchId },
    SettleOperation  { operation: OperationId },              // status unknown
    // task
    SyncStatus       { task, target: JiraStatus, transition: TransitionId },
    ObserveStatus    { task, issue: IssueKey },
    Brief            { task },
    BindSlot         { task },
    Plan             { task },
    Implement        { task, remaining_turns: u32 },
    Checkpoint       { task, launch: LaunchId },
    RecordProof      { task, missing: Vec<CriterionId> },
    Review           { task, snapshot: Snapshot },
    Disposition      { task, findings: Vec<FindingId> },
    Repair           { task, findings: Vec<FindingId> },
    ResolveGaps      { task, gaps: Vec<String> },
    Publish          { task, step: PublishStep },
    AwaitChecks      { task, pr: PrNumber, deadline: Instant },
    AwaitHumanReview { task, snapshot: Snapshot },
    FinalVerify      { task, commit: Sha },
    Complete         { task },
    Cleanup          { task, slot: SlotId },
    Hold             { task, reason: HoldReason },            // budget or stall exhausted
    OpenDelivery     { task, remaining: Vec<CriterionId> },   // merged, acceptance open
}
```

Today's 26 action strings map onto these: the three `synchronize-jira*`
strings become `SyncStatus`; `implement-or-validate` and
`validate-existing-work-or-hold` become `Implement` or `Hold` depending on
budget; `record-needs-human` becomes `Hold`; `end-stage` disappears;
`configure-loop` and `refresh-queue` disappear because policy is fixed at
`init` and refresh is coordinator-driven.

### Events

One event per fact. A command may write several. `apply` is total.

```rust
enum Event {
    // run
    RunInitialised   { config: RunConfig, profile: Profile },
    QueueDiscovered  { tasks: Vec<TaskSpec>, frozen: Instant },
    QueueRefreshed   { changed: Vec<TaskSpec>, removed: Vec<TaskId> },
    QuestionRaised   { task: Option<TaskId>, question: Question },
    // task
    Claimed          { task },
    Held             { task, reason: HoldReason },
    Resumed          { task },
    Briefed          { task, criteria: Vec<Criterion>, requirements: Digest, tier: Tier, provisional: bool },
    TierRaised       { task, to: Tier, by: TierSource },       // Signals | Coordinator { reason }
    SlotBound        { task, binding: SlotBinding },
    SlotReleased     { task, slot: SlotId },
    Planned          { task, plan: Plan },
    Snapshotted      { task, delivery: DeliveryId, snapshot: Snapshot, size: ChangeSize },
    Checkpointed     { task, checkpoint: Checkpoint, stalled: u32 },
    SubtaskRecorded  { task, subtask: Subtask },
    Excluded         { task, reason: ExclusionReason },
    // launches
    LaunchStarted    { launch: Launch },
    LaunchEnded      { launch: LaunchId, result: LaunchResult, usage: Option<Tokens> },
    // proof and review
    ProofRecorded    { task, delivery, entries: Vec<ProofEntry> },
    ProofInvalidated { task, delivery, cause: InvalidationCause },
    ReviewSettled    { task, delivery, launch: LaunchId, findings: Vec<Finding>, verdict: Verdict },
    Dispositioned    { task, finding: FindingId, disposition: Disposition },
    HumanReviewed    { task, snapshot: Snapshot, receipt: Digest },
    LessonRecorded   { lesson: Lesson },
    // delivery
    DeliveryOpened   { task, delivery: Delivery },
    AcceptanceNarrowed { task, delivery, criteria: Vec<CriterionId>, authority: AuthorityId },
    OperationStarted { operation: Operation },
    OperationSettled { operation: OperationId, status: OpStatus, observation: Observation },
    PrObserved       { task, delivery, pr: PullRequest },
    DeliveryClosed   { task, delivery, outcome: Outcome },
    Verified         { task, receipt: Receipt },
    Completed        { task },
    // jira
    StatusIntended   { task, issue: IssueKey, target: JiraStatus, transition: TransitionId },
    StatusObserved   { task, issue: IssueKey, read: JiraStatus, outcome: SyncOutcome },
    // authority
    AuthorityRegistered { task, authority: Authority },
    GrantUsed        { task, authority: AuthorityId, by: UseId },
}
```

Thirty-seven. `Observation` is a typed per-`GitHubAction` payload, never
`Value`.

### Budgets and caps

`Budgets` stores counts only. Caps are computed at each check from the current
tier, so a tier raised mid-task raises the cap and never resets a count.

```rust
struct Budgets {
    implementation_turns: u32,
    stalled_checkpoints: u32,
    reviews: u32,
    repairs: u32,
    ci_repairs: u32,
    escalations: u32,
}

fn remaining(kind: BudgetKind, budgets: &Budgets, tier: Tier, authorities: &[Authority], caps: &Caps) -> u32 {
    caps.for_tier(tier)[kind] + granted(kind, authorities) - budgets[kind]
}
```

Launches under a path-scoped pair carry `counted: false` and are excluded from
the counts. Default caps, overridable per tier in run config:

| Tier | reviews | repairs | turns | stalled | CI repairs | escalations |
| --- | --- | --- | --- | --- | --- | --- |
| trivial | 1 | 1 | 4 | 2 | 1 | 0 |
| lite | 3 | 2 | 8 | 2 | 1 | 1 |
| full | 5 | 4 | 12 | 2 | 1 | 1 |

### Tier signals and thresholds

```rust
fn tier(signals: &Signals, config: &RiskConfig) -> Tier {
    let forced = signals.sensitive_paths > 0
        || signals.manifest_or_lockfile
        || signals.human_only_criteria > 0
        || signals.dependencies > 2
        || signals.lines > 100
        || signals.files > 20;
    if forced { Tier::Full }
    else if signals.lines <= 10 && signals.files <= 5 { Tier::Trivial }
    else { Tier::Lite }
}
```

At brief time `lines` is unknown and `files` is the planned component count;
the result is provisional. Each `Snapshotted` recomputes from real
`ChangeSize`; the tier moves up or stays. `escalate-tier --reason` is the
coordinator's door.

### Verdict

Suggestions never affect the verdict. Non-empty evidence gaps or missing
required proof give `BLOCKED` regardless of findings.

| Tier | any Critical | Warnings, same category | Warnings, total |
| --- | --- | --- | --- |
| trivial | `CHANGES_REQUIRED` | 3 | 5 |
| lite | `CHANGES_REQUIRED` | 3 | 5 |
| full | `CHANGES_REQUIRED` | 2 | 4 |

Reaching a warning threshold gives `CHANGES_REQUIRED`; below it, warnings are
recorded, dispositioned and do not consume a repair cycle.

### Profile

`profiles/<name>.toml` beside the skill, selected by `init --profile <name>`,
validated at init, recorded in `RunInitialised`. This is the multi-model seam.

```toml
[harness]
kind = "pi"                                     # "codex" | "pi"

[models]
"openai/gpt-5.6-terra" = { rank = 1 }
"openai/gpt-5.6-sol"   = { rank = 2, fallback_for = ["openai/gpt-5.6-terra"] }
"openai/gpt-6-astra"   = { rank = 3 }

[coordinator]
model = "openai/gpt-5.6-sol"
effort = "medium"                               # reported on mismatch, never enforced

[tier.trivial]
implementer = { model = "openai/gpt-5.6-terra", effort = "medium" }
reviewer    = { model = "openai/gpt-5.6-sol",   effort = "medium" }

[tier.lite]
implementer = { model = "openai/gpt-5.6-sol", effort = "medium" }
reviewer    = { model = "openai/gpt-5.6-sol", effort = "medium" }

[tier.full]
implementer = { model = "openai/gpt-5.6-sol", effort = "high" }
reviewer    = { model = "openai/gpt-5.6-sol", effort = "high" }

[escalation]
reviewer = { model = "openai/gpt-6-astra", effort = "high" }
```

Invariants checked at init, replacing the eight string comparisons in
`agents.rs`: every referenced model is declared; `reviewer.rank >=
implementer.rank` in every tier; `escalation.reviewer.rank >
tier.full.reviewer.rank`; a mid-task model transfer goes only to a declared
`fallback_for` target, with a reason. `ModelId` is `provider/id`; the Codex
adapter passes `id`, the pi adapter passes the whole string. An `anthropic`
profile is the same file with `anthropic/claude-*` IDs and `kind = "pi"`.

The harness adapter reports `Capabilities { structured_output:
StructuredOutput, isolation: Isolation }` where `Isolation` is `Sandbox`,
`ToolRestriction` or `None`; `status` prints both.

### Run config

`init --config run.toml`. Immutable after init; replaces `configure-loop` and
`LoopPolicy`.

```toml
repo        = "/home/michael/businesscraft"
github_repo = "businesscraft/businesscraft"
epic        = "GAIN-847"
profile     = "openai"
review_mode = "autonomous"                      # "autonomous" | "human-review"
max_open_prs = 2
feedback_file = "docs/agent-lessons.json"       # optional committed lesson library

[jira.statuses]
todo = "1"
progress = "2"
review = "3"
done = "4"

[risk]
sensitive = ["auth/**", "crypto/**", "**/migrations/**", ".github/workflows/**", "infra/**", "**/permissions/**"]

[caps.full]                                     # optional per-tier overrides
reviews = 5
```

### Task additions

Two fields Section 4 omitted:

```rust
struct Task {
    // ... Section 4 ...
    tier: TierState,                       // current, provisional, history of TierRaised
    subtasks: BTreeMap<IssueKey, Subtask>, // owned steps within this task's deliveries
}

struct Subtask {
    criteria: Vec<CriterionId>,
    owned: bool,
    was_terminal: bool,
    status: JiraStatus,
    sync: Sync,
}
```

A subtask transitions through the same `set-status` and `observe-status` with
its own issue key, and `subtask_done_requires_record` is the rule that it
must exist here first.
