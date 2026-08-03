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

Finalize creates a versioned review candidate. Present:

- critical product, architecture, and program decisions;
- main interface and high-level call path;
- slice sequence and verifier;
- unresolved risks; and
- changes since the prior review.

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
