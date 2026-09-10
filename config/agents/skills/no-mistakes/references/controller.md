# Controller reference

Use this reference when initializing a run, diagnosing a rejection, or recovering
without a usable `next` response. Normal delivery follows the filled template and
schema returned by `next`.

## Build and state

```sh
EPIC_SKILL=/absolute/path/to/no-mistakes
. "$EPIC_SKILL/controller/env.sh"
cargo build --release --locked --manifest-path "$EPIC_SKILL/controller/Cargo.toml"
CONTROLLER="$CARGO_TARGET_DIR/release/controller"
"$CONTROLLER" init --run "$EPIC_RUN" --config run.toml --profile "$EPIC_SKILL/profiles/pi.toml"
```

`EPIC_RUN` is `<repository>/.no-mistakes/<epic-key>`: absolute, Git-ignored,
outside worker checkouts, and reused for every session on that epic. SQLite
events are authoritative. The projection is checked against a complete event
fold whenever state loads. Keep the database and its WAL files together. Never
edit them or create another run to reset a limit.

Every command derives from one `Command` enum. `controller <command> --help`
prints that command's fields, each with its type and whether it is required.
A scalar field (string, integer, boolean) is a `--field value` flag; the field
named `task` may instead be given as the first positional. A field the schema
does not declare as scalar must come through `--input <file|->` (`-` reads
stdin); where argv and `--input` both set a field, argv wins. Serde rejects
unknown fields; scalar commands reject unknown options, duplicate options, and
extra positionals.

Commands return JSON directly. `--pretty` formats output. A command that wrote
events also returns `next`, the same annotated action a standalone `next`
would, so the coordinator performs the following step without another call.
Every mutating command except `init` accepts `--check`, which returns its
proposed events without writing and no `next`. A rejection is a typed JSON object on stderr: `class` and
`reason` name it, `message` is a human-readable rendering, `phase` is the
affected task's actual phase, and `next` is the same annotated action `next`
returns. The exit code is class-specific.

`next` returns `action` (the typed decision), `command` (the invocable command
line), `template` (the filled request), and `schema` (the real JSON schema,
taken from the `Command` enum, for the command that performs it). A rejection
carries the same `next` when one applies.

## Configuration

The immutable TOML run config names the repository, GitHub repository, epic,
review mode, Jira status IDs, open-PR cap, optional feedback file, sensitive path
globs, optional tier cap overrides, and `follow_up_deliveries`, the number of
code deliveries per task that may reopen after a hold without a receipt
(default 0). Use actual status IDs and an absolute repository path.

The profile selects `codex`, `pi`, or `claude`. It declares every model, rank,
fallback, coordinator assignment, tier assignment, and escalation reviewer.
Initialization rejects undeclared models, a reviewer ranked below its
implementer, and an escalation reviewer not ranked above the full-tier
reviewer. Bundled profiles are under `profiles/`.

`status` reports the resolved profile and harness capabilities. Codex has native
structured review output and sandbox isolation. Pi self-validates review
output; its reviewer keeps `bash` for `git diff`, so its isolation capability
is none, whatever tool list the launch argv carries. Claude has native
structured review output: the controller passes the review schema inline with
`--json-schema` and reads the report from the result. Its isolation is tool
restriction: the reviewer runs under permission mode `dontAsk` with tools
limited to Read, Grep, Glob, and Bash. Bash accepts Claude's read-only command
set plus `git diff`, `git log`, and `git show`; any other command is denied and
recorded in the result. Workers load only user-level settings, so a target
repository's hooks cannot alter a governed launch.

## Command catalogue

The public surface has exactly 37 commands.

| Group | Commands |
| --- | --- |
| Run | `init`, `status`, `next`, `usage-report` |
| Queue | `discover`, `refresh`, `claim`, `hold`, `resume` |
| Task | `brief`, `plan`, `checkpoint`, `snapshot`, `subtask-record`, `escalate-tier` |
| Worktree | `bind-slot`, `cleanup` |
| Proof | `record-proof`, `run-check` |
| Agents | `run-agent`, `review-schema`, `validate-review`, `disposition`, `human-review`, `lesson-record` |
| Delivery | `publish`, `observe-pr`, `poll-checks`, `final-verify`, `complete` |
| Quality | `judge` |
| Jira | `set-status`, `observe-status` |
| Authority | `grant`, `open-delivery`, `narrow-acceptance` |
| Recovery | `recover-operation` |

