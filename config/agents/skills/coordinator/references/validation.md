# Manual coordinator validation

Exercise the delivery loop with existing subagent tools, Git, temporary fixtures,
and repository checks. These are controlled/rehearsed scenarios, not a reliability
benchmark or proof of unprompted behavior. Record what happened, never invented
passes. Use only tools available in the current host/session.

## Prepare safely

- Read the active brief and skill. Record `git status --short` and relevant diffs;
  preserve all pre-existing changes. Identify actual start/status/message/wait
  capabilities and disclose absent live visibility before choosing scenarios.
- Create a uniquely named scratch directory with `mktemp -d`; record its exact
  path. Keep fixtures outside tracked source, with disjoint worker-owned files.
  Do not modify repository gates/configuration or install dependencies.
- Choose disposable fixtures and existing tooling suited to the task: for example,
  Markdown with repository markdownlint, or JavaScript with Node's built-in
  `node --test` / `node --check`. Target explicit fixture paths. First prove the
  chosen check accepts clean input and detects deliberately invalid input; ignored
  files or zero tests/checks exercised are not a pass. Retain command/output evidence.
- Define fixture requirements (for example, required output or a scope sentence),
  owner, quality command, and parent acceptance criteria in the
  existing brief. Label injected faults and staged reports before interpreting them.

## Startup and healthy control

- Dispatch bounded assignments with disjoint ownership; request medium effort
  through supported controls. Require startup acknowledgement and meaningful
  progress evidence while working, without waiting for approval at healthy milestones.
- Confirm startup from acknowledgement/activity, not merely a dispatch receipt.
  Use available contact/status tools to resolve missing startup evidence.
- Give one healthy worker an independent clean-fixture task. Inspect its early
  approach and intermediate evidence, but send no correction when scope is sound.
  Let it finish uninterrupted; record observation versus actual intervention.

## Material drift: sent → acknowledged → uptake → rechecked

- In a separate assignment, explicitly rehearse an incorrect scope sentence or
  proposed out-of-scope addition. Have the worker expose this intermediate artifact
  and continue useful in-scope work long enough to permit live inspection/steering.
  Record this as a controlled checkpoint, not naturally occurring drift.
- Parent inspects the artifact while work is active and sends a concrete correction:
  observed mismatch, required scope, and expected evidence. Do not edit owned files.
- Record separately: message sent/delivered; worker's explicit acknowledgement;
  subsequent artifact/activity showing uptake; parent diff review and rerun of the
  relevant check. Acknowledgement alone cannot close the correction.
- If steering races completion or visibility is missing, record that limitation.
  Resume/contact the owner through available tools for correction; do not relabel
  a post-completion correction as observed in-flight steering.

## Failed check repair

- Assign a labeled invalid fixture (for example, a failing Node assertion or a
  Markdown heading violation). Require the worker to run the chosen unchanged
  check and report the real failure before repair, then rerun the same command.
- Parent verifies failure/repair output, corrected artifact, and unchanged gate
  configuration. A suppression, ignored fixture, skipped check, or lowered threshold
  is not repair. Resolve ordinary failures internally; ask the operator only for a
  genuine authority/information blocker, not an arbitrary retry count.

## Conflicting or incomplete findings

- Give read-only reviewers a fixture and require file/command evidence. Rehearse
  a labeled stale or incomplete report alongside a contrary current finding; keep
  staged claims distinguishable from independently observed reviewer behavior.
- Parent reads the current file/diff and reruns the disputed check, establishing
  which report applies to which artifact state. Request missing evidence or a
  corrected report. Do not decide by confidence, majority, or a claimed green gate.

## Fresh-context continuation

- In the same disposable fixture, record decisions/exclusions, accepted work, an
  open correction, worker IDs/owned paths and status evidence in the existing brief.
  Include pre-existing dirty work and a real passing check tied to its source state;
  then change relevant source so that pass is stale. Stop/release a fixture owner
  through actual tools and record evidence; leave an unresolved ID labeled unknown.
- Give a fresh worker/session only that durable brief and the live checkout (with
  the skill/reference paths), not prior transcripts or operator re-explanation.
  Require rediscovery of actual capabilities, checkout/PR reconciliation as relevant,
  and live ownership checks before dispatch/takeover. Unknown ownership must remain
  blocked, not be inferred safe from silence; released paths may be explicitly reassigned.
- Inspect whether it preserves dirty work, exclusions and accepted work, avoids
  duplicate dispatch, continues the open correction and rejects the stale pass.
  Require rerun evidence tied to the resulting source and parent review before
  acceptance. Record actual observations and tool/access limits separately from
  scripted setup; if fresh context or live ownership checks are unavailable, mark
  those behaviors unexercised rather than passed.

## Close with evidence and cleanup

- In the brief, separate worker quality commands/outcomes from the parent's
  criterion-by-criterion requirements review and acceptance. Record correction
  acknowledgement, observed uptake, recheck, unresolved gaps, and stale evidence.
- Parent accepts only after inspecting final artifacts and current relevant checks.
  Mark unavailable, failed, or unexercised scenarios honestly; no blanket pass.
- After all workers stop writing, retain necessary evidence in the brief, inspect
  the exact recorded scratch path, and remove only fixtures created for this run.
  Compare final Git status/diffs to baseline; never use broad clean/reset commands.
