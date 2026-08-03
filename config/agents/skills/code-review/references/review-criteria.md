# Review Criteria

## Requirements and Correctness

- Trace each acceptance criterion to behavior and evidence.
- Exercise representative success, failure, boundary, and regression paths.
- Inspect errors, side effects, ordering, state transitions, partial failure,
  retry, cancellation, and resource cleanup where affected.
- Search for callers and configurations beyond obvious static references.
- Check whether the implementation silently narrowed, expanded, or reinterpreted
  the plan.

## Tests and Evidence

- Confirm a new test would fail for the intended missing or incorrect behavior.
- Derive expected results independently of production output.
- Prefer behavioral observation over private fields, call order, or object
  layout.
- Inspect skipped tests, loosened assertions, snapshot churn, deleted cases,
  new ignores, and test-only production branches.
- Check determinism around time, randomness, ports, remote services, shared
  state, execution order, and cleanup.
- Require integration, performance, security, migration, or operational
  evidence where unit tests are not predictive.

## Design and Maintainability

- Can a future maintainer understand the changed behavior locally?
- Does each rule, policy, invariant, or representation have one clear owner?
- Does a small coherent interface hide useful knowledge?
- Can the next likely change remain localized?
- Do stable policy and domain concepts avoid depending on volatile details?
- Are abstractions earned by present behavior or approved change pressure?
- Did mechanical cleanup obscure semantic change?
- Can obsolete code or indirection be deleted?

Treat smells and pattern names as prompts. Report a design defect only when a
credible maintenance scenario and consequence exist.

## Compatibility and Data

Inspect published APIs, serialized formats, schemas, events, errors,
configuration, reflection, generated clients, and external consumers. Require
coexistence, migration, versioning, rollback, and mixed-version evidence where
the plan affects them.

## Security and Operations

Inspect inputs, privileges, trust boundaries, secrets, dependencies, failure
behavior, rate or resource limits, logs, metrics, traces, alerts, deployment
markers, and recovery. A green scanner or suite does not replace reasoning
about novel risks.

## Review Standard

Prefer continuous improvement of code health over perfection. Technical facts
and requirement evidence outrank preferences. A nonblocking suggestion should
not hold a correct, maintainable change hostage to unrelated polish.
