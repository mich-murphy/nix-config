# Design Review and Pattern Selection

## Contents

- [Make Complexity Concrete](#make-complexity-concrete)
- [Prefer Deep, Coherent Boundaries](#prefer-deep-coherent-boundaries)
- [Use Changeability Lenses](#use-changeability-lenses)
- [Design It Twice](#design-it-twice)
- [Investigate Red Flags](#investigate-red-flags)
- [Select Patterns From Pressure](#select-patterns-from-pressure)
- [Reject Pattern Cargo Cults](#reject-pattern-cargo-cults)

## Make Complexity Concrete

Ousterhout frames complexity as the information a developer must hold to make a
change and how difficult that information is to discover. Review a representative
maintenance task:

```text
task -> modules touched -> facts/invariants required -> hidden dependencies
     -> duplicated decisions -> coordinated edits -> verification burden
```

Prefer a design that reduces required knowledge and makes the remaining facts obvious
at their point of use. Do not substitute method length, file count, class count,
coverage, or a named pattern for this walkthrough.

## Prefer Deep, Coherent Boundaries

A deep module exposes a small coherent interface while hiding substantial useful
policy, representation, or mechanism. For each consequential module, state:

- the design decision or knowledge it owns;
- what callers must know, including failure and cost semantics;
- what callers no longer need to know;
- the common operation the interface makes simple; and
- a likely internal change that leaves callers untouched.

Depth does not excuse a low-cohesion god module. A private field does not hide
information if every caller must understand the state machine. A wrapper is shallow
when it forwards calls without adding a distinct abstraction.

Pull complexity downward when one implementation can provide safe defaults, complete
common-case handling, and errors at the caller's abstraction level. Do not conceal
cost, security, partial failure, distributed state, or choices callers genuinely own.

## Use Changeability Lenses

- **DRY:** seek one authoritative representation of knowledge, not merely fewer
  repeated lines. Similar text with different change reasons need not be unified.
- **Orthogonality:** a change to one policy should disturb only its owner and explicit
  collaborators. Dependencies are acceptable when they change for the same reason.
- **Reversibility:** invest in a seam when reversal is expensive; use a direct solution
  when reversal is cheap. Do not pre-build hypothetical features.
- **Dependency direction:** keep domain policy independent from volatile transports,
  frameworks, storage details, clocks, randomness, and third-party APIs where the
  boundary makes a demonstrated change more local.
- **Explicit contracts:** locate preconditions, postconditions, invariants, failure
  semantics, state ownership, concurrency, and resource lifetime at the narrowest
  meaningful boundary.

## Design It Twice

For a consequential boundary, compare two genuinely different decompositions before
the first large edit. Evaluate:

- information hidden and leaked;
- common-path interface burden;
- dependency and coordinated-change surface;
- state, lifetime, concurrency, errors, and special cases;
- fit with repository concepts and conventions;
- test and operational observation points; and
- compatibility and migration cost from the current design.

Reject cosmetic alternatives around the same ownership and dependency graph. Choose
the smallest experiment, spike, or code walkthrough that could invalidate the favored
design.

## Investigate Red Flags

Use these Ousterhout-style red flags as questions, not automatic violations:

| Red flag | Investigation |
| --- | --- |
| Shallow module | What useful complexity disappears for its caller? |
| Information leakage | Which decision or invariant is known in more than one place? |
| Conjoined code | Must both pieces be read together to understand either? |
| Temporal decomposition | Do sequential stages split code that shares the same policy or state? |
| Pass-through method/layer | Does the layer add a distinct abstraction or compatibility boundary? |
| Special case | Can the normal contract absorb it without another branch or option? |
| Vague/inconsistent name | Does one term denote one precise concept throughout the domain? |
| Long interface explanation | Is the interface exposing implementation knowledge callers should not need? |

Also investigate shotgun surgery, divergent reasons to change, feature envy, global
state, hidden temporal coupling, primitive obsession, and tests coupled to private
structure. A finding is material only when connected to a real comprehension or
change scenario.

## Select Patterns From Pressure

Prefer repository-native idioms. Use this table to generate candidates, then justify
the candidate with the current pressure and verifier.

| Pressure | Candidate pattern or form | Avoid when |
| --- | --- | --- |
| Several stable policies implement one operation | Strategy or a function value | One small conditional is clearer or variation is speculative |
| Behavior genuinely varies by state with guarded transitions | State plus an explicit transition model | An enum and local switch keep the lifecycle clearer |
| Third-party or legacy interface should not leak | Adapter | It merely renames an already suitable interface |
| A subsystem needs one simpler cohesive entry point | Facade | Callers need distinct capabilities or costs the facade would hide |
| Construction policy or implementation choice must be hidden | Factory Method/function | Direct construction is stable and obvious |
| Many optional construction steps must preserve invariants | Builder | It permits invalid partial objects or replaces a simple constructor |
| Actions require queueing, logging, retry, or undo | Command | Plain function calls already express the lifecycle |
| One-to-many notification is intrinsic | Observer/events | It creates hidden control flow, weak delivery semantics, or debugging ambiguity |
| Behavior must compose independently around a core | Decorator/middleware | Ordering interactions become the main complexity |
| A domain concept has value equality and invariants | Value Object | Identity and lifecycle, not value, define the concept |
| Absence has a legitimate neutral behavior | Null Object | It conceals missing/invalid data or an operational failure |
| Related data/behavior share one invariant | Encapsulated aggregate/module | The result becomes a low-cohesion god object |
| Pure transformation can be separated from effects | Functional core, imperative shell/pipeline | The split exports ordering or state complexity to callers |
| A durable contract must change gradually | Parallel Change, Branch by Abstraction | No coexistence is required or the abstraction is speculative |

Favor composition over inheritance when behavior varies independently and callers do
not need subtype identity. Use inheritance only when substitutability is real and the
base contract remains coherent; do not use it solely for code reuse.

## Reject Pattern Cargo Cults

Do not introduce a pattern because the user named one, a linter suggested one, or the
catalog contains one. Ask:

1. What concrete change or maintenance task is costly now?
2. Which knowledge, invariant, or volatility needs one owner?
3. What does the pattern hide or make local?
4. What indirection, failure mode, or migration cost does it add?
5. Is a rename, extraction, inline operation, data transformation, or plain function
   simpler?
6. What test or walkthrough would show that the new design is better?

If the answers are weak, keep the direct design and document **no pattern warranted**.
