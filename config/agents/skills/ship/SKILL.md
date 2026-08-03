---
name: ship
description: Manage an implementation-ready software plan through repository orientation, disciplined TDD, verification, fresh-context independent review, bounded remediation, optional existing-PR feedback handling, and a release-ready decision. Use only when requirements and consequential design decisions are already settled and the user explicitly invokes $ship or supplies a validated ship handoff. Stop for missing plan information or replanning; do not perform research, product or architecture design, deployment, release, publishing, or merge.
---

# Ship an Approved Change

Take one approved plan to a release-ready candidate. Own the lifecycle and
acceptance decision; delegate specialist mechanics without delegating scope or
final judgment.

Resolve `<ship-cli>` to `.agents/skills/ship/scripts/ship.py` when present;
otherwise use `~/.agents/skills/ship/scripts/ship.py`.

## Admit the Plan

1. Read repository instructions, the supplied plan, relevant source and tests,
   and the initial worktree state.
2. Read [readiness-and-stops.md](references/readiness-and-stops.md).
3. Confirm that behavior, acceptance criteria, non-goals, consequential design
   decisions, compatibility constraints, and required verification are
   actionable. Inspect the codebase to apply the plan; do not research or
   invent missing intent.
4. If anything consequential is missing or conflicts with repository evidence,
   make no edit. Return the exact evidence, missing decision, impact, and
   requested plan clarification.
5. Copy [readiness.json](assets/readiness.json), fill it with task facts, and
   start the state machine:

   ```sh
   python3 <ship-cli> --root <repo> start <slug> \
     --title "<title>" --plan <plan-path>
   python3 <ship-cli> --root <repo> ready <slug> \
     --readiness <readiness-json>
   ```

Do not continue unless `ready` succeeds.

## Establish the Baseline

Run the fastest trustworthy repository checks for the affected behavior before
editing. Record pre-existing failures and protect unrelated user changes. Stop
when the plan depends on a broken or unavailable verifier and no approved
alternative exists.

## Implement Through TDD

Invoke `$tdd` for each predictable behavior slice. Keep the living behavior
list aligned with the plan and preserve its red, green, and refactoring
evidence. Use `$refactor` for a multi-step structural change; return to green
before continuing.

Stop and request replanning when implementation evidence invalidates a
requirement, approved boundary, data contract, security property, or other
consequential decision. Do not design around it.

After the agreed behavior is green, write a compact implementation-evidence
artifact and run:

```sh
python3 <ship-cli> --root <repo> advance <slug> \
  --stage implementation --evidence <implementation-evidence>
```

## Verify the Candidate

Run focused tests, relevant prior tests, and repository-required build, type,
lint, integration, security, documentation, and artifact checks. Inspect tests
for weakening, implementation coupling, nondeterminism, and missing failure
sensitivity. Inspect the final diff for scope and user-work preservation.

Record commands, results, limitations, and unexecuted checks in a verification
artifact, then run:

```sh
python3 <ship-cli> --root <repo> advance <slug> \
  --stage verification --evidence <verification-evidence>
```

## Require Fresh Independent Review

Read [review-loop.md](references/review-loop.md). Start a fresh read-only agent
or session and invoke `$code-review` with the original plan, repository
instructions, base revision, candidate diff, changed tests, and verification
evidence. Do not pass the implementation transcript or ask the implementer to
self-certify.

If a genuinely fresh review context is unavailable, stop with
`review_required`; do not mark the candidate release-ready.

Save the reviewer's JSON using [review.json](assets/review.json), then run:

```sh
python3 <ship-cli> --root <repo> record-review <slug> \
  --review <review-json>
```

Route the result:

- `tdd`: return to `$tdd` in behavior mode.
- `refactor`: invoke `$refactor` with observable behavior fixed.
- `replan` or `clarify`: stop and return an upstream handoff.
- pass: continue only if the candidate hash still matches verified evidence.

After remediation, record the remediation evidence:

```sh
python3 <ship-cli> --root <repo> advance <slug> \
  --stage remediation --evidence <remediation-evidence>
```

Then repeat verification and a newly fresh review. Never reuse a review after
the diff changes.

## Handle Existing PR Feedback Conditionally

When a target pull request already exists and unresolved threads are in scope,
invoke `$resolve-review` in the authority mode granted by the user. Any code
change invalidates the prior verification and review; run:

```sh
python3 <ship-cli> --root <repo> invalidate <slug> \
  --reason "PR feedback changed the candidate"
```

Then repeat TDD or refactoring as appropriate, verification, and fresh review.
Do not create, push, reply, resolve, or merge unless separately authorized.

## Declare Release Readiness

Read [release-readiness.md](references/release-readiness.md), then run:

```sh
python3 <ship-cli> --root <repo> validate <slug>
```

Finish only when validation reports `release-ready`. Report the plan and
candidate identities, changed behavior and design, TDD and verification
evidence, independent-review disposition, compatibility and operational
impact, remaining risk, and unexecuted checks.

Never deploy, release, publish, merge, or describe release-ready as deployed.

## Reference Route

- [readiness-and-stops.md](references/readiness-and-stops.md): plan-admission
  contract and upstream stop conditions.
- [review-loop.md](references/review-loop.md): fresh-review packet, finding
  routing, and bounded remediation.
- [release-readiness.md](references/release-readiness.md): final evidence and
  claim boundary.
- [sources.md](references/sources.md): evidence posture and provenance.
