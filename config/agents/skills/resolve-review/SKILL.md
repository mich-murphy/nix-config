---
name: resolve-review
description: Resolve PR review comments end-to-end. Use when a human asks to address, resolve, or respond to review feedback on a pull request. Validates each comment, implements fixes with tests, runs the full validation gate, replies inline, and resolves handled threads before reporting a summary.
---

# Resolve PR Review

Work through every unresolved review thread on the target PR until each one is
fixed, answered, or explicitly deferred with the reviewer's context preserved.

## Gather

1. Identify the PR (from the argument, current branch, or `gh pr view`).
2. List all unresolved review threads:

   ```sh
   gh api graphql \
     -f query='query($owner:String!,$repo:String!,$pr:Int!){
       repository(owner:$owner,name:$repo){pullRequest(number:$pr){
         reviewThreads(first:100){nodes{id isResolved path line
           comments(first:20){nodes{id databaseId author{login} body}}}}}}}' \
     -f owner=OWNER -f repo=REPO -F pr=NUMBER
   ```

3. Sync the branch with upstream before assessing anything
   (`git fetch origin && git status`); never judge a comment against a stale
   ref.

## Validate Each Comment

For each unresolved thread, read the referenced code at the current head and
decide:

- **Valid** — the concern is real. Implement the fix.
- **Already addressed** — a later commit resolved it. Reply with the commit
  reference; do not re-change code.
- **Disagree or out of scope** — do not silently ignore it. Reply explaining
  why, and leave the thread unresolved for the reviewer to close.

## Fix

- Implement each valid fix with an accompanying or updated test where the
  change has testable behavior.
- Keep changes scoped to what the comment requires; flag anything larger
  rather than expanding the diff.
- After any merge-conflict resolution, re-run formatters/linters before
  pushing.

## Validation Gate

Run the full gate for the affected area before pushing:

- Web (`web/`): `bun run check-types`, `bun run lint`, `bun run test:unit`,
  and `bun run build`, plus `bun run changelog:check` if the change is
  customer-visible.
- Classic/DBL: `abc --json dev --changed --elb` (60-second cap; on timeout
  inspect `target/abc/abc-last-run.json`).

Do not push or reply until the gate is green.

## Reply and Resolve

1. Push the fixes (Conventional Commit messages, one commit per logical fix or
   a single `fix(scope): address review feedback` when the fixes are small).
2. Reply inline on each thread with what was done and the commit SHA:

   ```sh
   gh api repos/OWNER/REPO/pulls/NUMBER/comments/COMMENT_ID/replies \
     -f body='Fixed in <sha>: <one-line description>'
   ```

3. Resolve only threads whose fix is pushed and green:

   ```sh
   gh api graphql \
     -f query='mutation($id:ID!){resolveReviewThread(input:{threadId:$id}){
       thread{isResolved}}}' \
     -f id=THREAD_ID
   ```

   Leave disagreement/deferral threads unresolved.

## Report

Finish with a summary: threads fixed (with commits), threads answered without
code change, threads deferred and why, and the validation gate results.
