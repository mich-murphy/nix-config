# Program Design

Shape the code below system architecture and above line-level implementation.

## Sketch Consequential Details

- Modules and the knowledge or representation each hides.
- Public types, functions, methods, events, or commands.
- Preconditions, postconditions, errors, and compatibility.
- Domain entities, state transitions, ownership, mutation, and invariants.
- Data operations, scale, lifecycle, and the simplest adequate representation.
- Main success and important failure call paths.
- Dependency direction, substitution/test seams, concurrency, cancellation,
  idempotency, and resource lifetime where relevant.
- Repository naming/layout conventions and the safe path from current design.

For an expensive boundary, produce two materially different decompositions.
Compare information hidden and leaked, interface surface, change propagation,
error/state complexity, testability, migration, and repository fit.

Prefer a deep boundary that hides meaningful complexity over many pass-through
modules. Avoid speculative generality.

## Gate

Approve when a reviewer can trace representative behavior through contracts,
locate each invariant, and explain why likely changes remain local. Use a
disposable logical prototype when a critical call path or model remains
uncertain.

## Evidence Posture

These practices synthesize Ousterhout, Parnas, Liskov, Fowler, and
operation-first data-structure guidance. They are strong design arguments and
expert methods, not universal causal laws.
