# Evaluation and Release

Each skill owns its evaluation package. Begin with at least three realistic
outcome cases and three positive plus three negative routing prompts. Assign a
development or held-out split before inspecting results.

Run three experiments:

1. Automatic routing: is the skill selected only for eligible tasks?
2. Forced conditional efficacy: does it help when explicitly activated?
3. Automatic end-to-end utility: does the deployable catalogue improve accepted
   outcomes after routing errors and context cost?

Compare no-skill, incumbent, and candidate within each harness using the same
model, effort, tools, permissions, timeout, environment, and verifier. Use fresh
sessions and clean snapshots. Run three development repetitions and five
held-out release repetitions. Retain every valid run and classify invalid
harness, environment, telemetry, and evaluator runs separately.

Use binary accepted outcome as the primary measure and keep criterion-level
quality detail. Report paired transitions (`both_pass`, `candidate_only`,
`control_only`, `both_fail`) plus tokens, cost, latency, and human rework per
accepted task. Never select the best stochastic run or treat an invalid run as
a pass.

Promotion requires all functional, regression, integrity, and safety checks;
no critical permission, destructive-action, secret, test-tampering, or policy
bypass regression; held-out routing precision and recall of at least 90%; no
false activation on side-effect safety cases; and no unexplained control-only
pass on a blocking case. The candidate must add accepted outcomes or material
quality, or improve one accepted-task efficiency measure by at least 10% while
no other efficiency measure worsens by more than 15% without explicit owner
approval.

Stages:

- Alpha: explicit invocation, deterministic checks, and development evidence.
- Beta: constrained automatic routing after held-out gates and at least 20
  eligible activations with outcome-linked metadata traces.
- RC: frozen package and routes; behavioral changes create another RC and full
  held-out replay.
- Stable: at least two RC weeks and 20 eligible activations without safety,
  material-quality, or unexplained-rework regression.

Return stable to alpha after material redesign and to beta after a major model
or harness compatibility change. Limit every benefit claim to the tested task,
model, effort, harness, tool, and version matrix.
