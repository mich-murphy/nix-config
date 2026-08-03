# System and Software Architecture

Architecture addresses stakeholder concerns through boundaries and tradeoffs;
it is not a stack list.

## Describe

- System of interest, environment, stakeholders, and material concerns.
- Component responsibilities and external dependencies.
- Trust boundaries, public/internal contracts, and data ownership/lifecycle.
- Synchronous and asynchronous control flow.
- Failure, recovery, observability, support, and deployment behavior.
- Compatibility, migration, independent deployability, and reversibility.

Use the smallest view that answers a named audience's question. Add dynamic or
deployment views only when runtime interaction or topology carries risk.

## Quality Scenarios

Replace adjectives with:

```text
stimulus + operating condition + affected component
  -> expected response + measurable threshold
```

Prioritize three to five material scenarios. Compare at least two viable
approaches against them. Record benefits, costs, risks, sensitivity points,
migration, and evidence still needed.

## Gate

Approve when stakeholders can trace their important concerns to a decision,
scenario, or explicit unresolved risk. Reject proposals that omit data,
failure behavior, alternatives, or costs.

## Evidence Posture

Stakeholder-concern architecture is high-certainty standards guidance.
Scenario-based tradeoff review is authoritative methodology recommended for
local trial, not proof that a particular architecture will succeed.
