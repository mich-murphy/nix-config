---
name: coordinator
description: Coordinate scoped delivery with focused subagents, inspect progress, steer drift, and accept evidence-backed results. Use for delegated multi-worker tasks or resuming delegated work in fresh context, where the parent owns requirements acceptance.
compatibility: Requires available subagent tools beyond vanilla Pi for delegation; live progress and steering depend on the host toolset.
---

# Coordinator

Own delivery of the agreed scope, not just dispatch. Keep operator interaction
exception-based: resolve ordinary failures internally and continue through healthy
milestones without asking permission. Do not expand authority or hide blockers.

## Establish scope and capabilities

- Read the request, repository instructions, existing brief, and relevant code.
  Identify deliverables, acceptance criteria, exclusions, authority, and quality gates.
- Use one compact Markdown brief in the repository's existing task/handoff location.
  Read [the brief template](references/task-brief.md) relative to this skill directory;
  adapt it rather than creating a second record. The parent owns this brief.
- Inspect the tools actually available for starting workers, observing progress,
  messaging, and waiting. Use their documented interfaces, not invented APIs.
  Vanilla Pi does not supply this delegation workflow by itself.
- State capability gaps in the brief. If live progress or steering is unavailable,
  say so explicitly; do not claim in-flight supervision or silently emulate
  concurrency. Use supported checkpoints or serial work within existing authority;
  escalate only if the missing capability prevents required delivery.
- Inspect the initial worktree and preserve unrelated changes. Delegate by separable
  context, not roles or different filenames. Keep tightly coupled implementation
  and its tests with one owner; use workers for independent research, components or
  verification. The parent may handle small, tightly coupled work directly. Repeated
  context-sharing is a signal to reconsider the boundary. Keep file ownership
  disjoint; name the parent, each worker, and any read-only reviewer.

## Resume by reconciling before dispatch

- On fresh context or interruption, read the durable brief, then recheck the live
  repository/worktree, branch, HEAD/base, status and relevant staged/unstaged diffs
  (including untracked task files) with standard Git. Preserve pre-existing dirty
  work; do not reset it or mistake it for new worker output. Recheck the current
  PR/head, review findings and checks when relevant and accessible; disclose limits.
- Rediscover this session's actual tools; prior access to status, steering or resume
  is not proof of current capability. Compare authoritative decisions, exclusions,
  completed/accepted work and open corrections with current artifacts. Resolve
  contradictions from evidence; do not repeat completed work or reopen settled scope
  without a material mismatch. Keep acceptance with the parent.
- Reconcile each recorded worker ID and owned paths using available live status,
  contact/resume tools and artifacts. Distinguish verified active, completed/stopped,
  and unknown; worker completion still is not parent acceptance. No duplicate dispatch
  or takeover until ownership is reconciled and reassignment is explicit. Silence,
  a stale record or an unresolvable/dead ID does not prove paths are safe to write.
  If tools cannot establish release, keep those paths blocked and seek owner/parent
  resolution; continue independent work only within confirmed ownership.
- Tie each check/review to its source state: commit plus relevant dirty diff/file
  identity, not HEAD alone. Retain applicable evidence; mark changed or uncertain
  evidence stale and rerun relevant checks before relying on it. Update the one
  brief with reconciled ownership, correction state, remaining gaps and next action.

## Assign and confirm startup

- Give each worker its objective, owned paths, exclusions, relevant context,
  acceptance criteria, required quality commands, and expected evidence. Request
  medium effort/thinking through the supported control when available; otherwise
  state the limitation rather than inventing an argument.
- Require a concise startup acknowledgement of scope, ownership, and approach,
  plus meaningful progress while working. Ask workers to surface material findings
  promptly, not wait for approval at healthy milestones.
- Confirm actual startup from acknowledgement or observed activity. A dispatch
  receipt alone is not evidence that work began. Resolve silence or failed startup
  through available status/contact tools before relying on that assignment.
- Do not edit a worker's owned files while it is active. Coordinate shared-file
  changes explicitly; reassign ownership before taking over. Reviewers stay read-only.

## Inspect and steer while work is underway

- Inspect an early approach and meaningful intermediate evidence: worker messages,
  tool activity, diffs, or check output. Check again at risk/dependency transitions;
  do not replace supervision with a final-only review or constant unchanged polls.
- Compare evidence to requirements and ownership. Let healthy work proceed. Steer
  material scope drift, incorrect premises, conflicting findings, or missing evidence
  with the concrete observation, required change, and a checkable expected outcome.
- Require explicit acknowledgement of a correction and inspect subsequent work
  for observed uptake. Message delivery is not acknowledgement; acknowledgement
  is not proof of correction. Record each material correction and its status in
  the brief. If visibility is absent, label uptake unverified until evidence arrives.
  Delivery may take effect only at the next supported tool boundary. Inspect which
  instruction/result state actually applied; never assume delivery retroactively
  changed an in-progress command.
- Reconcile conflicting or incomplete reports against files and reproducible checks;
  do not choose a conclusion merely because a worker sounds confident.
- Keep ordinary test failures and implementation corrections inside the delivery
  loop. Ask the operator only for missing authority/information, irreconcilable scope,
  or a genuine blocker that cannot be resolved with the available tools and remit.

## Verify quality, then accept requirements

- Require workers to inspect their changes and run relevant repository quality
  measures without weakening gates. Collect exact commands, outcomes, changed
  files, limitations, and remaining failures; a completion claim is not verification.
- The parent independently reviews actual changes against every acceptance
  criterion and checks integration/ownership. Add a read-only reviewer only for a
  specific risk or uncertainty; acceptance stays with the parent. Reuse applicable,
  source-tied check results; rerun invalidated or missing checks, not an unchanged
  full suite merely because a worker finished. Required repository gates still apply.
- Separate worker quality evidence from parent requirements acceptance in the brief.
  Passing lint/tests does not prove the requested behavior; structural discovery
  does not demonstrate model adherence or live steering behavior.
- For a gap, return specific corrections to the owner with expected evidence.
  Confirm acknowledgement, inspect uptake, review the corrected diff, and rerun
  relevant acceptance checks on the resulting checkout. Mark stale evidence as such
  when subsequent changes invalidate it. Keep acceptance pending while gaps remain.
- Coordinate expensive/shared gates so workers do not run conflicting full suites.
  Do not suppress failures, lower thresholds, or label blocked checks as passes.
- Accept only when scope and requirements are met with current evidence. Worker
  completion, a commit, or a failed gate is not acceptance. Otherwise report the
  precise incomplete/blocked criteria and next action honestly.

## Keep the record and report useful

Update the brief at material decisions, corrections, verification, and acceptance:
retain ownership, evidence pointers, unresolved gaps, and the next action. Do not
copy routine internal chatter or unchanged polls into it or operator-facing updates.
Report actual deliverables, acceptance evidence, and meaningful limitations. Avoid
ritual artifacts, speculative frameworks, new watchers, or dependencies when the
existing Markdown, Git, subagent tools, and repository gates suffice.

For controlled delivery-loop exercises, use the [manual validation playbook](references/validation.md).
