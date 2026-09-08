# Session continuity and review evidence

Read on handoff/restart, or when preparing a review packet. The controller owns
counters and delivery facts. Narrative files preserve user intent and explain
proof; they must not introduce new restrictions.

## Preserve the objective

Rewrite the current `HANDOFF.md` at a task boundary instead of appending successive
instructions. Archive the old file once. Keep these fields distinct:

| Field | What belongs here |
| --- | --- |
| Objective | The user's full epic objective and explicit exclusions, with the instruction source. |
| Current task | Where to resume first, and the next eligible work after it. This is not a scope restriction. |
| Decisions | Accepted business decisions and source receipts; do not reopen them on restart. |
| Authority | Each bounded operation, source receipt, target, remaining allowance, and whether it is unused, consumed, revoked or superseded. |
| Delivery | Controller location, current delivery/stage, PR identities, unresolved criteria, retained work and uncertain operations. |
| Next actions | Concrete authorized work, followed by the exact blocker for any dependent operation. |
| Usage | `usage-sync` result, coordinator interval imported, and remaining unknown launch IDs. |

When a newer instruction supersedes one restriction, replace that restriction in
the current handoff and link its history. Preserve unrelated boundaries. Never
leave both “deployment authorized” and “obtain deployment permission” as current
instructions for the same operation. Finishing a session does not revoke unused
authority, grant another consumed attempt, or narrow an epic to its current task.

On restart, compare the current user instruction with the objective and authority
receipts. Resolve an obsolete handoff from those sources without asking the user
to repeat an already-clear instruction. An instruction to stop after this task
ends that session at the task boundary; record whether any continuing scope
restriction was actually stated. Continue other eligible work when the current
task is held, unless the user has explicitly limited the scope.

## Build a compact review packet

Before a normal review launch, run `review-readiness`. Resolve missing evidence
or checkpoints before spending the review budget. Keep the current review, merge,
Jira Done and controller settlement out of acceptance criteria that must precede
those very gates. Product criteria describe behavior; the controller enforces
review and delivery separately. A requested investigation of evidence gaps uses
`allow_evidence_gaps:true` and a specific question, and still cannot PASS with
missing proof.

Include the complete criteria and a compact map for the changed behavior:

| Boundary | Evidence to record |
| --- | --- |
| Callers and enforcement | Entry point, shared implementation, indirect policy/configuration inputs, relevant generated sources and consumers. Trace actual calls as well as identifier searches. |
| Invocation variants | Relevant environments, event types, defaults and absent inputs. Name which variants changed and which contract tests exercise them. |
| Documentation | Active customer/developer/operator instructions and the reason for excluding historical records. |
| Invariants | Fields or states that must agree, invalid combinations and the real executable boundary that rejects them. |

For a missing-consumer repair, attach the map and searched revision/terms before
verification. A grep match list is supporting evidence, not proof that indirect
behavior is covered. Test ownership/coverage inventories against required behavior,
not just the entries already in the proposed implementation. For example, trace a
permission's route, mutation wrapper, settings source and repository, not only
paths containing `security`.

Use accepted `reference-propagation` lessons when that family applies. Do not add
unrelated families merely to load more context. Before review, confirm the selected
lesson changed the coverage/search work; having the tag alone proves nothing.

## Prove the real contract

Identify identity and consistency invariants before writing fixtures. For an
artifact inventory, matching source/digests does not establish matching release
classification, version and tag. For a workflow input, test missing properties on
non-dispatch callers as well as explicit true/false values on dispatch. Apply
these examples only where those contracts exist.

Exercise the production CLI, predicate, parser or shell step with valid inputs and
representative contradictory inputs. A local test helper that restates the desired
predicate does not prove the real gate. Label source inspection, shell execution,
composed runtime proof and human evidence accurately. Independent review should
challenge the invariant and fixture assumptions before repeating passing tests.

## Retain primary measurement evidence

Use controller `run-check` for ordinary task checks; it records the actual command,
working directory, snapshot, process output, elapsed time and terminal outcome.
Its `check-<id>.stdout`, `.stderr` and JSON receipts remain ignored local evidence.
Do not replace them with a prose claim.

For an authorized temporary fault or repeated timing experiment, use
`scripts/capture-proof.py` around each actual command. It requires Python 3.9+
and an existing ignored output parent. Supply a new output directory per sample
and each relevant changed file through `--path`:

```sh
python3 "$EPIC_SKILL/scripts/capture-proof.py" \
  --cwd "$OWNED_WORKER" --output "$EPIC_RUN/$TASK/timing-1" \
  --path path/to/changed-source \
  -- actual-command argument
```

The helper stores intent before launch, stdout/stderr, actual exit code, wall time,
tracked diffs/status and the selected files' hashes before and after the command.
It prints only a receipt path and exit code. It grants no operation authority and
performs no automatic fault injection or restoration. Pass credentials through
existing environment/authentication, never argv. Keep raw receipts private and
return sanitized excerpts to reviewers.

Capture baseline, the exact temporary fault, failing command output, restoration
and the restored passing check. A clean final tree proves removal but cannot prove
that the earlier fault was detected. Compare restored hashes/diff with the baseline,
preserving unrelated pre-existing changes. A killed command or interrupted helper
may leave only intent/logs; reconcile actual effects before repeating it. Do not
claim missing timing or fault evidence from memory, and do not automatically rerun
a live exercise to replace missing evidence.

## Reconcile only the work that needs fresh observation

Use `reconcile-plan` with the selected key before claim/resume. Observe its listed
open, uncertain or otherwise relevant deliveries. Completed receipts can be reused
when nothing contradicts their PR/merge/Jira identity. The current task, open PR
capacity, failed synchronization and uncertain operations remain fresh checks.

At restart and final reporting, call it without a key to include all unfinished
known deliveries once. Do not query every completed PR at every task boundary.
An explicit external change or contradictory Jira/branch observation requires a
fresh read even if a receipt was previously complete. Do not cache live health,
authorization, changed prerequisites, or exact-head checks needed for a merge.
