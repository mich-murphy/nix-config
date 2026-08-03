---
name: resolve-review
description: Triage and address GitHub pull-request review threads through verified fixes. Use when the user asks to inspect, address, respond to, or resolve PR review feedback. Support local-only remediation and explicit end-to-end mode; push, reply, or resolve remote threads only when the user authorizes PR updates.
---

# Resolve Pull Request Review Feedback

Account for every unresolved review thread without overwriting user work,
expanding the requested scope, or claiming more validation than was run.

## Route Model and Effort

Use a balanced model at medium effort for bounded inspection and local fixes
with executable checks. Use an efficient model at low effort only for
mechanical pagination, extraction, or ledger updates. Escalate to a frontier
model at high effort for end-to-end remote handling, conflicting threads,
nonlocal design, or security, data-loss, concurrency, and release-critical
risk. Keep authority, final classification, validation, and remote mutation
with the parent agent.

## Establish Scope

1. Read repository instructions, record the initial worktree state, and identify
   the target PR and mode:
   - **inspect**: assess threads and report;
   - **local**: edit and validate locally, then draft replies; or
   - **end-to-end**: commit, push, reply, and resolve satisfied threads.
2. Treat requests to “address” or “fix” feedback as local by default. Remote
   mutation requires wording such as “push,” “reply,” “resolve/close the
   threads,” or “handle end-to-end,” or an established grant of that authority.
3. Use the PR metadata recipe in
   [references/github-review-api.md](references/github-review-api.md). Compare
   local `HEAD` with the PR head OID; fetching does not update the checkout.
4. Never check out, merge, rebase, discard, or overwrite around user changes
   without authorization. Stop if the checkout cannot safely reach the PR head.

## Gather and Track Every Thread

Use the paginated thread query in
[references/github-review-api.md](references/github-review-api.md); never rely
on one `first:100` page or omit later replies.

Track ID, location, latest request, classification, action, verification,
reply, and resolution. Re-fetch before remote finalization.

## Triage Before Editing

Classify each unresolved thread against the current PR head:

- **Valid**: the concern is present; fix it.
- **Already addressed**: current code or a later commit satisfies it; cite the
  evidence instead of changing code again.
- **Needs clarification**: ambiguity, conflict, or consequential scope increase;
  ask before guessing.
- **Disagree or defer**: explain the technical evidence, trade-off, or scope
  boundary. Preserve the reviewer’s context and leave the thread unresolved.

An outdated diff position does not make the concern obsolete. Inspect the
current code and discussion before classifying it.

## Implement and Verify

1. Group related valid comments into the smallest coherent change.
2. Add or update a regression test when the concern describes testable
   behavior; when practical, confirm it fails without the fix.
3. Keep unrelated cleanup out of the patch and preserve pre-existing changes.
4. Discover validation from repository instructions, CI, and scripts. Run
   focused checks while iterating and the required full gate before publishing.
5. Inspect the final diff and tests. Green proves only what they exercised.

If required validation cannot run or remains red, report the exact limitation.
Do not push, claim completion, or resolve affected threads unless explicitly
accepted by the user.

## Publish Only in End-to-End Mode

1. Follow repository commit conventions. Push only intended, validated commits.
2. Re-fetch the PR head and unresolved threads. Stop if either changed in a way
   that invalidates the local assessment.
3. Reply on every handled thread with the outcome, relevant commit SHA, and
   verification. Use the reply recipe in the API reference.
4. Resolve only when the concern is satisfied, its reply is posted, validation
   is green, and the viewer may resolve. Leave clarification, disagreement, and
   deferral unresolved unless directed otherwise.
5. Re-fetch to verify resolution and report required checks.

## Completion Report

Summarize every thread category, commit and remote action, validation result,
remaining check, and unresolved risk, including newly discovered threads.
