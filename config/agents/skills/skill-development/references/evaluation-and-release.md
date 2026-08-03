# Evaluation and Release

Each skill owns its evaluation package. Begin with at least three realistic
outcome cases and three positive plus three negative routing prompts. Assign a
development or held-out split before results are inspected.

Run three experiments:

1. Automatic routing: is the skill selected only for eligible tasks?
2. Forced conditional efficacy: does it help when explicitly activated?
3. Automatic end-to-end utility: does the deployable catalogue improve accepted
   outcomes after routing errors and context cost?

Compare no-skill, incumbent, and candidate within each harness using the same
model, effort, tools, permissions, timeout, and clean snapshot. Use fresh
sessions, three development repetitions, and five held-out release repetitions.
Retain every valid run and classify invalid harness, environment, telemetry,
and evaluator runs separately.

Promotion requires all functional, regression, integrity, and safety checks;
no critical permission, destructive-action, secret, test-tampering, or policy
bypass regression; held-out routing precision and recall of at least 90%; no
false activation on side-effect safety cases; and no unexplained control-only
pass on a blocking case. The candidate must add accepted outcomes/material
quality or improve one accepted-task efficiency measure by at least 10% while
no other efficiency measure worsens by more than 15% without an accepted
outcome gain and explicit owner approval.

Stages:

- Alpha: explicit invocation; structure, routing development, and three-repeat
  all-harness comparisons.
- Beta: constrained automatic routing after held-out gate; at least 20 eligible
  activations with metadata traces and owner feedback.
- RC: frozen routes/package; blocker fixes only; behavioral changes make a new
  RC and rerun held-out cases.
- Stable: at least two RC weeks and 20 eligible activations without safety,
  material quality, or unexplained rework regression.

Return stable to alpha after material redesign and to beta after a major
model/harness compatibility change.
