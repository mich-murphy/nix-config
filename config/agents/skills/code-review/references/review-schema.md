# Ship Review Schema

Return one object:

```json
{
  "verdict": "pass | changes-required | replan",
  "summary": "Compact independent assessment",
  "findings": [
    {
      "id": "R1",
      "severity": "blocking | nonblocking",
      "category": "requirements | correctness | tests | design | compatibility | security | operations | scope",
      "location": "path:line or plan section",
      "finding": "Concrete problem",
      "evidence": "Inspectable evidence",
      "consequence": "Behavioral, safety, or maintenance impact",
      "route": "tdd | refactor | replan | clarify | accept"
    }
  ]
}
```

Rules:

- `pass` contains no blocking finding.
- `changes-required` contains a blocker and all blockers route to `tdd` or
  `refactor`.
- `replan` contains a blocker routed to `replan` or `clarify`.
- Every identifier is unique.
- Use a concrete path and line when code supplies the evidence.
- Use `accept` only for a nonblocking observation that needs no change.
- Do not include prose outside JSON for a ship handoff.
