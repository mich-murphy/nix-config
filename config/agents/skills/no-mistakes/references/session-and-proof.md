# Session continuity and review evidence

Read this after interruption or when preparing an independent review.

## Resume from recorded facts

Start with `status` and `next` in the original run directory. Inspect any process,
GitHub, and Jira outcome that remains unsettled. Do not repeat a mutation because
a later status update failed. Reuse confirmed closed-delivery receipts unless a
new observation contradicts their PR, head, merge, or Jira identity.

A compact handoff keeps these facts separate:

| Field | Contents |
| --- | --- |
| Objective | Full user objective and explicit exclusions |
| Current task | Exact phase, hold, delivery, and next action |
| Decisions | Business decisions and their receipts |
| Authority | Grant, scope, target, and consumed or unused state |
| Delivery | Branch, slot, base, head, PR, merge, and unresolved criteria |
| Recovery | Owned process identity or external observation still needed |
| Usage | Recorded totals and unknown launches |

Replace obsolete restrictions rather than appending contradictions. A session
boundary does not revoke unused authority, renew a spent grant, or narrow the
whole run to one task.

## Review packet

Resolve readiness failures before launching a normal reviewer. Keep review,
merge, Jira Done, and controller settlement out of product criteria that must
precede those gates.

Give the reviewer the immutable base and head, full current delivery criteria,
requirements digest, and a compact coverage map:

| Boundary | Required evidence |
| --- | --- |
| Callers | Entry point, shared implementation, policy inputs, and consumers |
| Variants | Relevant environments, defaults, absent inputs, and event types |
| Documentation | Active instructions and excluded historical records |
| Invariants | Fields that must agree and the public boundary that rejects conflicts |

For a missing-consumer repair, include the searched revision, terms, matches,
and classifications. Trace actual calls as well as names. A path list does not
prove an indirect rule is covered.

## Prove the contract

Write down identity and consistency rules before fixtures. Test valid inputs and
representative contradictions through the production CLI, parser, predicate, or
adapter. Do not use a helper that merely repeats the intended rule. Label source
inspection, command execution, runtime proof, and human evidence accurately.

`run-check` records argv, working directory, timeout, process identity, exit, and
terminal state. Keep its raw output in the ignored run directory. For deliberate
fault tests, retain the clean baseline, exact temporary fault, failing result,
restoration, and restored passing result. A clean final tree cannot prove the
fault was detected earlier.

A timeout or killed process never creates passing proof. If recovery finds a
still-running owned process, terminate only through `recover-operation`. If an
external mutation may have succeeded, observe that system instead of inferring
failure from the local process.
