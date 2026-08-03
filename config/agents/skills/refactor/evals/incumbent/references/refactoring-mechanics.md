# Refactoring Mechanics

## Contents

- [Preserve an Explicit Observation Boundary](#preserve-an-explicit-observation-boundary)
- [Work Under One Hat](#work-under-one-hat)
- [Select a Transformation From the Pressure](#select-a-transformation-from-the-pressure)
- [Refactor Legacy Code Safely](#refactor-legacy-code-safely)
- [Choose the Refactoring Workflow](#choose-the-refactoring-workflow)
- [Evolve Published Contracts as Migrations](#evolve-published-contracts-as-migrations)
- [Stop Conditions](#stop-conditions)

## Preserve an Explicit Observation Boundary

Fowler's precise definition holds observable behavior fixed while improving internal
structure. Declare what is observable for this task; it may include:

- returned values, errors, events, and external side effects;
- persistent data, serialization, ordering, and state transitions;
- API signatures and protocol behavior visible to published consumers;
- logs, metrics, or files consumed by automation; and
- latency, memory, throughput, or resource use only when contractual.

Tests provide evidence over what they observe, not proof of equivalence. Expand the
safety net before the first structural edit when the observation boundary is poorly
represented.

## Work Under One Hat

Use Beck's two-hats discipline and add a separate migration category:

- Refactoring hat: restructure; keep the agreed boundary fixed.
- Adding-function hat: introduce or correct behavior; require a meaningful failing
  example before the production change when practical.
- Migration hat: change a durable or published contract through coexistence,
  compatibility, and retirement.
- Optimization hat: measure, profile, change, and compare; do not infer success from
  functional tests.

Switch often if needed, but return to green and make the transition explicit. Do not
hide a feature or bug fix inside a cleanup diff.

## Select a Transformation From the Pressure

Use catalog names as a shared vocabulary, not a checklist.

| Pressure | Candidate moves | Check before choosing |
| --- | --- | --- |
| Meaning is obscure | Rename Variable/Field/Function, Extract Variable, Extract Function | Does the new name express domain intent rather than mechanics? |
| One routine mixes stages | Split Phase, Extract Function, Move Function | Will the new boundary hide knowledge, or force readers to reconstruct conjoined flow? |
| Responsibility is misplaced | Move Function/Field, Extract Class, Inline Class | Which module owns the rule and why does the change become more local? |
| Data rules leak | Encapsulate Variable/Record/Collection, Introduce Parameter Object, Replace Primitive with Object | Is there a real invariant or behavior, not merely a desire for another type? |
| Branching hides policy | Decompose Conditional, Consolidate Conditional Expression, Guard Clauses, Replace Conditional with Polymorphism | Is variation stable and meaningful enough to justify dispatch machinery? |
| Indirection adds no value | Inline Function/Class, Remove Middle Man/Subclass/Dead Code | Can callers become simpler without exposing volatile knowledge? |
| A change must be isolated | Extract Function/Class/Interface, Encapsulate Variable, Split Phase | Is the seam present for a current pressure rather than speculative extensibility? |

Inverse refactorings are equally valid. Extraction is not automatically improvement;
inlining may restore locality and remove a shallow boundary.

Use the microstep loop:

1. run the current fast verifier;
2. apply one mechanically comprehensible move;
3. compile, type-check, lint, or test as close to the edit as possible;
4. inspect the diff and affected callers;
5. keep green or repair/undo only the last move; and
6. checkpoint only through the repository's authorized workflow.

## Refactor Legacy Code Safely

Use the Feathers-derived sequence when unfamiliar code lacks tests:

1. Identify the exact change point.
2. Find a trustworthy observation/test point.
3. Break only the dependency that blocks observation or control.
4. Add characterization or regression evidence for current behavior.
5. Make the requested change and refactor in small verified steps.

Characterization captures what the system does, including surprising behavior; it
does not assert that all existing behavior is desirable. If a behavior is a bug, first
record it, then switch hats and specify the corrected behavior.

Use the smallest seam possible: parameterize a dependency, wrap a hard external
boundary, extract a pure calculation, or expose a stable behavior-level entry point.
Avoid building a dependency-injection framework merely to make one test isolated.

## Choose the Refactoring Workflow

- **Comprehension:** encode newly learned intent in names and structure while reading.
- **Litter pickup:** make a tiny safe local improvement encountered during other work.
- **Preparatory:** reshape the code so the imminent feature has a natural home.
- **TDD:** improve implementation design from green before the next example.
- **Planned:** bound a larger effort that cannot fit safely inside current work.
- **Long-term:** move toward a rough target through many working, compatible states.

Tie each workflow to an economic boundary. Stop opportunistic cleanup when adjacent
work no longer makes the requested change safer or clearer. A rewrite needs its own
business case, migration strategy, and rollback plan.

## Evolve Published Contracts as Migrations

Treat published APIs, events, files, database schemas, configuration, and serialized
representations as behavior. Visible repository callers are not a complete consumer
inventory.

Use expand-migrate-contract:

1. inventory producers, consumers, readers, writers, reflective uses, and generated
   bindings;
2. add a compatible new path while the old path remains valid;
3. migrate code and data with observable progress;
4. verify old/new and mixed-version behavior plus rollback;
5. remove the old path only after evidence shows it is unused.

For a long implementation replacement, use Branch by Abstraction only when parallel
paths are genuinely required: introduce a narrow boundary, move consumers, replace
behind it, then simplify the temporary mechanism.

## Stop Conditions

Pause when:

- the starting state is red for an unexplained reason;
- the next edit cannot be explained as one recoverable transformation;
- externally observable behavior must change;
- tests or other observation are inadequate for the risk;
- a published consumer or persisted representation is unknown;
- performance is the actual success criterion but no measure exists;
- several structural purposes have accumulated in one diff; or
- expected maintenance payoff no longer justifies the scope.
