# Evaluation Contract

Evaluate routing separately from outcome quality and compare isolated runs with
and without Neo. Bootstrap confirmed routing with the deterministic CLI so the
candidate's first model invocation performs discovery, not router preflight.

## Deterministic Checks

Check state transitions, approvals, invalidation, artifact shape, card length,
prototype routing, required brief sections, verifiers, and stale-plan refusal
with code.

## Narrow Judges

Use one binary judge per semantic criterion:

1. The workflow investigates thoroughly, asks for consequential user input,
   and avoids unsupported conclusions.
2. Decision material is clear, compact, and focused on critical interfaces,
   flows, options, and tradeoffs.
3. The approved brief gives an implementation agent complete, coherent
   requirements and applies engineering practice relevant to the task.

Give each judge only the evidence needed for its criterion. Require a critique
and Pass/Fail verdict. Do not use generic helpfulness or coherence scores.

## Calibration

Treat judge output as exploratory until compared with human labels. Target at
least 20 human Pass and 20 human Fail examples before prompt design, then
validate on a disjoint held-out set. Measure true-positive and true-negative
rates; target both above 90%, with 80% the minimum usable threshold.

Record exact model, harness, prompt, versions, cost, latency, and failures.
Authentication errors, timeouts, and missing harnesses are unverified rather
than passes. Preserve real failures as regression cases.

Run independent case/harness pairs with bounded concurrency (`--jobs`, maximum
3). Store normalized final messages, tool call/failure counts, state, and
artifacts by default. Retain raw event streams only for an investigation that
explicitly passes `--raw-streams`; never duplicate raw streams at result and
step levels.

## Evidence Posture

This hierarchy follows evaluation research summarized from Shreya Shankar,
Hamel Husain, Sayash Kapoor, and Arvind Narayanan. Human calibration is a
minimum safeguard, not proof that a judge generalizes.
