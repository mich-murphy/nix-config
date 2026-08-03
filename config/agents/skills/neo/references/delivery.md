# Delivery Design

Turn approved decisions into an order of learning and integration.

## Vertical Slice Contract

For every slice state:

1. Behavior or risk proved.
2. Minimum changes across real affected boundaries.
3. Prerequisites and explicit non-goals.
4. Focused automated verifier.
5. Real-interface, visual, or operational evidence.
6. Compatibility, migration, rollout, and rollback.
7. Architecture and program decisions exercised.
8. Stop or replan condition.

Start with the slice that retires the most consequential uncertainty while
leaving the system coherent and reviewable. A schema-only, service-only, or
UI-only batch is horizontal unless it independently proves the intended
behavior or a named migration risk.

Use a production-quality tracer slice when the path is understood. Route an
unanswered design question to research or a disposable prototype instead.

## Gate

Approve when every slice has observable completion evidence, fits human review
capacity, and preserves a usable integration path. “All tests pass” is not the
only evidence, and working behavior must appear before the last slice.

## Evidence Posture

Vertical delivery is experience/practitioner consensus with low causal
certainty. Small-batch delivery research is observational and context
dependent. Measure rework, feedback interval, reviewability, and operational
outcomes locally.
