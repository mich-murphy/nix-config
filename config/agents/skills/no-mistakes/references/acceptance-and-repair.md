# Acceptance sequencing and complete repairs

Read this when a task needs external proof, changes a managed control, or gets a
missing-consumer or stale-reference finding.

## Establish the sequence

Map every criterion to its proof, prerequisite, and earliest verification point.
The point may be before merge, after merge, after a separately authorized
operation, or after human acceptance. Keep the original criterion. A passing
command cannot stand in for a different required result.

If full proof cannot exist before merge, record the conflict before opening a PR.
Prepare a narrow proposal that names the criteria for the first merge, remaining
criteria, later operation, owner, cleanup, and required authority. Only the user
can approve that sequence. With approval, use `narrow-acceptance` on the current
code delivery. The narrowed merge does not verify the task.

After a narrowed merge, use a `Verification` delivery when no code remains. Use a
new `Code` delivery with a fresh receipt when cleanup or correction code remains.
Do not weaken `brief`, relabel operational proof, or mark Jira Done between these
deliveries.

## Record a useful hold

Distinguish missing evidence, access, authority, and business decisions. Use
supported read-only discovery before calling access unavailable. Record the exact
criterion, observed state, source and time, work already attempted, available
tools, next bounded action, and the instruction that blocks it.

An OAuth failure may require reconnection. It does not revoke existing approval
or require approval for the same action again.

## Managed controls

For each changed identity, permission, trust rule, or managed setting, record:

- Its declaration and owner, including IaC or generated configuration.
- The effective selector and fallback values.
- Native readback of the relevant identity and permission scope.
- What the next reconciliation would create, retain, or remove.
- Regression evidence and current operator instructions.

Give these references to the first reviewer. A live deletion is incomplete when
reconciliation recreates it. Configuration readback proves configuration, not a
token exchange or workload. Do not dispatch an apply, deployment, or exercise
only to improve evidence unless the user authorized that operation.

## Repair affected references once

For a missed consumer or stale statement, search the tracked tree at the reviewed
revision. Use stable identifiers and relevant alternate forms. For a helper,
search slash paths, basenames, imports, invocations, tests, and fixtures. For a
retired capability, search its names, trigger, identities, linked decisions,
runbooks, and infrastructure comments.

Keep a search receipt with the revision, terms, matched files, classifications,
and exclusions. Separate active instructions and executable consumers from dated
history. Repair every task-caused contradiction in the same round. Do not rewrite
truthful history or unrelated debt.

Run documented diagnostics when they support acceptance. Include a clean case and
a representative deliberate leftover or invalid case. Confirm that the diagnostic
does not match its own instructions.

Before review, map callers, invocation variants, documentation, and consistency
invariants to proof. A grep list supports that map but does not prove behavior.
Tests must exercise the production parser, predicate, CLI, or runtime boundary
with valid and contradictory inputs.
