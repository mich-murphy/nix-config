# Reviewed workflow evidence

Source: one severity-one incident retrospective and four subsequent review
threads, redacted on 2026-07-30.

Reviewers repeatedly missed the first causal error across service logs. The
accepted workflow reads local redacted logs, aligns timestamps, identifies the
first upstream failure, cites exact local line references, separates facts from
hypotheses, and writes a proposed triage report. It must not contact external
services, transmit log content, mutate or delete evidence, execute remediation,
or declare root cause without reviewer confirmation. A human incident commander
owns acceptance.
