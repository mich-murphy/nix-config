# Step 3a: typed identifiers and `Authority`

First part of `design/redesign.md` Section 13 step 3. Read `redesign.md`
Sections 4, 5, 6, 12, 17 and 18, and `design/behaviours.md` in full before
starting. Work inside the existing crate; the crate split is a later step. Do
not begin 3b or later.

Baseline: `cae6c7c` on `main`. Gate command: `controller/quality.sh`. Tests:
source `controller/env.sh`, then `cargo test --locked` in `controller/`.

## Goal

Two things land, each with the `behaviours.md` lines that own them ported as
tests at a public boundary.

### 1. Parsed identifiers

New module `controller/src/ids.rs` with newtypes `TaskId`, `SlotId`, `Sha`
(40 lowercase hex) and `Digest` (64 lowercase hex). Each implements `FromStr`,
`Display`, `AsRef<str>`, ordering and hashing, and deserialises with
`#[serde(try_from = "String")]` so an invalid value is rejected at the edge.
Serialise transparently; the JSON wire shape does not change.

Use them where the value is parsed today: `valid_key()` and `valid_slot()`
call sites, `Snapshot.base` and `Snapshot.head`, `Task.merged_commit`, every
receipt `digest` field, and `Input` variants carrying keys or slots. Delete
`valid_key()` and `valid_slot()` once nothing calls them. Map keys stay
`String` where SQLite or JSON object keys force it; say so in the report.

`ModelId` and `Effort` are deferred to the profile step (step 7); leave the
model strings alone.

### 2. `Authority` and `Grant`

New module `controller/src/authority.rs` carrying the types in Section 5 and
the seven rules listed there, as pure functions over `&[Authority]`. Add
`Task.authorities: Vec<Authority>`. `Grant::Pair { paths: Option<Vec<..>> }`
replaces both `ExtraCycle` and `DocumentationCycle`; `previous_extra_cycles`,
`extra_cycle`, `documentation_cycles` and `documentation_reconciliation` are
removed from `Task`. Every reader of those fields (`agents.rs`, `budget.rs`,
`documentation.rs`, `followup.rs`, `stages.rs`, `superseded.rs`, `loops.rs`,
`engine.rs`) is redirected to `authorities`.

Replay is one rule: a digest registers once across `Task.authorities`. The
receipts that still live in `followup.rs`, `stages.rs` and `superseded.rs`
register their digest in `authorities` when created, so the existing
per-module replay chains (`reject_replay`, `unused`, `stage_receipt_reuse`
digest checks) collapse to that one lookup. Their other fields move in 3d;
do not restructure those modules beyond the digest and pair redirection.

`Grant::Delivery`, `Grant::Narrowing` and `Grant::SlotReuse` are declared now
so the enum is complete, with rules unit-tested, but nothing constructs them
until 3b and 3d.

Retroactive documentation credit is dropped (`behaviours.md`, decisions
taken). Delete `controller/src/documentation/reconciliation.rs`, the
`reconcile-documentation-extra` command, `AdjustmentMode::ReconcileDocumentationExtra`,
and `tests/documentation_budget/reconciliation.rs`. Remove the legacy
`authorize-extra-cycle` and `authorize-documentation-cycle` commands;
`authorize-budget-adjustment` is the single entry and now writes a
`Grant::Pair` directly instead of re-dispatching a legacy input through a
cloned `State`. Keep that command name; renames are step 5.

## Tests

Port these `behaviours.md` lines, one test each, named as listed there:

- `domain::authority`: `digest_registers_once`, `changed_receipt_blocks_grant`,
  `receipt_must_be_absolute`, `grant_requires_current_criteria`,
  `partial_pair_stays_put`, `scope_requires_markdown_paths`,
  `grant_requires_idle`, `one_unused_pair`.
  `unused_pair_repurposes` is declared `#[ignore]` with a note that 3d wires
  it, since nothing opens a delivery yet.
- `domain::budget`, pair rules only: `pair_grants_one_of_each`,
  `failed_pair_is_spent`, `pair_requires_hold`, `pair_is_task_scoped`,
  `scoped_pair_is_uncounted`, `scoped_pair_keeps_exhaustion`,
  `scoped_pair_needs_acceptance`.

The seven pure rules in `authority.rs` are unit tests in that module with no
store or tempdir. The budget lines drive the CLI as the existing fixtures do.
Remove the existing tests these replace (`additional_adjustments_archive_history...`,
`human_exception_allows_one_cycle...`, `exception_does_not_authorize_other_tasks...`,
`ordinary_exception_preserves_documentation_schema_marker`, and the five
`documentation_budget/mod.rs` tests). Delete every assertion on
`state.version`, `previous_extra_cycles`, `documentation_cycles` or other
representation you meet while doing so. All other tests must still pass
unchanged in meaning; adapting their fixtures to the removed legacy commands
is expected, changing what they assert is not.

## Constraints

- Section 17 rules 1 to 5. No test asserts on struct layout, serialised
  `State`, or schema versions. Names are short and state one rule; an `_and_`
  is a split signal. This applies to functions and types too.
- No new `unwrap` or `expect` in `ids.rs` or `authority.rs`.
- No `/home/michael` in tracked files. No changes to `design/`.
- `controller/quality.sh` and `cargo test --locked` pass. If the existing
  gate thresholds (complexity 20, 120 lines) block a necessary change, say
  so rather than restructuring unrelated code; tightening them is step 3d.
- One commit on `main`.

## Report

Commit hash; the list of `String` fields you could not convert and why;
the ported test names with pass status; deleted files and commands;
`controller/quality.sh` and `cargo test --locked` outcomes with test counts;
anything left undone.
