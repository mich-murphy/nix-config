# Decision and Feedback Interaction

## Decision Card

Keep a card at or below 250 words and use these headings:

```markdown
## Decision
## Why now
## Evidence
## Affected interface or flow
## Options
## Recommendation
## Approval question
```

Offer two or three genuinely viable options. State costs and risks, not only
benefits. Recommend an option when evidence permits, including its evidence
strength. Ask one consequential question at a time; group only independent
low-risk confirmations.

## Feedback

Classify a response before changing state:

- **Approve:** preserve the approved artifact version.
- **Clarify:** answer without changing approval state. Convert to change if the
  answer reveals a changed requirement or decision.
- **Change:** name affected decision IDs. If mapping is ambiguous, confirm the
  interpretation before recording it.
- **Reject:** name the earliest stage whose direction is rejected.

Use `record-feedback` for every response that affects the plan. Change and
rejection deterministically invalidate downstream stages. Regenerate only
invalidated artifacts, then present a before/after delta.

Never infer approval from silence, enthusiasm, or a request for explanation.

Validate Neo artifacts only with the Neo CLI. Do not run general Markdown
linters, download validation tools, or add network-dependent checks to the
interactive path.

## Evidence Posture

One-question-at-a-time grilling and compact upstream design review are
experience-backed practices from Matt Pocock and Dex Horthy. Their value is
plausible but not established by controlled comparisons; preserve failures as
evaluation cases.