`publish` uses `create`, `ready`, or `merge`. `run-agent` uses `implementer`,
`reviewer`, or `escalation`; the profile supplies model and effort.
`checkpoint` snapshots the turn it marks; `snapshot` alone is for an
integration commit with no implementer turn. `open-delivery` without a
receipt opens a briefed task's first delivery as verification of a commit on
main, for work that is already delivered. `judge`
runs the LLM-as-judge for one task in the background of a run; `complete` and
`usage-report` start it, so the coordinator never calls it directly. See
[quality.md](quality.md). `poll-checks
--wait` polls every 30 seconds until no required check is pending or its
per-head deadline passes, then records the hold; without `--wait` it observes
once. `bind-slot` reserves and creates a fresh worktree, or performs explicitly
authorized historical reuse. `record-proof` accepts an atomic array of
criterion results.

## Prefactor tracing

The controller mirrors its event ledger to Prefactor through the `prefactor`
CLI when `PREFACTOR_API_TOKEN`, `PREFACTOR_AGENT_ID`, and
`PREFACTOR_AGENT_IDENTIFIER` are set. `env.sh` exports them from
`$EPIC_SKILL/.env`, which stays Git-ignored; `.env.example` lists the names.
It also puts the CLI's default install location, `~/.prefactor/bin`, on `PATH`.
Instance registration and the quality payload go to the HTTP API at
`PREFACTOR_API_URL` (default `https://app.prefactorai.com`) through `curl`,
because the CLI cannot declare or record quality schemas.

One run is one agent instance, opened by the first committed event and
closed by `usage-report`, or by the last background judge when judges are
still running at that point; a later command opens a fresh one. Every committed
event is one span named after its kind and carrying the event body, its
sequence, and its actor, so briefs, plans, checkpoints, proofs, review
findings and verdicts, dispositions, PR observations, Jira transitions,
budgets, and holds all appear in order. A launch, a GitHub operation, and a
Jira status change each open a span and finish it with the ending event as
the result. The launch span adds the prompt text; the ending carries the
output and usage. Prompt and output are sent as Prefactor sensitive values,
redacted by default, and any string is cut at 100,000 characters.
`PREFACTOR_CAPTURE_INPUTS=false` omits the prompt text and
`PREFACTOR_CAPTURE_OUTPUTS=false` the output.

A `--check` writes no events and sends nothing. Tracing never fails or blocks
a command: unsent records wait in the run's SQLite file, outside the event
log, and replay in order on the next commit. The token is removed from
worker environments. Inspect a run with `prefactor agent_instances list
--agent_id` and `prefactor agent_spans list --agent_instance_id <id>
--start_time <iso> --end_time <iso>`.

## Jira bridge

The controller has no Jira credential. Before changing status, read the issue and
available transitions through the approved connector. Send the fresh status,
target, and transition list to `set-status`. Perform only the selected transition.
Read the issue again and settle it with `observe-status`.

The intent event precedes the connector mutation. An interruption therefore
leaves synchronization unknown. `next` asks for observation before another
intent. A target read confirms success without a retry. If the source status is
unchanged, two retries are allowed, each after a fresh read; the third failure
holds the task. Older reads cannot overwrite a newer confirmed receipt. Record
subtasks before changing their status.

## Process and external operations

`run-check` and harness adapters start commands behind a gate, record the PID,
process-group ID, Linux start ticks, and timeout, then release them. Timeout
terminates the owned process group and records failure. Recovery compares every
identity field before inspection or termination. A reused PID does not match.

`recover-operation` never trusts a `terminate` claim. With no flag it reports a
still-running owned process. With `--terminate` it may stop only the matching
process group. A stopped process can settle failed, and an owned harness launch
can settle failed or cancelled without refunding recorded spend. GitHub
operations have no recoverable local process receipt in controller state, so
observe the PR instead of treating process termination as proof of remote
failure. The controller refuses to invent a terminal result for an unowned
launch.

