# Recover unfinished acceptance after a merge

`prepare-followup` separates the current delivery attempt from cumulative task
completion. A verified PR merge does not settle missing operational proof,
cleanup, or any other full task criterion.

Follow-up preparation preserves full task acceptance and grants no stage merge,
deployment or additional review allowance. For explicitly authorized sequencing,
use the separate [staged acceptance contract](controller.md#explicit-staged-acceptance)
after binding the follow-up and recording its actual cycle allowance. End any
active stage before preparing another delivery. Do not weaken `brief` or
manufacture full acceptance to permit an intermediate merge.

## Prepare the delivery

Hold the task first. The controller must have no active task, live launch,
prepared operation or uncertain operation. Refresh the complete frozen queue
from actual current Jira reads immediately before preparation. The selected
task must remain unresolved, owned, in the epic and free of recorded blockers.
Pre-existing terminal tasks and tasks with final acceptance are rejected.

```json
{
  "key": "GAIN-101",
  "source": "Actual current-user message authorizing one bounded correction and review",
  "artifact": "/absolute/ignored/run/GAIN-101/user-approval.md",
  "scope": "Correct the remaining diagnostic; full task acceptance is unchanged",
  "requirements": "<unchanged full task requirements hash from status>"
}
```

Call `prepare-followup` with this request. It verifies the saved snapshot,
current PR number, branch, head and merge through GitHub, fetches origin/main
and verifies the merge is present there. The source head must still be a local
Git commit. No worker checkout is needed for this historical verification.
`fresh()` remains a strict check of an existing, clean, owned current checkout.

The actual approval artifact is hashed. Replayed preparation and previously
consumed budget receipts are rejected. The coordinator remains responsible for
checking that the receipt is actual user authority for the named scope. A source
string cannot authenticate a human decision.

Preparation leaves the task held and grants **zero launches**. Use the existing
`authorize-budget-adjustment` command with `mode: "additional-cycle"` and the
same actual receipt for the first follow-up implementation/review pair. Then
refresh if needed and `resume`. Additional cycles require fresh receipts after
the preceding review launch is consumed. Failed or cancelled launches consume
the pair normally. Historical allowances do not authorize an ordinary follow-up.
For a closed PR replaced externally and a wholly unused exercise pair, use the
separate [superseded recovery contract](superseded-recovery.md).

## Replace the worker safely

Use `reserve-slot`, then `provision-slot` with a different numbered slot and a
new task branch. Provisioning still uses the repository helper. A safely
provisioned owned checkout can instead use `bind`. Both paths validate receipt,
repository ownership, clean state, branch identity and the prepared main base.
Ordinary binding does not reuse an archived slot, even if its old checkout is
absent. This keeps existing local work, branches and reservations intact and
avoids reassignment based merely on a missing directory.

The current user may make a narrow maintenance exception for an exact follow-up
task, completed historical task and slot. Preserve the historical branch first.
Verify that no unfinished task owns the slot, reset the run-owned checkout with
the repository helper, create the new task branch from fresh `origin/main`, and
use the `historical_slot_reuse` bind request documented in
[the controller contract](controller.md#brief-worker-and-snapshots). The bind
records the receipt and still fails on dirty, foreign, redirected, wrongly
reserved or stale checkouts. It grants no launch and cannot act after current
delivery work or proof begins.

The base is pinned to origin/main fetched during preparation. Bind promptly.
If origin/main changes before an ordinary bind, the controller fails closed.
The exact historical-slot authorization above can refresh an otherwise unused,
unbound preparation during its bind. It updates the current prepared base and
unused current allowance snapshot together. It does not reset the original
baseline, proof, history or budgets. After binding, ordinary commits and
snapshot updates follow the existing rules.

## Plan, evidence and delivery

Call `loop-plan` after binding. Its `baseline_commit` and per-criterion baseline
records still describe the original task base, found under the first
`delivery_history` record. Its snapshot and the reviewer snapshot use the new
delivery base. Revise current components and examples as needed while keeping
the full deliverable and original baseline records. Current agent context
includes the original base, primary PR and follow-up authorization.

Continue with the usual commands:

1. `agent-begin` or `run-agent`, then actual terminal `agent-finish`.
2. `snapshot`, `checkpoint`, current measurements and evidence for every criterion.
3. Independent review, `validate-review`, `agent-finish`, and dispositions.
4. `gh-prepare` and `gh-run` for create, ready and merge, with the normal Jira
   bridge and required checks.
5. `final-verify` naming the **current** merge, full acceptance, Jira Done,
   `complete`, then current worker cleanup.

Preparation clears current evidence, review, plan, measurements and human review.
No previous PASS automatically proves the follow-up. Any evidence reuse must be
explicitly re-recorded for the current snapshot, with a description identifying
the historical proof and explaining why it remains applicable. All full criteria
still require current satisfied measurements and passing evidence. Final
verification also verifies that all historical merge commits remain on main.

## Schema7 and history

Task `pr`, `branch`, `slot`, `snapshot`, `main_verified`, `merged_commit`, checks,
and acceptance records describe the current delivery. `final_verified` describes
full task acceptance. `followup` stores its serial identity, receipt path/digest,
source, scope, requirements, prepared base and timestamp.

`delivery_history` archives the previous task record, verified GitHub observation,
timestamp and original operation/launch IDs. Each archived task omits its own
history to avoid repeated nesting. The first record retains the original task
base and primary PR. Generated ledger rows show primary PR, current PR, delivery
identity and history count. Status exposes the complete archives.

Top-level launches, usage, findings, dispositions, counters, consumed
exceptions, checkpoints and stall counts remain intact. Prior role-session bindings
stay in the archived delivery and original launches. The follow-up starts new
role sessions so recovery does not depend on unavailable native sessions. Every extra-cycle receipt
has a `delivery_id`, defaulting to zero for older records. Preparation consumes a
new serial ID; later launches and operations belong to that delivery. Earlier
IDs remain historical and cannot dispatch or settle the new delivery. Historical
PR observations verify identity and main ancestry without writing current state.
Per-head deadlines remain recorded and cannot restart through preparation.

The full task criteria remain immutable throughout this follow-up. Ordinary
single-PR tasks keep their existing behavior. Defaults load schema1 through
schema6 without writing schema7; only successful preparation introduces the new
schema. An accepted historical-slot bind writes schema10. Do not run an older
binary after either write.
