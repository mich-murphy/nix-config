# Context-Aware Task Sizing

Use task sizing to protect model attention and feedback quality. It is not an
estimate of lines changed, elapsed time, or developer effort.

## Default Policy

When no measured harness policy exists, record:

- advertised window class: 200,000 tokens;
- warning threshold: 100,000 tokens; and
- reserve: 50 percent.

The 100,000-token warning is an experience-based HumanLayer heuristic, not a
universal model limit. Prefer a repository's measured threshold. Do not fill a
larger advertised window merely because it is available.

## Assess the Whole Loop

For each task record these sizing drivers:

1. **Starting context:** task plan, repository instructions, relevant source,
   callers, tests, schemas, and configuration.
2. **Boundary breadth:** independently changing modules, packages, services,
   interfaces, data stores, or deployment surfaces.
3. **Behavior breadth:** distinct success paths, failure paths, migrations, and
   compatibility modes.
4. **Feedback load:** expected test, build, browser, log, or generated output;
   feedback latency; and likely iterations.
5. **Uncertainty:** unfamiliar code, ambiguous repository evidence, flaky
   verifiers, or assumptions likely to force replanning.
6. **Remediation reserve:** room to diagnose failed verification and address
   review findings without losing the task's governing decisions.

Classify a task as `well-below-warning` or `within-warning`. Do not emit an
`over-warning` or `low`-confidence task as Ship-ready. Split it or stop for
upstream clarification.

## Split When

- the task contains two outcomes that can be independently demonstrated;
- separate risky failure, migration, or rollback paths can be made coherent;
- unrelated source regions or verifier suites dominate the context;
- implementation and verification together approach the warning threshold;
- the task requires retaining broad exploration that can instead be compressed
  into an approved boundary; or
- a concrete split trigger recorded in the task becomes true.

Split at behavior, compatibility phase, or independently green migration
boundaries. Preserve a vertical path and explicitly connect dependencies.

## Merge When

- a task only adds schema, plumbing, tests, or UI with no independent behavior
  or named migration risk;
- two fragments require the same source orientation and verifier and neither is
  useful alone;
- a prerequisite exists only to satisfy the decomposition rather than a real
  repository constraint; or
- handoff and repeated setup cost would consume more context than the boundary
  saves.

Do not merge merely to reduce task count.

## Reassess During Execution

The graph is a forecast. A Ship run must stop and return upstream when actual
repository evidence invalidates scope, dependencies, approved decisions, or the
context assessment. Update the task graph and source-plan linkage before
dispatching affected tasks again.
