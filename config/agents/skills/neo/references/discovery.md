# Discovery Contract

Discovery reconstructs the problem and current system without deciding the
solution.

## Investigate

- Read applicable repository instructions first.
- Trace current entry points, data/control flow, tests, schemas, dependencies,
  history, and established conventions.
- Use source locations for current-state claims.
- Ask how the relevant system works before asking how to change it.
- Keep solution preferences out of factual research questions.

## Artifact

Cover:

- `Current state`: observed behavior and affected people or systems.
- `Facts`: inspectable evidence and locations.
- `User statements`: supplied intent, priorities, and constraints.
- `Inferences`: interpretations with supporting evidence.
- `Unknowns`: question, owner, blocking status, and cheapest way to resolve it.
- `Success`: observable outcome and existing verification surfaces.

Do not claim that repository silence proves a product preference. Do not
propose architecture, modules, or implementation steps.

If targeted inspection immediately reveals one blocking unknown that prevents
meaningful discovery, use the blocked-discovery fast path: record that one
unknown, create no artifact, skip the gate, and ask exactly one compact
question. Resume discovery only after the answer is recorded.

## Gate

Discovery is ready when the problem is understandable, current behavior is
traceable, ambiguous terms are resolved or recorded, and the next stage can
work from compact evidence rather than exploration transcripts.
