# Implementation Brief

The final brief is compact context for a fresh implementation agent. It is not
code written in English.

Use these exact level-two headings:

```markdown
## Intent
## Requirements and Non-goals
## Current-state Evidence
## Product Scenarios
## Architecture
## Program Design
## Delivery Slices
## Verification
## Compatibility, Rollout, and Recovery
## Assumptions, Risks, and Replan Triggers
```

Include only approved decisions and clearly marked non-blocking risks. Cite
repository evidence for current state. Show interfaces, types, invariants, and
the principal success and failure call paths where they matter. Every delivery
slice needs a verifier and real evidence.

## Review Loop

Finalize creates a versioned review candidate. The brief stays complete enough
for implementation; the conversation stays compact enough for an informed
human decision. Never substitute a path or hash for the review surface.

Present at most 650 words before the artifact metadata and approval question.
Do not omit required review information to meet the limit; compress the source
material instead:

- **Outcome:** intended behavior plus the one or two exclusions most likely to
  be misunderstood.
- **Critical decisions:** distinct Product, Architecture, and Program groups,
  each containing one to three consequential choices, their rationale or
  trade-off, and the main consequence to challenge. Use real boundaries,
  contracts, types, and invariants where they carry the decision.
- **External interface and contract delta:** a compact table of added, changed,
  or removed commands, routes, events, or public operations and their
  input/output or compatibility effect. Mark contracts as reused, added, or
  changed.
- **Dependency and flow views:** one static dependency or wiring view followed
  by one to three representative runtime paths: main read, main mutation, and
  critical failure/recovery only when materially distinct. Use real symbols
  and show boundary crossings, transaction or authority ownership, durable
  effects, and recovery.
- **Error contract:** a compact table of important failures, their normalized
  error or outcome, owning handler, and retry, reconciliation, or terminal
  behavior.
- **Slice sequence and verifiers:** every slice in order as one compact row or
  line containing its behavior or retired risk, focused verifier, and
  observable proof. Include dependencies, rollback, or replan conditions only
  when they affect approval.
- **Change surface:** when it helps review scope, only the packages, modules,
  migrations, or runtime wiring created, materially changed, or removed.
- **Residual gates:** only risks being accepted for implementation, including
  evidence or owner when known, what could change, and the stage each blocks.
- **Delta:** a before/after item only after a prior review candidate changed.

Omit phase-completion recaps, exhaustive technology/API inventories, file
inventories, and low-level pseudocode. These are mandatory review views, not
inventories: they must remain concrete enough for a human to challenge public
behavior, reuse, boundaries, dependency direction, transaction ownership,
failure behavior, implementation order, and proof.

## Rendering Structural Views

Use Markdown tables for fixed-field comparisons and fenced `text` blocks for
dependency graphs and call paths. Do not mark structural diagrams as a
programming language. Give every block a short level-three heading, keep one
operation per line, use two- or three-space indentation consistently, and keep lines
short enough to scan without horizontal scrolling. Use `├─` for siblings and
`└─` for the final child. Put separate flows in separate blocks.

Use this shape for a dependency or wiring view:

### Dependency/wiring

```text
EntryPoint
└─ ApplicationOperation
   ├─ DomainPolicy
   ├─ DurableStore
   │  └─ DatabaseAdapter
   └─ OutboundPort
      └─ ProviderAdapter
```

Use this shape for a runtime path:

### Mutation — `operationName(input)`

```text
External event
└─ ProtocolBoundary.verify(...)
   └─ ApplicationOperation(input)
      └─ Transaction owner
         ├─ Success -> state + audit + intent
         └─ Failure -> reconcile or terminal outcome
```

Do not use a dense line such as
`A -> B -> C -> D -> E -> F`, paste executable implementations, or mix a
read, mutation, and recovery flow into one diagram.

Show the artifact path and SHA-256 after the summary. Ask whether the user
approves that exact version as the implementation baseline, including its
decisions, interface and contract delta, dependency and runtime flows, error
ownership, slice plan, residual gates, and replan triggers.

Clarification does not invalidate the design. Requested change supersedes the
affected decision and invalidates its dependent stages. Rejection reopens the
named stage. Regenerate and re-gate only affected work, then show a compact
before/after delta.

Approval applies to an exact SHA-256 version. Any later edit makes the brief
stale. Implementation begins in a fresh context only from an approved version.

## Lifecycle

Retain the approved brief and consequential decision records. Treat research,
state, prototype code, and tactical artifacts as regenerable, but never delete
them automatically.
