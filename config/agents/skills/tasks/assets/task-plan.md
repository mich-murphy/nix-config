# Task: {task title}

- Source plan: `<path>`
- Source plan SHA-256: `<sha256>`
- Task ID: `<task-id>`

## Outcome

{One observable behavior or named migration/operational risk proved.}

## Acceptance Criteria

- {Caller-, user-, or operator-visible criterion.}

## Non-goals

- {Explicit exclusion inherited from or compatible with the source plan.}

## Preserved Decisions and Invariants

- {Approved contract, invariant, ownership rule, or failure behavior.}

## Repository Evidence and Scope

- Evidence: `<path, symbol, test, schema, command, or observed behavior>`
- Likely touch: `<bounded source or test surface; guidance, not permission>`
- Must not touch: `<out-of-scope boundary>`

## Dependencies and Preconditions

- Blocking task IDs: `<none or IDs>`
- Preconditions: `<repository or operational state required>`

## Implementation Guidance

{Minimum approved cross-boundary path and relevant source-plan references;
do not write the code in prose.}

## Verification

- Focused automated: `<command and expected evidence>`
- Broader regression: `<command and expected evidence>`
- Real interface or operational: `<observable proof or explicit not-applicable reason>`

## Compatibility, Rollout, and Recovery

- Compatibility/migration: `<requirement or not applicable with reason>`
- Rollout: `<step or not applicable with reason>`
- Rollback/recovery: `<step or not applicable with reason>`
- Documentation: `<required update or not applicable with reason>`

## Context Budget

- Window class: `<tokens>`
- Warning threshold: `<tokens>`
- Assessment: `<well-below-warning | within-warning>`
- Confidence: `<high | medium>`
- Sizing drivers: `<starting context, boundaries, behaviors, feedback, uncertainty>`
- Split if: `<concrete observed trigger>`

## Replan Triggers

- {Evidence that invalidates an approved decision, task boundary, dependency,
  verifier, compatibility property, or context assessment.}
