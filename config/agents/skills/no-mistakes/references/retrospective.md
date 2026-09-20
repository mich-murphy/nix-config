# Retrospective

Read this at handoff, after `usage-report`. The retrospective is the run's
own evidence turned into a reviewable artifact: what was delivered, what it
cost, where the process itself created friction. Compile it; do not write it
from memory.

## Sources

Everything comes from state the run already recorded — never invent entries.

- `status`: per-task phase, holds, deliveries, PRs, budgets, authorities.
- `usage-report`: recorded and unknown usage per launch; judge verdicts that
  have landed.
- The ledger (the run directory's SQLite, read through command output only):
  rejections, holds, escalations, and repair rounds per task.
- Judge verdicts: the friction events and `improvements` each judge named.

## Format

Store as `<run>/retrospective.md`; summarize the highlights in the handoff
message.

```markdown
# Retrospective: <work-key> (<date>)

## Outcome
| Task | Result | PR | Head | Judge |
| --- | --- | --- | --- | --- |
| WORK-1 | merged, verified | #12 | abc1234 | pass 91 |

## Cost
- Launches by role and tier; model substitutions and their reasons.
- Recorded usage; launches with unknown usage.
- Wall time per task from claim to complete.

## Friction
- Every hold raised, its cause, and what resolved it.
- Rejections by class (the controller's refusal log): which rules fired
  and whether the rule or the behavior was wrong.
- Escalations and tier raises, with the triggering stall.
- Repair rounds per review; recurring finding families.

## Authority
- Receipts used (pair, delivery, narrowing, merge, slot reuse) and what
  granted them. A merge-receipt count per task is expected: zero or more,
  never reused.

## Improvements
- Judge-named improvements, accepted or rejected with reasons.
- Rules that fired often enough to want promotion to a mechanism (see
  [enforcement.md](enforcement.md)).
```

## Reading it

One run is an anecdote. The retrospective's value is across runs: recurring
holds point at missing authority defaults, recurring rejection classes point
at rules fighting the coordinator, and recurring finding families are lesson
candidates for `lesson-record`. Compare retrospectives before changing the
skill; a single bad run is not a trend.
