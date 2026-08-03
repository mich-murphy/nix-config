---
name: code-review
description: Independently review a candidate software change against its original plan, repository constraints, behavior, tests, design quality, compatibility, security, operations, and verification evidence without modifying files. Use for review-only requests, fresh-context $ship review handoffs, or evidence-led diff assessment. Do not use to implement fixes, refactor code, resolve existing GitHub review threads, perform product or architecture design, or approve a change whose governing plan is missing.
---

# Review a Candidate Independently

Remain read-only. Reconstruct intent from primary artifacts, identify material
problems, and return a verdict that another owner can act on.

## Establish the Review Boundary

1. Read repository instructions.
2. Identify the original plan, acceptance criteria, non-goals, base revision,
   complete candidate diff, changed tests, and verification evidence.
3. Record missing or stale inputs before judging the implementation.
4. Inspect the relevant current code and callers needed to understand the diff.
5. Do not trust an implementation summary when the plan, code, test, or command
   evidence says otherwise.

For a `$ship` handoff, work in a genuinely fresh context and do not read the
implementation transcript. If the plan is missing or a consequential decision
is unresolved, return a blocking `replan` or `clarify` finding instead of
inventing an expected design.

## Reconstruct the Change

State compactly:

- intended and preserved behavior;
- affected users, callers, data, and operational paths;
- changed responsibilities, dependencies, interfaces, and representations;
- evidence claimed by the implementation; and
- dimensions that remain unverified.

Use this model to review the diff, not as a replacement for concrete findings.

## Review by Risk

Read [review-criteria.md](references/review-criteria.md). Inspect:

1. **Requirements and correctness:** every criterion, failure path, boundary,
   state transition, and unintended behavior.
2. **Tests and evidence:** failure sensitivity, meaningful red evidence,
   independent expectations, determinism, observation boundary, integrity, and
   missing broader checks.
3. **Design and maintainability:** comprehension, information hiding,
   responsibility, change locality, dependency direction, duplication,
   unnecessary indirection, and speculative abstraction.
4. **Compatibility and data:** public consumers, errors, formats,
   serialization, schemas, mixed versions, migration, and rollback.
5. **Security and operations:** trust boundaries, privileges, secrets,
   failure modes, resource lifetime, telemetry, rollout, and recovery.
6. **Scope and repository health:** unrelated edits, generated files,
   documentation, conventions, and preservation of user work.

Passing tests are evidence only for what they observe. Do not infer production
safety, architecture quality, or compatibility from green checks alone.

## Write Material Findings

Lead with findings ordered by severity. A finding must include:

- exact location;
- concrete observation and evidence;
- realistic behavioral or maintenance consequence;
- smallest credible direction, not a full implementation;
- blocking or nonblocking severity; and
- route: `tdd`, `refactor`, `replan`, `clarify`, or `accept`.

Use `tdd` for incorrect or missing behavior and `refactor` only when
observable behavior should remain fixed. Use `replan` when the plan is
missing, contradicted, or requires a consequential decision. Label optional
polish as nonblocking; do not turn taste into correctness.

A valid outcome may contain no findings. Do not invent issues to demonstrate
review effort.

## Return the Verdict

Read [review-schema.md](references/review-schema.md). For a `$ship` handoff,
return only JSON matching [review.json](assets/review.json):

- `pass`: no blocking findings;
- `changes-required`: every blocker routes to `tdd` or `refactor`;
- `replan`: at least one blocker routes to `replan` or `clarify`.

For an ordinary chat review, lead with the same prioritized findings and
conclude with the verdict, inspected evidence, and remaining uncertainty.

Never edit files, apply fixes, create commits, push, reply to review threads,
or resolve them. Use `$resolve-review` for existing PR feedback.

## Reference Route

- [review-criteria.md](references/review-criteria.md): detailed correctness,
  test, design, compatibility, security, and operations lenses.
- [review-schema.md](references/review-schema.md): finding and verdict contract
  shared with `$ship`.
- [sources.md](references/sources.md): evidence posture and provenance.
