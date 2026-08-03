# Risk Routing

Use confirmed facts to select planning depth. Planning effort follows the cost
of being wrong, not estimated lines of code.

## Signals

Use only these exact signals with `neo.py assess`:

- Product: `problem-uncertain`, `outcome-uncertain`, `user-uncertain`,
  `interaction-uncertain`.
- Architecture: `system-boundary`, `trust-boundary`, `public-contract`,
  `persistent-data`, `security`, `reliability`, `deployment`, `compatibility`,
  `expensive-reversal`.
- Program: `new-abstraction`, `domain-invariant`, `state-machine`,
  `concurrency`, `consequential-interface`, `data-structure`,
  `multi-module-call-path`.

No confirmed signal produces the direct route and returns the task to ordinary
Codex planning without Neo artifacts. Any confirmed signal requires discovery,
delivery, and finalization. Some design levels produce a focused route; all
three produce the full route.

## Confirmation Rule

The agent proposes labels and cites the observation behind each one. The user
confirms any label based on intent, priority, or consequence. Do not turn
absence of information into a negative signal.

## Evidence Posture

Risk-proportional planning is practitioner consensus with limited causal
evidence. The four-level distinction is a moderate-certainty synthesis grounded
in human-centred design standards, architecture standards, software-design
literature, and Dex Horthy's practitioner reports. Trial and calibrate locally.
