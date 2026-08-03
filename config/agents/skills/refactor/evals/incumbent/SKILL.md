---
name: refactor
description: Review and refactor existing code for safer change, clearer design, lower complexity, and long-term maintainability. Use when asked to review code health or design, simplify or restructure code, remove technical debt or code smells, improve abstractions or module boundaries, apply or assess design patterns, prepare code for a feature, or make an existing codebase easier for humans to understand without unintentionally changing behavior. Also use when a requested cleanup overlaps tests, public APIs, compatibility, or migration risk.
---

# Review and Refactor Code

## Route Model and Effort

Classify before acting:

- Use an efficient or balanced model at low or medium effort for read-only mapping,
  mechanical renames, formatting, and tightly specified local transformations.
- Use a balanced model at medium effort for a bounded refactor with a trusted test
  suite and clear acceptance boundary.
- Use a frontier model at high effort for ambiguous design, unfamiliar code,
  cross-module changes, weak verification, public contracts, concurrency, security,
  persistent data, or release-critical review. Reserve still higher effort for a
  demonstrated gap; more reasoning does not repair missing context or a weak test.
- Delegate only independent read-heavy investigation when the harness and user allow
  it. Keep requirements, design judgment, shared edits, and final acceptance with one
  owner.

Read [model-routing.md](references/model-routing.md) only when selecting an exact
current model, configuring agents, or evaluating cost versus quality.

## Declare the Mode

State one mode before editing:

- **Review mode:** inspect and report; do not modify files unless asked.
- **Refactoring mode:** improve internal structure while declared observable behavior
  remains fixed.
- **Behavior mode:** add or correct behavior and change tests to specify it.
- **Migration mode:** deliberately evolve a published interface, persistent format,
  dependency, or architecture through a compatible transition.

Do not call a rewrite, feature, bug fix, optimization, or migration a refactor. If a
task mixes modes, separate them into green, reviewable stages and say when the hat
changes. A valid review outcome may be **no change**.

## Orient Before Judging

1. Read repository instructions and inspect the smallest relevant code path, callers,
   tests, and history or runtime evidence available.
2. State the motivating maintenance task or next credible change. Do not optimize for
   an abstract ideal.
3. Define observable behavior: results, errors, side effects, state, data formats,
   ordering, public API shape, and contractual performance or telemetry.
4. Classify the boundary as private, repository-internal, or published. Search beyond
   static callers for configuration, serialization, reflection, generated code, and
   external consumers when relevant.
5. Run the fastest relevant baseline checks before editing. Report pre-existing
   failures rather than absorbing them into the refactor.
6. Map risk and knowledge: what must a maintainer understand, which module owns each
   rule or invariant, and which likely changes cross unrelated areas?

Read [testing-and-verification.md](references/testing-and-verification.md) when the
baseline is weak, tests must change, a bug is discovered, or non-functional behavior
is part of the contract.

## Diagnose Before Prescribing

Treat smells, static-analysis findings, and pattern names as investigation prompts.
For each material finding, record:

```text
observation -> change pressure -> underlying knowledge/design problem
            -> candidate transformation -> trade-off -> verifier
```

Use these lenses:

- **Comprehension:** Can names, control flow, and responsibilities be understood
  locally?
- **Information hiding:** Does a small coherent interface hide a useful decision,
  policy, or representation?
- **Change locality:** Does one rule have one authority, and can a likely change stay
  inside one module?
- **Dependency direction:** Do stable policy and domain concepts avoid depending on
  volatile details?
- **Tests:** Do they observe behavior without freezing private structure?
- **Deletion:** Can obsolete code, indirection, or configuration be removed safely?

For a consequential boundary, produce two materially different designs and compare
interface burden, hidden knowledge, dependencies, failure/state complexity,
compatibility, repository fit, and migration cost. Do not count renamed versions of
the same dependency graph as alternatives.

Read [design-and-patterns.md](references/design-and-patterns.md) for boundary review,
Ousterhout red flags, DRY/orthogonality/reversibility, or pattern selection. Read
[refactoring-mechanics.md](references/refactoring-mechanics.md) before a multi-step,
unfamiliar, legacy, or public-interface refactor.

## Execute in Verified Microsteps

1. Begin from a known baseline and name one structural purpose.
2. Choose one catalog refactoring or equivalent small transformation.
3. Make the smallest coherent edit that advances that purpose.
4. Run the fastest relevant build, type, lint, and behavioral checks.
5. Inspect the diff for accidental behavior, public-contract, generated-file, and
   test-fixture changes.
6. Keep only a green step. If red, repair or undo the last microstep before proceeding;
   shrink the next step when the cause is unclear.
7. Run broader integration and repository validation at boundary crossings and before
   handoff.

Prefer repository-native tooling and automated symbol refactors where reliable, but
still inspect dynamic uses and the final diff. Do not create commits, rewrite history,
add dependencies, or broaden scope without authorization.

Stop or change modes when behavior must intentionally change, the verifier is not
credible, an unknown published consumer may break, performance is the real goal, the
system must remain broken across speculative steps, or the cleanup no longer serves
the motivating task.

## Review the Result

In review mode, lead with prioritized findings and cite concrete code locations. For
each finding, explain the maintenance scenario, consequence, evidence, and smallest
credible remedy. Separate defects and contract risks from design opportunities and
nonblocking preferences. Do not report style taste as a correctness issue.

In refactoring mode, hand off:

- the motivating task and preserved observation boundary;
- named transformations and why they improve the next credible change;
- tests and checks run, including the starting baseline and any gaps;
- public consumers, data, performance, or operational dimensions inspected;
- remaining uncertainty, deferred smells, and explicit no-change decisions; and
- any point where the work switched into behavior or migration mode.

Never claim semantic equivalence, architecture quality, or production safety beyond
the evidence actually supplied.

## Reference Route

- [refactoring-mechanics.md](references/refactoring-mechanics.md): Fowler-style hats,
  catalog moves, legacy seams, workflow types, and compatibility transitions.
- [design-and-patterns.md](references/design-and-patterns.md): Ousterhout-style
  complexity review, changeability principles, and pressure-driven pattern selection.
- [testing-and-verification.md](references/testing-and-verification.md): safety nets,
  characterization, risk-based test selection, and non-functional verifiers.
- [model-routing.md](references/model-routing.md): semantic lanes, current Codex and
  Claude mappings, effort escalation, and route evaluation.
- [sources.md](references/sources.md): provenance, evidence limits, and primary links.
