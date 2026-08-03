# Fresh Review and Remediation

## Review Packet

Give the reviewer primary artifacts:

- original plan and readiness record;
- repository instructions;
- base revision and complete candidate diff;
- changed and relevant existing tests;
- red, green, refactoring, and verification evidence;
- compatibility, security, and operational constraints.

Exclude the implementation transcript and persuasive implementation summary.
The reviewer should reconstruct intent rather than inherit the implementer's
attention and assumptions.

## Independence

Use a fresh session or a separate read-only agent with no authority to edit,
commit, push, or resolve feedback. The implementation owner retains scope and
acceptance. A review performed by the implementation context is useful
preparation but does not satisfy the independent gate.

## Finding Routes

| Finding | Route |
| --- | --- |
| Missing or incorrect behavior | `tdd` |
| Internal complexity or maintainability problem with behavior fixed | `refactor` |
| Missing or contradicted requirement or consequential design decision | `replan` |
| Ambiguous evidence or ownership | `clarify` |
| Nonblocking observation with no justified change | `accept` |

The reviewer never applies the route. The ship owner validates the finding and
dispatches the corresponding specialist.

## Bounds

Use the state machine's configured review-cycle limit. Stop early when:

- the same blocking finding recurs without new evidence;
- remediation expands beyond the approved plan;
- a fix requires a consequential upstream decision;
- verification becomes unreliable; or
- candidate churn makes another automatic attempt less safe than human review.

Every candidate change invalidates the preceding verification and review.
