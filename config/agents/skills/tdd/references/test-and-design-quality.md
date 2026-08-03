# Test and Design Quality

## A Meaningful Red

A useful test would fail for the intended missing or incorrect behavior. It
does not count when it:

- was always green;
- failed only because setup, syntax, imports, or environment were broken;
- copied the implementation's observed output into the expectation;
- weakened or removed an assertion;
- depended on execution order or shared residue; or
- observed a private call sequence rather than the contract.

For a defect, confirm the regression test fails without the fix when practical.

## Observation Boundary

Test fundamental behavior through the narrowest meaningful public or domain
seam. Keep tests readable, specific, isolated enough to diagnose, deterministic
where controllable, and fast enough for the claimed loop.

Control time, randomness, ports, resource identifiers, remote services, and
cleanup where appropriate. Do not conceal unavoidable concurrency or
distributed nondeterminism behind larger sleeps or timeouts.

Review generated tests as production-quality code. Coverage reveals unexecuted
areas; it is not a correctness target.

## Mocks and Testability

Use a test double when it represents a meaningful dependency contract and
improves feedback without erasing important semantics. Avoid:

- mocking private collaborators and call order;
- adding interfaces solely for a test;
- replacing integrated evidence with isolated interaction assertions;
- wrappers that add indirection without hiding useful knowledge; and
- test-only branches in production.

Difficult setup, many unrelated failures, hidden time or state, and slow
feedback may reveal responsibility or dependency problems. Treat them as
design signals, then compare whether changing the test boundary or production
boundary better serves a likely change.

## Refactoring Quality

Refactor only from green. Name the structural pressure and verifier. Prefer one
small behavior-preserving move, inspect the diff, and keep only a green step.
Stop when cleanup no longer helps the current behavior or next credible change.