## Delivery history and authority

A task has one phase, an optional orthogonal hold, Jira synchronization, monotonic
budgets, authorities, and ordered deliveries. A merge is a fact, not acceptance.
Only `Verified` means every full criterion passed.

A task's first delivery needs no authority, whether it is the code delivery
`bind-slot` opens or a verification delivery of a commit already on main that
`open-delivery` opens without a receipt. A later delivery opened without a
receipt is a standing follow-up: it needs the same needs-human hold, fresh
Jira read, unchanged criteria, closed prior delivery, and intact history as a
granted one, counts against `follow_up_deliveries` when it is a code delivery,
and is recorded with no authority. Every other later delivery carries one
`Delivery` grant. A `Verification` delivery has no worktree and rejects an
implementer. `narrow-acceptance` changes the current open code delivery's
criteria under a `Narrowing` grant. Its merge stays in history and leaves full
acceptance open.

The coordinator supplies a receipt's source, absolute artifact, artifact digest,
criteria digest, and grant; the controller assigns its id, records the time, and
tracks its use. The artifact must remain readable and unchanged. A digest
registers once. A grant is consumed once. One unused pair may exist. A wholly
unused pair may become the next delivery grant; a partially spent pair cannot.
Path-scoped grants apply to every commit in the range, including reverted work.

A later delivery requires a needs-human hold, fresh unresolved Jira membership
and ownership, unchanged criteria, a closed prior delivery, and intact history.
Every prior merge must remain on main. Every replacement must retain its recorded
PR, head, and merge identity.

## Worktrees

A fresh bind accepts only a slot whose directory is missing; slot names are
free, so choose an unused one rather than clearing a directory this run did
not create. Existing unknown directories, symlinks, foreign repositories, and
dirty checkouts fail before mutation. State also rejects any slot with an
unfinished owner.

Historical reuse needs a `SlotReuse` grant naming the exact completed task and
slot. The current delivery must have no work, launches, operations, proof, or
review. The checkout must be clean and owned. The adapter switches it to the new
branch at current `origin/main`, resets it, and cleans untracked files. The grant
is consumed by the bind. No authority permits deleting an unfinished worker.

## Agent and review settlement

The controller records launch attempts and captures usage from Codex JSON, Pi
JSON, or Claude JSON result events. Missing usage is reported as unknown, never
zero. Cached input stays null if any contributing launch omitted it. No token
report infers billing cost.

Review starts only after current proof passes. A new snapshot or new proof
moves the settled review to the delivery's prior review, and the next reviewer
launch's prompt ends with that review's head and findings. The reviewer
reports only findings, evidence gaps, and its own opinion; the controller
supplies the launch, session, snapshot, computed verdict, and dispositions
itself. Use
`review-schema` for the exact schema and `validate-review` for deterministic
validation against a task's tier. The controller recomputes the verdict from
finding severity and tier. Critical findings and configured warning patterns
require changes. Evidence gaps block. Every finding needs a disposition before
merge.

Escalation requires a completed ordinary reviewer launch. Starting it spends one
review and one escalation unit. Rejected preconditions spend neither.

## Publication and completion

Create a draft PR from a recorded snapshot. Ready and merge operations require
the same head. Merge also requires independent PASS, all findings dispositioned,
green required checks, and a current human receipt when human-review mode is
selected. The GitHub adapter uses exact-head protection and does not bypass
branch rules.

`observe-pr` records open, closed, merged, or externally replaced outcomes. A
closed historical delivery is immutable. `poll-checks --wait` keeps one
deadline per PR head, set to the first observed pending check plus 1800
seconds; holding and resuming do not restart it.

`final-verify` checks the recorded PR head against the reviewed snapshot, the
recorded merge identity against the supplied commit, and the merge commit on
main. This supports squash merges without pretending the PR head equals the
merge commit. `complete` also requires confirmed Jira Done for the parent and all
recorded subtasks. Cleanup requires completed delivery and acts only on its owned
slot; a task verified without a slot has nothing to clean and releases the
queue at `complete`.
