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

`EPIC_RUN` must be absolute, Git-ignored, and outside worker checkouts. SQLite
events are authoritative. The projection is checked against a complete event
fold whenever state loads. Keep the database and its WAL files together. Never
edit them or create another run to reset a limit.

Commands return JSON directly. Errors return a typed JSON object on stderr and a
class-specific exit code. `--pretty` formats output. Every mutating command except
`init` accepts `--check`, which returns its proposed events without writing.
Complex requests use `--input file.json` or `--input -`. Serde rejects unknown
fields. Scalar commands reject unknown options, duplicate options, and extra
positionals.

## Configuration

The immutable TOML run config names the repository, GitHub repository, epic,
review mode, Jira status IDs, open-PR cap, optional feedback file, sensitive path
globs, and optional tier cap overrides. Use actual status IDs and an absolute
repository path.

The profile selects `codex` or `pi`. It declares every model, rank, fallback,
coordinator assignment, tier assignment, and escalation reviewer. Initialization
rejects undeclared models, a reviewer ranked below its implementer, and an
escalation reviewer not ranked above the full-tier reviewer. Bundled profiles are
under `profiles/`.

`status` reports the resolved profile and harness capabilities. Codex has native
structured review output and sandbox isolation. Pi self-validates review output
and restricts reviewer tools.

## Command catalogue

The public surface has exactly 36 commands.

| Group | Commands |
| --- | --- |
| Run | `init`, `status`, `next`, `usage-report` |
| Queue | `discover`, `refresh`, `claim`, `hold`, `resume` |
| Task | `brief`, `plan`, `checkpoint`, `snapshot`, `subtask-record`, `escalate-tier` |
| Worktree | `bind-slot`, `cleanup` |
| Proof | `record-proof`, `run-check` |
| Agents | `run-agent`, `review-schema`, `validate-review`, `disposition`, `human-review`, `lesson-record` |
| Delivery | `publish`, `observe-pr`, `poll-checks`, `final-verify`, `complete` |
| Jira | `set-status`, `observe-status` |
| Authority | `grant`, `open-delivery`, `narrow-acceptance` |
| Recovery | `recover-operation` |

`publish` uses `create`, `ready`, or `merge`. `run-agent` uses `implementer`,
`reviewer`, or `escalation`; the profile supplies model and effort. `poll-checks
--wait` performs bounded waiting. `bind-slot` reserves and creates a fresh
worktree, or performs explicitly authorized historical reuse. `record-proof`
accepts an atomic array of criterion results.

## Jira bridge

The controller has no Jira credential. Before changing status, read the issue and
available transitions through the approved connector. Send the fresh status,
target, and transition list to `set-status`. Perform only the selected transition.
Read the issue again and settle it with `observe-status`.

The intent event precedes the connector mutation. An interruption therefore
leaves synchronization unknown. `next` asks for observation before another
intent. A target read confirms success without a retry. If the source status is
unchanged, one retry is allowed. Older reads cannot overwrite a newer confirmed
receipt. Record subtasks before changing their status.

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

The first code delivery needs no authority. Every later delivery carries one
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

A fresh bind accepts only a missing numbered slot. Existing unknown directories,
symlinks, foreign repositories, and dirty checkouts fail before mutation. State
also rejects any slot with an unfinished owner.

Historical reuse needs a `SlotReuse` grant naming the exact completed task and
slot. The current delivery must have no work, launches, operations, proof, or
review. The checkout must be clean and owned. The adapter switches it to the new
branch at current `origin/main`, resets it, and cleans untracked files. The grant
is consumed by the bind. No authority permits deleting an unfinished worker.

## Agent and review settlement

The controller records launch attempts and captures usage from Codex JSON or Pi
JSON events. Missing usage is reported as unknown, never zero. Cached input stays
null if any contributing launch omitted it. No token report infers billing cost.

Review starts only after current proof passes. The reviewer reports only
findings, evidence gaps, and its own opinion; the controller supplies the
launch, session, snapshot, computed verdict, and dispositions itself. Use
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
closed historical delivery is immutable. `poll-checks --wait` keeps one deadline
per PR head. Holding and resuming do not restart it.

`final-verify` checks the recorded PR head against the reviewed snapshot, the
recorded merge identity against the supplied commit, and the merge commit on
main. This supports squash merges without pretending the PR head equals the
merge commit. `complete` also requires confirmed Jira Done for the parent and all
recorded subtasks. Cleanup requires completed delivery and acts only on its owned
slot.
