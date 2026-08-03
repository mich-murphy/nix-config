# Plan Readiness and Stop Conditions

## Admission Contract

Admit a plan only when repository inspection confirms:

1. One observable outcome and affected users or operators.
2. Testable acceptance criteria plus explicit non-goals.
3. Approved public behavior, interfaces, ownership, invariants, and data
   semantics affected by the implementation.
4. Compatibility, migration, security, reliability, and operational
   constraints where relevant.
5. The ordinary repository verification path.
6. No open question whose answer could materially change the implementation.

The plan may identify implementation choices that TDD should explore. It may
not defer product intent, public contracts, persistent-data meaning, trust
boundaries, or other expensive-to-reverse decisions to the implementation
agent.

## Inspect Without Researching

Read repository instructions, source, tests, schemas, configuration, local
history, and runtime artifacts already supplied or locally available. Use this
inspection to locate the planned behavior and detect contradictions.

Do not search externally, choose among unsettled product outcomes, or expand
the plan into a new architecture exercise. If external facts or stakeholder
decisions are required, stop.

## Stop Handoff

Return:

```text
status: plan_not_ready | replan_required | verifier_unavailable
evidence: repository locations, commands, or observed behavior
missing_decisions: exact unresolved items
impact: why safe implementation cannot continue
requested_handoff: the smallest plan clarification or decision required
preserved_state: worktree and checks already performed
```

Ask only questions that repository inspection cannot answer. Do not create a
partial implementation to make a missing decision appear concrete.
