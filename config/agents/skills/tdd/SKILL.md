---
name: tdd
description: "Implement or repair predictable software behavior through disciplined test-driven development: a living behavioral test list, one meaningful failing test at a time, the smallest real passing change, optional green-state refactoring, and explicit evidence. Use for behavior changes, regression fixes, interface-driving examples, or a validated $ship behavior handoff. Do not use for pure refactoring, unsettled requirements or architecture, deployment, research, or behavior without a timely trustworthy executable oracle."
---

# Develop One Behavior at a Time

Use TDD as a programming and local design-feedback discipline, not as a ritual
or a complete quality strategy.

## Confirm TDD Fits

Read [boundaries.md](references/boundaries.md). Continue only when:

- intended inputs, outputs, state transitions, or effects are predictable;
- one useful example can be checked automatically;
- feedback can be made timely and sufficiently deterministic;
- the required product and consequential architecture decisions are settled;
  and
- the tests can observe behavior without depending on private structure.

When these conditions fail, stop and name the missing oracle, decision, or
separate verifier. Do not manufacture a unit seam or mock-shaped architecture
merely to claim TDD.

## Orient and Baseline

1. Read repository instructions, the approved behavior, relevant production
   path, callers, and existing tests.
2. State preserved behavior and non-TDD risks such as security, performance,
   concurrency, compatibility, migration, or operations.
3. Run the fastest relevant baseline checks. Separate pre-existing failures
   from the new behavior.
4. Identify the repository's normal focused and broader test commands.

## Build a Living Behavioral List

List behavior, not classes or implementation tasks:

- basic success;
- meaningful variants and boundaries;
- rejected inputs and failures;
- regressions and preserved behavior;
- external effects, state transitions, and lifecycle events;
- unanswered behavioral questions; and
- design concerns discovered but not yet justified.

Keep implementation ideas separate. Add discoveries as they appear. Do not
write the entire list as speculative test code.

## Select One Informative Example

Read [mechanics.md](references/mechanics.md). Choose one example that is small,
caller-relevant, forces a useful interface decision, distinguishes a plausible
wrong implementation, and can fail for a clear reason.

Write setup, invocation, and an expected result derived independently of the
production implementation. Prefer a meaningful behavioral boundary over one
test per method, class, or branch.

## Red

1. Add exactly one concrete automated test.
2. Run the narrowest command that exercises it.
3. Confirm it fails because the intended behavior is absent.
4. Record the command, failure, and why the failure is meaningful.

If it passes, fails in setup, or fails for an unrelated reason, do not change
production code. Determine whether behavior already exists, the test cannot
observe it, the fixture is wrong, or the plan is stale.

Never weaken an assertion, copy actual output into the expectation, disable a
check, or accept a failure that does not test the named behavior.

## Green

1. Make the smallest real production change that satisfies the example.
2. Run the focused test and all relevant prior tests.
3. Prefer an obvious general implementation when it is clear.
4. Use a narrow fake only as a temporary green step followed immediately by an
   example that forces generalization.
5. Add discoveries to the living list; do not silently expand scope.

When the implementation overfits examples, use triangulation: name the
simplifying assumption, add one example that breaks it, then generalize so both
pass.

## Refactor From Green

Read [test-and-design-quality.md](references/test-and-design-quality.md).
Refactoring is optional. Begin only from green, keep observable behavior fixed,
make one structural transformation, and run fast checks after each microstep.

Remove justified duplication or improve a named responsibility, dependency,
representation, or interface. Do not add an abstraction without a current
example or approved change pressure. Invoke `$refactor` for an unfamiliar,
multi-step, legacy, or published-interface transformation.

## Repeat or Stop

At each green state choose deliberately:

- refactor a justified structural issue;
- select the next informative behavior;
- run a broader non-TDD verifier;
- stop because agreed behavior is complete; or
- return for clarification because evidence invalidated the plan.

Shrink the next step when failures are difficult to explain, feedback is slow,
or several changes could explain the result. Reorder examples when repeated
small steps create only test-specific production branches.

## Hand Off Evidence

Use [evidence.json](assets/evidence.json) as the output shape. Report:

- behavior list with implemented, deferred, and discovered cases;
- each meaningful red and corresponding green command/result;
- interface and implementation-design decisions;
- refactoring checkpoints;
- focused and broader tests run;
- test-quality limitations;
- non-TDD verifiers and residual risks.

Never claim that a green suite establishes architecture, security, performance,
reliability, usability, or production safety outside its observation boundary.

## Reference Route

- [mechanics.md](references/mechanics.md): step sizing, green tactics,
  triangulation, and stopping.
- [test-and-design-quality.md](references/test-and-design-quality.md): test
  quality, determinism, mocks, and testability signals.
- [boundaries.md](references/boundaries.md): prerequisites and decisions TDD
  cannot own.
- [sources.md](references/sources.md): evidence provenance and limits.
