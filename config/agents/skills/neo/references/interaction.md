# Decision and Feedback Interaction

## Decision Card

Keep the durable decision card at or below 250 words and use these headings in
the on-disk artifact:

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

## Present the Question

Do not paste the raw decision-card artifact or its seven headings into the
conversation. Present only the context needed to answer the current decision,
without a fenced code block.

When the harness provides a native question or choice control, use it for the
approval question. Put the recommended option first, mark it as recommended,
and keep each option label and description short. Send supporting context
before invoking the control, and do not repeat the question after it.

When no native control is available, use one short rendered heading, a numbered
list of options with the recommendation clearly marked, and one final plain-text
question. Never ask the user to respond to Markdown source or an artifact view.

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
