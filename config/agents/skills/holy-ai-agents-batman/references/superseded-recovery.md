# Recover a superseded PR under an existing exercise approval

Use this path when this run's current PR was closed without merging, a separately
accepted replacement merged to main, and the user already approved one exercise
with reviewed preparation and reviewed removal of its temporary controls. The
current preparation implementation/review pair must be recorded and wholly unused.
This is a narrow transfer of existing authority. It does not authorize another
operational attempt or expand deployment, environment, or infrastructure scope.

## Recover preparation

Reconcile outstanding operations and launches first. Hold the unfinished task,
then refresh the complete frozen Jira queue. It must remain unresolved, owned,
in the epic, and free of recorded blockers. Use `recover-superseded --input <file>`:

```json
{
  "key": "GAIN-123",
  "source": "Actual user approval covering preparation, one exercise and reviewed cleanup",
  "artifact": "/absolute/run/renewed-exercise-approval.md",
  "requirements": "<unchanged full task requirements hash>",
  "scope": "Prepare temporary controls for the approved exercise on the accepted replacement",
  "replacement": {"pr": 456, "head": "<full head SHA>", "merge": "<full merge SHA>"},
  "cleanup_scope": "Remove the exercise controls and preserve normal release behavior",
  "cleanup_paths": [".github/workflows/example.yaml"],
  "cleanup_criteria": [
    {"id":"CLEAN1","text":"Temporary controls are removed and normal release behavior is verified","human_only":false}
  ]
}
```

Check the actual receipt covers both phases before issuing the command. Choose
exact repository-relative cleanup files and criteria from that approved scope.
The controller freezes them now; criteria IDs must differ from full task IDs.
The artifact must match the unused recorded allowance and its unchanged snapshot.
Partially consumed pairs, changed receipts, unknown operations, active stages,
reopened original PRs and unmerged replacements are rejected.

The command reads both PRs through GitHub and verifies the replacement merge on
origin/main. It archives the original PR as closed and unmerged, records the
replacement separately, preserves counters and history, and transfers the unused
pair to a new delivery. It grants zero new preparation launches. Do not call
`authorize-budget-adjustment` again, reopen the old PR, consume dummy launches,
edit SQLite, or restart the run to bypass these checks.

Resume, reserve a different worker, and provision from the prepared main base.
Preserve archived workers and their local material. Bind promptly because main
movement before binding still fails closed. Use `begin-stage` with the actual
exercise approval for independently reviewed preparation, then `loop-plan` with
the original task baseline from the first delivery archive. Implement, validate,
review and merge through ordinary controller operations. End the merged stage.

Run only the operational exercise already authorized by the user. These controller
commands neither dispatch it nor prove its outcome. Qualify the prerequisite
environment, perform the approved failure injection at the specified point, and
record observed recovery. Do not repeat a failed attempt without authority.

## Remove controls after the attempt

Create an absolute JSON outcome artifact with `terminal: true`, a nonempty
`dispatch` string identifying the actual run or explaining why none was started,
and a nonempty `recovery_observation` string describing what happened with links
to proof. A terminal failure is a valid cleanup boundary. Never fabricate success
or describe a still-running exercise as terminal.

Hold and refresh again after the preparation merge and ended stage, then call
`prepare-approved-cleanup --input <file>`:

```json
{
  "key": "GAIN-123",
  "preparation": 789,
  "outcome_artifact": "/absolute/run/exercise-outcome.json"
}
```

Use the preparation delivery ID returned by `recover-superseded`. The command
re-verifies its merged PR, freezes the terminal outcome, and makes the distinct
cleanup pair available once under the original approval. Preparation's review
launch must have been consumed. Counters and its consumed receipt remain intact.
A failed or cancelled cleanup launch consumes its allowance normally.

Resume in another reserved worker. All cleanup commits, including changes later
reverted, must stay within the frozen file list. Snapshot and acceptance checks
enforce this boundary. Keep the approval and outcome artifacts unchanged.

If full task proof passed, validate cleanup and review against all full criteria.
If the exercise failed or proof remains missing, use `begin-stage` with the exact
recorded `cleanup_scope`, `cleanup_criteria` and original approval artifact. The
usual stage request still needs truthful operational-proof and cleanup-plan fields.
Only this paired cleanup stage can reuse preparation's receipt. Review and merge
removal, then `end-stage`. Full acceptance remains unresolved and `final-verify`
cannot pass without fresh evidence for every full criterion. Keep Jira In Progress
and record the remaining blocker; cleanup does not authorize a second exercise.

## History and compatibility

Recovery writes schema9. Older runs load unchanged; older binaries must not be
used after recovery. Rebuild or use the installed updated executable before
resuming. `gh-observe` with an archived original PR verifies that it remains closed
and unmerged; with its replacement PR it verifies that external merge on main.
Neither observation changes current delivery. The ledger distinguishes external
replacements from this run's own PRs. Ordinary `prepare-followup` keeps its existing
fresh-receipt rules; this exception requires the specific unused exercise pair.
